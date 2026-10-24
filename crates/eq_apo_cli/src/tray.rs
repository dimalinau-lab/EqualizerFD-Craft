//! Windows System Tray (Notification Area) Manager for Equalizer FD CRAFT
//! Launches the native background notification service with full context menu controls

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

const CREATE_NO_WINDOW: u32 = 0x08000000;
static TRAY_STARTED: AtomicBool = AtomicBool::new(false);

fn find_powershell_exe() -> PathBuf {
    let candidates = [
        r"C:\Program Files\PowerShell\7\pwsh.exe",
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
    ];

    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }

    PathBuf::from("pwsh.exe")
}

fn find_tray_script() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("tray.ps1"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("tray.ps1"));
    }

    candidates.push(PathBuf::from(r"D:\testik\tray.ps1"));

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

pub fn start_tray_service() {
    if TRAY_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        if let Some(script_path) = find_tray_script() {
            let script_str = script_path.to_string_lossy().to_string();
            let ps_exe = find_powershell_exe();
            let ps_str = ps_exe.to_string_lossy().to_string();

            let res = Command::new(&ps_str)
                .args([
                    "-NoProfile",
                    "-WindowStyle",
                    "Hidden",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &script_str,
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();

            match res {
                Ok(_) => println!("[Tray] System tray service spawned successfully via '{}' using '{}'", script_str, ps_str),
                Err(e) => eprintln!("[Tray] Failed to spawn system tray: {:?}", e),
            }
        } else {
            eprintln!("[Tray] Warning: tray.ps1 not found, system tray not started");
        }
    });
}
