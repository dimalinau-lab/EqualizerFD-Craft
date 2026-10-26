use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;
use windows::core::HRESULT;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsZoomed,
    LoadImageW, SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow,
    IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
    WM_SETICON,
};

#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, f_attach: BOOL) -> BOOL;
}

const CREATE_NO_WINDOW: u32 = 0x08000000;
static STORED_APP_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn set_stored_app_hwnd(hwnd: HWND) {
    STORED_APP_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
}

pub fn get_app_hwnd() -> Option<HWND> {
    let val = STORED_APP_HWND.load(Ordering::SeqCst);
    if val != 0 {
        let hwnd = HWND(val as *mut _);
        unsafe {
            if IsWindow(hwnd).as_bool() {
                return Some(hwnd);
            }
        }
    }
    None
}

pub fn find_app_window() -> Option<HWND> {
    if let Some(hwnd) = get_app_hwnd() {
        return Some(hwnd);
    }
    unsafe {
        let mut target_hwnd: HWND = HWND(std::ptr::null_mut());

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_buf);
            let class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_buf[..class_len as usize])
            } else {
                String::new()
            };

            // Avoid helper or companion windows
            if class_name.starts_with("WindowsForms") || class_name.contains("Tray") {
                return BOOL(1);
            }

            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                if title.contains("Equalizer FD CRAFT") {
                    let out_ptr = lparam.0 as *mut HWND;
                    *out_ptr = hwnd;
                    return BOOL(0); // Stop enumerating
                }
            }
            BOOL(1)
        }

        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(&mut target_hwnd as *mut _ as isize),
        );

        if !target_hwnd.0.is_null() {
            set_stored_app_hwnd(target_hwnd);
            Some(target_hwnd)
        } else {
            None
        }
    }
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dw_attribute: u32,
        pv_attribute: *const std::ffi::c_void,
        cb_attribute: u32,
    ) -> HRESULT;
}

pub fn make_window_frameless(hwnd: HWND) {
    unsafe {
        // 1. Force light gray titlebar color (#F0F0F0) and dark text (#222222) via DWM
        let caption_color: u32 = 0x00F0F0F0;
        let _ = DwmSetWindowAttribute(
            hwnd,
            35, // DWMWA_CAPTION_COLOR
            &caption_color as *const u32 as *const std::ffi::c_void,
            4,
        );

        let text_color: u32 = 0x00222222;
        let _ = DwmSetWindowAttribute(
            hwnd,
            36, // DWMWA_TEXT_COLOR
            &text_color as *const u32 as *const std::ffi::c_void,
            4,
        );

        let dark_mode: u32 = 0;
        let _ = DwmSetWindowAttribute(
            hwnd,
            20, // DWMWA_USE_IMMERSIVE_DARK_MODE
            &dark_mode as *const u32 as *const std::ffi::c_void,
            4,
        );

        // 2. Set custom app icon directly on the window and taskbar
        let candidate_paths = [
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("EqualizerFD-Craft.ico"))),
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("icon.ico"))),
            Some(std::path::PathBuf::from("EqualizerFD-Craft.ico")),
            Some(std::path::PathBuf::from("icon.ico")),
            Some(std::path::PathBuf::from(r"D:\testik\EqualizerFD-Craft.ico")),
        ];

        for opt_path in candidate_paths.into_iter().flatten() {
            if opt_path.exists() {
                let wide: Vec<u16> = opt_path.to_string_lossy().encode_utf16().chain(Some(0)).collect();
                if let Ok(handle) = LoadImageW(
                    None,
                    windows::core::PCWSTR(wide.as_ptr()),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                ) {
                    if !handle.is_invalid() {
                        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(1), LPARAM(handle.0 as isize));
                        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(0), LPARAM(handle.0 as isize));
                        break;
                    }
                }
            }
        }
    }
}

pub fn move_window_by(dx: i32, dy: i32) {
    if let Some(hwnd) = find_app_window() {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left + dx,
                    rect.top + dy,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}

pub fn focus_existing_window() -> bool {
    if let Some(hwnd) = find_app_window() {
        unsafe {
            // 1. Un-hide and restore window
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );

            // 2. Bypass Windows foreground restriction via thread input attachment
            let fg = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg, None);
            let cur_thread = GetCurrentThreadId();
            if fg_thread != cur_thread && fg_thread != 0 {
                let _ = AttachThreadInput(cur_thread, fg_thread, BOOL(1));
            }
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            if fg_thread != cur_thread && fg_thread != 0 {
                let _ = AttachThreadInput(cur_thread, fg_thread, BOOL(0));
            }
        }
        true
    } else {
        false
    }
}

pub fn hide_window() {
    if let Some(hwnd) = find_app_window() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn minimize_window() {
    if let Some(hwnd) = find_app_window() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
    }
}

pub fn toggle_maximize_window() {
    if let Some(hwnd) = find_app_window() {
        unsafe {
            if IsZoomed(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            } else {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
        }
    }
}

pub fn close_window() {
    kill_app_browser_processes();
    std::process::exit(0);
}

pub fn kill_app_browser_processes() {
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name = 'msedge.exe'\" | Where-Object { $_.CommandLine -like '*EqualizerFDCraft\\profile*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

pub fn wait_for_window_and_monitor() {
    let mut last_hwnd: Option<HWND> = None;
    loop {
        if let Some(hwnd) = find_app_window() {
            if last_hwnd != Some(hwnd) {
                make_window_frameless(hwnd);
                last_hwnd = Some(hwnd);
                set_stored_app_hwnd(hwnd);
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

