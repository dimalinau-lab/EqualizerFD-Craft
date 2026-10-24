//! GitHub Releases Desktop Auto-Updater for Equalizer FD CRAFT
//! Implements atomic in-place binary swap pattern for Windows

use std::fs;
use std::os::windows::process::CommandExt;
use std::process::Command;
use serde::{Deserialize, Serialize};

const CREATE_NO_WINDOW: u32 = 0x08000000;
pub const CURRENT_VERSION: &str = "0.1.0-BETA";
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
pub struct UpdateCheckResponse {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_name: String,
    pub release_notes: String,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub repo_private: bool,
    pub message: String,
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

    // Look for .exe asset
    let exe_asset = release.assets.iter().find(|a| a.name.to_lowercase().ends_with(".exe"));
    let download_url = exe_asset.map(|a| a.browser_download_url.clone()).unwrap_or_default();
    let asset_name = exe_asset.map(|a| a.name.clone()).unwrap_or_default();
    let asset_size = exe_asset.map(|a| a.size).unwrap_or(0);

    UpdateCheckResponse {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: clean_tag.to_string(),
        has_update: is_newer,
        release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_notes: release.body.unwrap_or_default(),
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

/// Performs atomic in-place binary swap and restarts application
pub fn download_and_install_update(download_url: &str, token_opt: Option<&str>) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("Не удалось определить текущий .exe: {}", e))?;
    let temp_exe = current_exe.with_extension("exe.download");
    let old_exe = current_exe.with_extension("exe.old");

    if temp_exe.exists() {
        let _ = fs::remove_file(&temp_exe);
    }

    let mut cmd = Command::new("curl.exe");
    cmd.args(["-L", "-s", "-o"]);
    cmd.arg(&temp_exe);
    cmd.args(["-H", &format!("User-Agent: EqualizerFD-Craft/{}", CURRENT_VERSION)]);

    if let Some(token) = token_opt {
        let clean = token.trim();
        if !clean.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", clean)]);
            // For GitHub private asset downloads
            cmd.args(["-H", "Accept: application/octet-stream"]);
        }
    }

    cmd.arg(download_url);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let status = cmd.status().map_err(|e| format!("Ошибка загрузки обновления curl: {}", e))?;
    if !status.success() {
        return Err("Ошибка при скачивании файла обновления".to_string());
    }

    let metadata = fs::metadata(&temp_exe).map_err(|e| format!("Загруженный файл не найден: {}", e))?;
    if metadata.len() < 100_000 {
        let _ = fs::remove_file(&temp_exe);
        return Err(format!("Загруженный файл поврежден или слишком мал (размер: {} байт)", metadata.len()));
    }

    // Atomic in-place swap on Windows:
    // 1. Delete previous .old if exists
    if old_exe.exists() {
        let _ = fs::remove_file(&old_exe);
    }

    // 2. Rename running .exe -> .exe.old (Allowed by Windows kernel!)
    fs::rename(&current_exe, &old_exe).map_err(|e| format!("Не удалось переименовать текущий файл: {}", e))?;

    // 3. Rename .download -> .exe
    if let Err(e) = fs::rename(&temp_exe, &current_exe) {
        // Rollback rename if moving downloaded fails
        let _ = fs::rename(&old_exe, &current_exe);
        return Err(format!("Не удалось установить новый исполняемый файл: {}", e));
    }

    // 4. Spawn new updated executable detached and exit current process
    Command::new(&current_exe)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Не удалось перезапустить обновленную программу: {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(150));
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
