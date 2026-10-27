//! GitHub Releases Desktop Auto-Updater for Equalizer FD CRAFT
//! Implements atomic in-place binary swap pattern for Windows

use std::fs;
use std::os::windows::process::CommandExt;
use std::process::Command;
use serde::{Deserialize, Serialize};

const CREATE_NO_WINDOW: u32 = 0x08000000;
pub const CURRENT_VERSION: &str = "1.0.0";
pub const REPO_OWNER: &str = "dimalinau-lab";
pub const REPO_NAME: &str = "EqualizerFD-Craft";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub url: Option<String>,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateOption {
    pub mode: String, // "fast" or "full"
    pub title: String,
    pub description: String,
    pub asset_name: String,
    pub download_url: String,
    pub size: u64,
    pub is_installer: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateCheckResponse {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_name: String,
    pub release_notes: String,
    pub fast_update: Option<UpdateOption>,
    pub full_install: Option<UpdateOption>,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub repo_private: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateProgress {
    pub status: String, // "idle", "downloading", "applying", "restarting", "completed", "error"
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub percent: f32,
    pub message: String,
    pub error_message: Option<String>,
}

static UPDATE_PROGRESS: std::sync::Mutex<UpdateProgress> = std::sync::Mutex::new(UpdateProgress {
    status: String::new(),
    total_bytes: 0,
    downloaded_bytes: 0,
    percent: 0.0,
    message: String::new(),
    error_message: None,
});

pub fn get_update_progress() -> UpdateProgress {
    let lock = UPDATE_PROGRESS.lock().unwrap();
    if lock.status.is_empty() {
        UpdateProgress {
            status: "idle".to_string(),
            total_bytes: 0,
            downloaded_bytes: 0,
            percent: 0.0,
            message: "Ожидание".to_string(),
            error_message: None,
        }
    } else {
        lock.clone()
    }
}

pub fn set_update_progress(p: UpdateProgress) {
    if let Ok(mut lock) = UPDATE_PROGRESS.lock() {
        *lock = p;
    }
}

/// Housekeeping: cleans up any lingering .exe.old binaries from previous updates on startup
pub fn cleanup_previous_update() {
    if let Ok(exe_path) = std::env::current_exe() {
        let old_exe = exe_path.with_extension("exe.old");
        if old_exe.exists() {
            let _ = fs::remove_file(&old_exe);
        }
        // Also check if directory contains EqualizerFD-Craft.exe.old or ResonaDSP.exe.old
        if let Some(parent) = exe_path.parent() {
            let _ = fs::remove_file(parent.join("EqualizerFD-Craft.exe.old"));
            let _ = fs::remove_file(parent.join("ResonaDSP.exe.old"));
        }
    }
}

/// Checks GitHub API for latest release
pub fn check_for_updates(token_opt: Option<&str>) -> UpdateCheckResponse {
    let api_url = format!("https://api.github.com/repos/{}/{}/releases?per_page=1", REPO_OWNER, REPO_NAME);
    
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-s", "-H", "Accept: application/vnd.github+json", "-H", &format!("User-Agent: EqualizerFD-Craft/{}", CURRENT_VERSION)]);

    if let Some(token) = token_opt {
        let clean = token.trim();
        if !clean.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", clean)]);
        }
    }

    cmd.arg(&api_url);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return UpdateCheckResponse {
                current_version: CURRENT_VERSION.to_string(),
                latest_version: CURRENT_VERSION.to_string(),
                has_update: false,
                release_name: String::new(),
                release_notes: String::new(),
                fast_update: None,
                full_install: None,
                download_url: String::new(),
                asset_name: String::new(),
                asset_size: 0,
                repo_private: false,
                message: format!("Ошибка запуска сетевого модуля curl: {}", e),
            };
        }
    };

    let text = String::from_utf8_lossy(&output.stdout).to_string();

    let release_opt: Option<GithubRelease> = if let Ok(list) = serde_json::from_str::<Vec<GithubRelease>>(&text) {
        list.into_iter().next()
    } else if let Ok(single) = serde_json::from_str::<GithubRelease>(&text) {
        Some(single)
    } else {
        None
    };

    let release = match release_opt {
        Some(r) => r,
        None => {
            let (is_private, message) = if text.contains("Bad credentials") {
                (true, "Неверный GitHub Personal Access Token.".to_string())
            } else if text.contains("\"message\":\"Not Found\"") {
                (false, "Релизы на GitHub пока не найдены.".to_string())
            } else {
                (false, "Нет доступных релизов на GitHub.".to_string())
            };
            return UpdateCheckResponse {
                current_version: CURRENT_VERSION.to_string(),
                latest_version: CURRENT_VERSION.to_string(),
                has_update: false,
                release_name: String::new(),
                release_notes: String::new(),
                fast_update: None,
                full_install: None,
                download_url: String::new(),
                asset_name: String::new(),
                asset_size: 0,
                repo_private: is_private,
                message,
            };
        }
    };

    let clean_tag = release.tag_name.trim_start_matches('v').trim();
    let is_newer = is_newer_version(CURRENT_VERSION, clean_tag);

    // 1. Detect Fast Update Asset (e.g. EqualizerFD-Craft-Update-Installer.exe or update zip)
    let fast_asset = release.assets.iter().find(|a| {
        let lower = a.name.to_lowercase();
        lower.ends_with(".exe") && (lower.contains("update-installer") || lower.contains("update_installer") || lower.contains("mini-setup"))
    }).or_else(|| {
        release.assets.iter().find(|a| {
            let lower = a.name.to_lowercase();
            lower.ends_with(".zip") && lower.contains("update")
        })
    }).or_else(|| {
        // Fallback: standalone exe that is not full setup
        release.assets.iter().find(|a| {
            let lower = a.name.to_lowercase();
            lower.ends_with(".exe") && !lower.contains("setup")
        })
    });

    // 2. Detect Full Reinstallation Asset (Inno Setup with full plugins, ~165 MB)
    let full_asset = release.assets.iter().find(|a| {
        let lower = a.name.to_lowercase();
        lower.ends_with(".exe") && (lower.contains("setup") || lower.contains("full") || a.size > 50_000_000)
    });

    let fast_update = fast_asset.map(|a| UpdateOption {
        mode: "fast".to_string(),
        title: "Быстрое обновление (без повторного скачивания плагинов)".to_string(),
        description: "Лёгкий инсталлятор обновления (~3 МБ). Быстро обновляет исполняемые файлы Equalizer FD CRAFT, веб-интерфейс и системный трей, сохраняя ваши настройки и установленные плагины.".to_string(),
        asset_name: a.name.clone(),
        download_url: a.browser_download_url.clone(),
        size: a.size,
        is_installer: a.name.to_lowercase().ends_with(".exe"),
    });

    let full_install = full_asset.map(|a| UpdateOption {
        mode: "full".to_string(),
        title: "Полная переустановка (все VST плагины)".to_string(),
        description: "Официальный полный инсталлятор Inno Setup (~166 МБ). Включает все 98 профессиональных студийных VST-плагинов (ReaPlugs, Blue Cat, Kuassa), регистрацию компонентов и чистую переустановку.".to_string(),
        asset_name: a.name.clone(),
        download_url: a.browser_download_url.clone(),
        size: a.size,
        is_installer: true,
    });

    let primary_asset = fast_asset.or(full_asset);
    let download_url = primary_asset.map(|a| a.browser_download_url.clone()).unwrap_or_default();
    let asset_name = primary_asset.map(|a| a.name.clone()).unwrap_or_default();
    let asset_size = primary_asset.map(|a| a.size).unwrap_or(0);

    UpdateCheckResponse {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: clean_tag.to_string(),
        has_update: is_newer,
        release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_notes: release.body.unwrap_or_default(),
        fast_update,
        full_install,
        download_url,
        asset_name,
        asset_size,
        repo_private: false,
        message: if is_newer {
            format!("Доступна новая версия: v{} (текущая: v{})", clean_tag, CURRENT_VERSION)
        } else {
            format!("У вас установлена последняя версия Equalizer FD CRAFT (v{})", CURRENT_VERSION)
        },
    }
}

/// Downloads update and executes installer or performs in-place binary swap
pub fn download_and_install_update(
    download_url: &str,
    expected_size: u64,
    token_opt: Option<&str>,
    asset_name_opt: Option<&str>,
    mut is_installer: bool,
) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| {
        let err = format!("Не удалось определить текущий .exe: {}", e);
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: expected_size,
            downloaded_bytes: 0,
            percent: 0.0,
            message: "Ошибка обновления".to_string(),
            error_message: Some(err.clone()),
        });
        err
    })?;

    let asset_name = asset_name_opt.unwrap_or("EqualizerFD-Craft-Update-Installer.exe").trim();
    let lower_asset = asset_name.to_lowercase();
    if lower_asset.contains("installer") || lower_asset.contains("setup") || lower_asset.ends_with(".exe") {
        is_installer = true;
    }

    let target_file = if is_installer {
        let safe_name = if asset_name.is_empty() {
            "EqualizerFD-Craft-Update-Installer.exe"
        } else {
            asset_name
        };
        std::env::temp_dir().join(safe_name)
    } else {
        current_exe.with_extension("exe.download")
    };

    let old_exe = current_exe.with_extension("exe.old");

    if target_file.exists() {
        let _ = fs::remove_file(&target_file);
    }

    set_update_progress(UpdateProgress {
        status: "downloading".to_string(),
        total_bytes: expected_size,
        downloaded_bytes: 0,
        percent: 0.0,
        message: format!("Подключение к серверу загрузки ({})...", asset_name),
        error_message: None,
    });

    let mut cmd = Command::new("curl.exe");
    cmd.args(["-L", "-s", "-o"]);
    cmd.arg(&target_file);
    cmd.args(["-H", &format!("User-Agent: EqualizerFD-Craft/{}", CURRENT_VERSION)]);

    if let Some(token) = token_opt {
        let clean = token.trim();
        if !clean.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", clean)]);
            cmd.args(["-H", "Accept: application/octet-stream"]);
        }
    }

    cmd.arg(download_url);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().map_err(|e| {
        let err = format!("Ошибка запуска curl: {}", e);
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: expected_size,
            downloaded_bytes: 0,
            percent: 0.0,
            message: "Ошибка запуска загрузчика".to_string(),
            error_message: Some(err.clone()),
        });
        err
    })?;

    // Monitor file size in real-time while curl downloads
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let err = "Ошибка загрузки файла обновления через curl".to_string();
                    set_update_progress(UpdateProgress {
                        status: "error".to_string(),
                        total_bytes: expected_size,
                        downloaded_bytes: 0,
                        percent: 0.0,
                        message: "Ошибка загрузки".to_string(),
                        error_message: Some(err.clone()),
                    });
                    return Err(err);
                }
                break;
            }
            Ok(None) => {
                if let Ok(meta) = fs::metadata(&target_file) {
                    let dl = meta.len();
                    let percent = if expected_size > 0 {
                        ((dl as f64 / expected_size as f64) * 100.0).clamp(0.0, 99.0) as f32
                    } else {
                        0.0
                    };
                    set_update_progress(UpdateProgress {
                        status: "downloading".to_string(),
                        total_bytes: expected_size,
                        downloaded_bytes: dl,
                        percent,
                        message: format!("Скачивание: {:.0}%", percent),
                        error_message: None,
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
            Err(e) => {
                let err = format!("Ошибка отслеживания процесса: {}", e);
                set_update_progress(UpdateProgress {
                    status: "error".to_string(),
                    total_bytes: expected_size,
                    downloaded_bytes: 0,
                    percent: 0.0,
                    message: "Ошибка".to_string(),
                    error_message: Some(err.clone()),
                });
                return Err(err);
            }
        }
    }

    let metadata = fs::metadata(&target_file).map_err(|e| {
        let err = format!("Загруженный файл не найден: {}", e);
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: expected_size,
            downloaded_bytes: 0,
            percent: 0.0,
            message: "Ошибка проверки файла".to_string(),
            error_message: Some(err.clone()),
        });
        err
    })?;

    if metadata.len() < 100_000 {
        let _ = fs::remove_file(&target_file);
        let err = format!("Загруженный файл поврежден или слишком мал ({} байт)", metadata.len());
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: expected_size,
            downloaded_bytes: metadata.len(),
            percent: 0.0,
            message: "Поврежденный файл".to_string(),
            error_message: Some(err.clone()),
        });
        return Err(err);
    }

    if is_installer {
        set_update_progress(UpdateProgress {
            status: "restarting".to_string(),
            total_bytes: metadata.len(),
            downloaded_bytes: metadata.len(),
            percent: 100.0,
            message: "Запуск программы установки...".to_string(),
            error_message: None,
        });

        // Launch installer via Windows ShellExecuteW with "runas" to trigger UAC elevation prompt
        let path_hstr = windows::core::HSTRING::from(target_file.to_string_lossy().as_ref());
        let op_hstr = windows::core::HSTRING::from("runas");

        unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(
                windows::Win32::Foundation::HWND(std::ptr::null_mut()),
                &op_hstr,
                &path_hstr,
                windows::core::PCWSTR::null(),
                windows::core::PCWSTR::null(),
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(1000));
        std::process::exit(0);
    }

    set_update_progress(UpdateProgress {
        status: "applying".to_string(),
        total_bytes: metadata.len(),
        downloaded_bytes: metadata.len(),
        percent: 100.0,
        message: "Применение обновления...".to_string(),
        error_message: None,
    });

    // Atomic in-place swap on Windows:
    // 1. Delete previous .old if exists
    if old_exe.exists() {
        let _ = fs::remove_file(&old_exe);
    }

    // 2. Rename running .exe -> .exe.old (Allowed by Windows kernel)
    fs::rename(&current_exe, &old_exe).map_err(|e| {
        let err = format!("Не удалось переименовать текущий файл: {}", e);
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: metadata.len(),
            downloaded_bytes: metadata.len(),
            percent: 100.0,
            message: "Ошибка замены файла".to_string(),
            error_message: Some(err.clone()),
        });
        err
    })?;

    // 3. Rename target_file -> .exe
    if let Err(e) = fs::rename(&target_file, &current_exe) {
        let _ = fs::rename(&old_exe, &current_exe);
        let err = format!("Не удалось установить новый исполняемый файл: {}", e);
        set_update_progress(UpdateProgress {
            status: "error".to_string(),
            total_bytes: metadata.len(),
            downloaded_bytes: metadata.len(),
            percent: 100.0,
            message: "Ошибка распаковки".to_string(),
            error_message: Some(err.clone()),
        });
        return Err(err);
    }

    set_update_progress(UpdateProgress {
        status: "restarting".to_string(),
        total_bytes: metadata.len(),
        downloaded_bytes: metadata.len(),
        percent: 100.0,
        message: "Обновление установлено! Перезапуск студии...".to_string(),
        error_message: None,
    });

    // 4. Spawn new updated executable detached and exit current process
    Command::new(&current_exe)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| {
            let err = format!("Не удалось перезапустить обновленную программу: {}", e);
            set_update_progress(UpdateProgress {
                status: "error".to_string(),
                total_bytes: metadata.len(),
                downloaded_bytes: metadata.len(),
                percent: 100.0,
                message: "Ошибка перезапуска".to_string(),
                error_message: Some(err.clone()),
            });
            err
        })?;

    std::thread::sleep(std::time::Duration::from_millis(600));
    std::process::exit(0);
}

fn is_newer_version(current: &str, candidate: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.')
            .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    };

    let cur = parse(current);
    let cand = parse(candidate);

    for i in 0..cur.len().max(cand.len()) {
        let c_val = cur.get(i).copied().unwrap_or(0);
        let n_val = cand.get(i).copied().unwrap_or(0);
        if n_val > c_val {
            return true;
        } else if n_val < c_val {
            return false;
        }
    }
    false
}
