//! Windows Native System Tray (Notification Area) Manager for Equalizer FD CRAFT
//! Pure Win32 Shell_NotifyIcon implementation — zero external scripts, zero runtime bloat.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, LoadImageW, PostMessageW,
    PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu,
    HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WM_COMMAND,
    WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
    WS_OVERLAPPED,
};

const WM_TRAYICON: u32 = 0x800; // WM_APP (2048) matching WinForms TrayMouseMessage

const ID_TRAY_OPEN: usize = 1001;
const ID_TRAY_STREAM: usize = 1002;
const ID_TRAY_AUTOSTART: usize = 1003;
const ID_TRAY_RESTART_AUDIO: usize = 1004;
const ID_TRAY_OPEN_FOLDER: usize = 1005;
const ID_TRAY_EXIT: usize = 1006;

static TRAY_STARTED: AtomicBool = AtomicBool::new(false);
static mut GLOBAL_TRAY_HWND: HWND = HWND(std::ptr::null_mut());
static mut WM_TASKBAR_CREATED: u32 = 0;

fn load_tray_icon() -> HICON {
    unsafe {
        // 1. Try loading EqualizerFD-Craft.ico directly from filesystem
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
                    16,
                    16,
                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                ) {
                    if !handle.is_invalid() {
                        return HICON(handle.0);
                    }
                }
            }
        }

        // 2. Embedded resource icon (ID 1)
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        if let Ok(icon) = LoadIconW(hinstance, windows::core::PCWSTR(1 as _)) {
            if !icon.is_invalid() {
                return icon;
            }
        }

        // 3. Fallback standard application icon
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

fn set_tooltip(nid: &mut NOTIFYICONDATAW, tip_str: &str) {
    let mut tip_chars = [0u16; 128];
    for (i, c) in tip_str.encode_utf16().take(127).enumerate() {
        tip_chars[i] = c;
    }
    nid.szTip = tip_chars;
}

#[repr(C, packed(4))]
struct NotifyIconData952 {
    cb_size: u32,
    _pad1: u32,
    hwnd: HWND,
    uid: u32,
    flags: u32,
    callback_msg: u32,
    _pad2: u32,
    icon: HICON,
    tip: [u16; 128],
    dw_state: u32,
    dw_state_mask: u32,
    info: [u16; 256],
    timeout_or_version: u32,
    info_title: [u16; 64],
    info_flags: u32,
}

fn add_or_update_tray_icon(hwnd: HWND) -> bool {
    unsafe {
        let icon = load_tray_icon();
        println!("load_tray_icon handle: {:?}", icon.0);

        // 1. Try with custom 952-byte struct
        let mut d952 = NotifyIconData952 {
            cb_size: std::mem::size_of::<NotifyIconData952>() as u32,
            _pad1: 0,
            hwnd,
            uid: 1,
            flags: 0x01 | 0x02 | 0x04, // NIF_MESSAGE | NIF_ICON | NIF_TIP
            callback_msg: WM_TRAYICON,
            _pad2: 0,
            icon,
            tip: [0u16; 128],
            dw_state: 0,
            dw_state_mask: 0,
            info: [0u16; 256],
            timeout_or_version: 0,
            info_title: [0u16; 64],
            info_flags: 0,
        };
        for (i, c) in "Equalizer FD CRAFT - Studio Audio Engine".encode_utf16().take(127).enumerate() {
            d952.tip[i] = c;
        }

        println!("d952 size: {}", d952.cb_size);
        let res952 = Shell_NotifyIconW(NIM_ADD, &d952 as *const _ as *const NOTIFYICONDATAW);
        let err952 = GetLastError();
        println!("d952 NIM_ADD res: {}, err: {:?}", res952.as_bool(), err952);
        if res952.as_bool() {
            return true;
        }

        // 2. Try standard windows-rs NOTIFYICONDATAW (976 bytes)
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        set_tooltip(&mut nid, "Equalizer FD CRAFT - Studio Audio Engine");
        let res976 = Shell_NotifyIconW(NIM_ADD, &nid);
        let err976 = GetLastError();
        println!("d976 NIM_ADD res: {}, err: {:?}", res976.as_bool(), err976);
        res976.as_bool()
    }
}

pub fn update_tray_tooltip(status_text: &str) {
    unsafe {
        if GLOBAL_TRAY_HWND.0.is_null() {
            return;
        }
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: GLOBAL_TRAY_HWND,
            uID: 1,
            uFlags: NIF_TIP,
            ..Default::default()
        };
        let tip = format!("Equalizer FD CRAFT\n{}", status_text);
        set_tooltip(&mut nid, &tip);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg != 0x0200 && msg != 0x0084 && msg != 0x0020 {
        // Exclude mouse move and hit test spam
        println!("tray_wndproc msg: 0x{:04X}", msg);
    }
    if WM_TASKBAR_CREATED != 0 && msg == WM_TASKBAR_CREATED {
        // Explorer restarted, recreate the tray icon
        add_or_update_tray_icon(hwnd);
        return LRESULT(0);
    }

    match msg {
        WM_TRAYICON => {
            // With NOTIFYICON_VERSION_4: LOWORD(lparam) is the mouse event, HIWORD(lparam) is icon ID
            let event = (lparam.0 & 0xFFFF) as u32;
            match event {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    // Left-click / double-click: Open or focus main editor window
                    crate::open_or_focus_editor();
                }
                WM_RBUTTONUP => {
                    // Right-click: Show native popup context menu
                    if let Ok(hmenu) = CreatePopupMenu() {
                        let is_active = crate::server::is_stream_active();
                        let is_auto = crate::server::check_autostart_status();

                        let stream_label = if is_active {
                            "WASAPI аудиомост: [В ЭФИРЕ] (клик для паузы)"
                        } else {
                            "WASAPI аудиомост: [Пауза] (клик для запуска)"
                        };

                        let open_str: Vec<u16> = "Открыть Equalizer FD CRAFT\0".encode_utf16().collect();
                        let stream_str: Vec<u16> = format!("{}\0", stream_label).encode_utf16().collect();
                        let auto_str: Vec<u16> = "Автозагрузка с Windows\0".encode_utf16().collect();
                        let restart_str: Vec<u16> = "Перезапустить аудиослужбу (audiodg.exe)\0".encode_utf16().collect();
                        let folder_str: Vec<u16> = "Открыть папку с пресетами и VST\0".encode_utf16().collect();
                        let exit_str: Vec<u16> = "Выход\0".encode_utf16().collect();

                        let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_OPEN, windows::core::PCWSTR(open_str.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_STREAM, windows::core::PCWSTR(stream_str.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());

                        let auto_flags = MF_STRING | if is_auto { MF_CHECKED } else { MF_UNCHECKED };
                        let _ = AppendMenuW(hmenu, auto_flags, ID_TRAY_AUTOSTART, windows::core::PCWSTR(auto_str.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_RESTART_AUDIO, windows::core::PCWSTR(restart_str.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_OPEN_FOLDER, windows::core::PCWSTR(folder_str.as_ptr()));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
                        let _ = AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT, windows::core::PCWSTR(exit_str.as_ptr()));

                        let mut pt = POINT::default();
                        let _ = GetCursorPos(&mut pt);

                        let _ = SetForegroundWindow(hwnd);
                        let _ = TrackPopupMenu(
                            hmenu,
                            TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            None,
                        );
                        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
                        let _ = DestroyMenu(hmenu);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = wparam.0 as usize;
            match cmd_id {
                ID_TRAY_OPEN => {
                    crate::open_or_focus_editor();
                }
                ID_TRAY_STREAM => {
                    let active = crate::server::toggle_stream();
                    update_tray_tooltip(if active { "[В ЭФИРЕ]" } else { "[Пауза]" });
                }
                ID_TRAY_AUTOSTART => {
                    let current = crate::server::check_autostart_status();
                    let _ = crate::server::set_autostart_status(!current);
                }
                ID_TRAY_RESTART_AUDIO => {
                    crate::devices::restart_audio_engine();
                }
                ID_TRAY_OPEN_FOLDER => {
                    let _ = std::process::Command::new("explorer.exe").arg(".").spawn();
                }
                ID_TRAY_EXIT => {
                    let nid = NOTIFYICONDATAW {
                        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                        hWnd: hwnd,
                        uID: 1,
                        ..Default::default()
                    };
                    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                    let _ = DestroyWindow(hwnd);
                    crate::window_manager::kill_app_browser_processes();
                    std::process::exit(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn start_tray_service() {
    if TRAY_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    // 1. Try launching the specialized EqualizerTray companion
    let candidate_tray_paths = [
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("EqualizerTray.exe"))),
        Some(std::path::PathBuf::from("EqualizerTray.exe")),
        Some(std::path::PathBuf::from(r"D:\testik\EqualizerTray.exe")),
    ];

    let my_pid = std::process::id();
    for opt_path in candidate_tray_paths.into_iter().flatten() {
        if opt_path.exists() {
            if let Ok(_) = std::process::Command::new(&opt_path)
                .arg(format!("/parent:{}", my_pid))
                .spawn()
            {
                println!("[Tray Service] Started EqualizerTray companion: {:?}", opt_path);
                return;
            }
        }
    }

    // 2. Fallback to native Win32 message window
    std::thread::spawn(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let class_name = w!("EqualizerFDCraftTrayClass");
        let window_name = w!("EqualizerFDCraftTrayWindow");
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(tray_wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        let _ = RegisterClassExW(&wnd_class);

        WM_TASKBAR_CREATED = RegisterWindowMessageW(w!("TaskbarCreated"));

        let hwnd_res = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            class_name,
            window_name,
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        );

        let hwnd = match hwnd_res {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[Native Tray] Failed to create tray message window: {:?}", e);
                return;
            }
        };

        GLOBAL_TRAY_HWND = hwnd;

        // Register tray icon after window is successfully created
        let ok = add_or_update_tray_icon(hwnd);
        if !ok {
            let err = GetLastError();
            eprintln!("[Native Tray] Shell_NotifyIconW(NIM_ADD) failed with error: {:?}", err);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = DispatchMessageW(&msg);
        }
    });
}

pub fn run_tray_test() {
    println!("--- Testing Native Windows System Tray Registration ---");
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let class_name = w!("EqualizerFDCraftTrayTestClass");
        let window_name = w!("EqualizerFDCraftTrayTestWindow");
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(tray_wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        let _ = RegisterClassExW(&wnd_class);

        let hwnd = match CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00000100),
            class_name,
            window_name,
            windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x04C00000),
            0,
            0,
            136,
            39,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to create test window: {:?}", e);
                return;
            }
        };

        println!("Created message window: HWND({:?})", hwnd.0);
        let ok = add_or_update_tray_icon(hwnd);
        if ok {
            println!("SUCCESS: System tray icon successfully registered in Windows notification area!");
            println!("Icon is now visible in the taskbar / overflow chevron menu.");
            println!("Waiting 5 seconds, then cleaning up...");
            std::thread::sleep(std::time::Duration::from_secs(5));
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyWindow(hwnd);
            println!("Test completed cleanly.");
        } else {
            let err = GetLastError();
            println!("FAILED: Shell_NotifyIconW returned FALSE. Windows Error: {:?}", err);
        }
    }
}
