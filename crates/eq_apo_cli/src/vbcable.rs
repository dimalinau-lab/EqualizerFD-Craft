//! VB-Audio Virtual Cable Management and Installer Integration
//! Allows 1-click offline installation of VB-Audio Virtual Cable driver for Equalizer FD CRAFT

use std::env;
use std::path::{PathBuf};
use std::process::Command;
use std::os::windows::process::CommandExt;
use serde::{Deserialize, Serialize};
use crate::devices;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VbCableStatus {
    pub installed: bool,
    pub installer_available: bool,
    pub installer_path: String,
    pub devices: Vec<String>,
}

pub fn get_installer_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    
    // Relative to current running executable
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("tools").join("vbcable").join("VBCABLE_Setup_x64.exe"));
            candidates.push(parent.join("vbcable").join("VBCABLE_Setup_x64.exe"));
        }
    }

    // Relative to current working directory
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("tools").join("vbcable").join("VBCABLE_Setup_x64.exe"));
        candidates.push(cwd.join("vbcable").join("VBCABLE_Setup_x64.exe"));
    }

    // Fixed path in repo if running during dev
    candidates.push(PathBuf::from(r"D:\testik\tools\vbcable\VBCABLE_Setup_x64.exe"));

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

pub fn check_vbcable_installed() -> VbCableStatus {
    let mut all = devices::enumerate_endpoints("Capture");
    all.extend(devices::enumerate_endpoints("Render"));
    let mut cable_devices = Vec::new();

    for d in all {
        let name_lower = d.name.to_lowercase();
        if name_lower.contains("cable") || name_lower.contains("vb-audio") {
            cable_devices.push(format!("{} ({})", d.name, d.endpoint_type));
        }
    }

    let installed = !cable_devices.is_empty();
    let installer = get_installer_path();
    let installer_available = installer.is_some();
    let installer_path = installer.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    VbCableStatus {
        installed,
        installer_available,
        installer_path,
        devices: cable_devices,
    }
}

pub fn launch_vbcable_installer() -> Result<String, String> {
    let installer = get_installer_path();
    if let Some(path) = installer {
        let path_str = path.to_string_lossy().to_string();
        println!("[Equalizer FD CRAFT] Launching VB-Cable installer: {}", path_str);
        
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle", "Hidden",
                "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", path_str.replace("'", "''"))
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        match status {
            Ok(s) if s.success() => {
                Ok("Установщик VB-Audio Virtual Cable успешно запущен от имени Администратора. В появившемся окне нажмите кнопку 'Install Driver', после чего перезагрузите ПК или перезапустите аудиослужбу.".to_string())
            }
            Ok(_) => {
                let _ = Command::new(&path_str).spawn();
                Ok("Запущен установщик VB-Cable. Подтвердите права Администратора и нажмите 'Install Driver'.".to_string())
            }
            Err(e) => Err(format!("Не удалось запустить установщик: {}", e)),
        }
    } else {
        // Fallback: open official download site
        let _ = Command::new("cmd")
            .args(["/c", "start", "https://vb-audio.com/Cable/"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
        Ok("Файл инсталлятора не найден локально. В браузере открыта официальная страница загрузки VB-Audio Virtual Cable.".to_string())
    }
}

pub fn launch_vbcable_control_panel() -> Result<String, String> {
    let mut candidates = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("tools").join("vbcable").join("VBCABLE_ControlPanel.exe"));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("tools").join("vbcable").join("VBCABLE_ControlPanel.exe"));
    }
    candidates.push(PathBuf::from(r"D:\testik\tools\vbcable\VBCABLE_ControlPanel.exe"));

    for c in candidates {
        if c.exists() {
            let path_str = c.to_string_lossy().to_string();
            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle", "Hidden",
                    "-Command",
                    &format!("Start-Process -FilePath '{}' -Verb RunAs", path_str.replace("'", "''"))
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            return Ok("Панель управления VB-Cable запущена.".to_string());
        }
    }
    Err("VBCABLE_ControlPanel.exe не найден.".to_string())
}
