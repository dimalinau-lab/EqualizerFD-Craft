//! Embedded HTTP and API Server for ResonaDSP Studio
//! Standard library only - zero external web dependencies

use crate::devices;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const INDEX_HTML: &str = include_str!("../../../web/index.html");
const STYLE_CSS: &str = include_str!("../../../web/style.css");
const APP_JS: &str = include_str!("../../../web/app.js");
const I18N_JS: &str = include_str!("../../../web/i18n.js");
const LOCALE_RU: &str = include_str!("../../../web/locales/ru.json");
const LOCALE_EN: &str = include_str!("../../../web/locales/en.json");
const MANIFEST_JSON: &str = include_str!("../../../web/manifest.json");
const FAVICON_ICO: &[u8] = include_bytes!("../../../web/favicon.ico");
const ICON_PNG: &[u8] = include_bytes!("../../../web/icon.png");
const ICON_192_PNG: &[u8] = include_bytes!("../../../web/icon-192.png");
const APPLE_TOUCH_ICON_PNG: &[u8] = include_bytes!("../../../web/apple-touch-icon.png");

use std::sync::Mutex;
use std::time::{Duration, Instant};

static CACHED_DEVICES: Mutex<Option<(Instant, String)>> = Mutex::new(None);
pub static STREAM_CONTROLLER: Mutex<Option<eq_apo_device::stream::StreamController>> = Mutex::new(None);
pub static WIZARD_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn invalidate_device_cache() {
    if let Ok(mut lock) = CACHED_DEVICES.lock() {
        *lock = None;
    }
}

pub fn is_stream_active() -> bool {
    if let Ok(lock) = STREAM_CONTROLLER.lock() {
        if let Some(ref ctrl) = *lock {
            return ctrl.is_active();
        }
    }
    false
}

pub fn toggle_stream() -> bool {
    if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
        if lock.is_none() {
            *lock = Some(eq_apo_device::stream::StreamController::new());
        }
        if let Some(ref mut ctrl) = *lock {
            if ctrl.is_active() {
                ctrl.stop();
                return false;
            } else {
                let _ = ctrl.start("Microphone".to_string(), "cable".to_string());
                return true;
            }
        }
    }
    false
}

pub fn start_embedded_server(port: u16) -> Result<(), String> {
    let mut listener = None;
    for attempt in 1..=20 {
        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => {
                eprintln!("[ResonaDSP Server] Port {} not ready (attempt {}/20: {:?}), retrying in 500ms...", port, attempt, e);
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    let listener = listener.ok_or_else(|| format!("Failed to bind to port {} after 20 attempts", port))?;

    println!("===========================================================");
    println!("  Equalizer FD CRAFT Configuration Studio is LIVE!");
    println!("  Local Studio:  http://127.0.0.1:{}", port);
    println!("===========================================================");

    // Seed shared memory from config.txt initially
    for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt", "config.txt"] {
        if let Ok(txt) = std::fs::read_to_string(path) {
            sync_config_to_shared_mem(&txt);
            break;
        }
    }

    // Auto-start live WASAPI stream bridge to Virtual Cable
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(500));
        let mut ctrl = eq_apo_device::stream::StreamController::new();
        match ctrl.start("fifine".to_string(), "cable".to_string()) {
            Ok(_) => {
                println!("[ResonaDSP Stream Bridge] Real-time audio bridge (fifine SC3 -> VB-Audio Cable) is ACTIVE!");
                if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                    *lock = Some(ctrl);
                }
            }
            Err(e) => {
                println!("[ResonaDSP Stream Bridge] Info: Auto-bridge deferred: {:?}", e);
            }
        }
    });

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                thread::spawn(|| handle_client(stream));
            }
        }
    });

    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1500)));
    let mut req_bytes = Vec::with_capacity(4096);
    let mut temp = [0u8; 4096];
    let mut content_length = 0usize;
    let mut body_start_idx = None;

    loop {
        let n = match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        req_bytes.extend_from_slice(&temp[..n]);

        if body_start_idx.is_none() {
            if let Some(pos) = req_bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                body_start_idx = Some(pos + 4);
                let header_str = String::from_utf8_lossy(&req_bytes[..pos]);
                for line in header_str.lines() {
                    let lower = line.to_ascii_lowercase();
                    if lower.starts_with("content-length:") {
                        if let Some(val_str) = line.split(':').nth(1) {
                            content_length = val_str.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            } else if let Some(pos) = req_bytes.windows(2).position(|w| w == b"\n\n") {
                body_start_idx = Some(pos + 2);
                let header_str = String::from_utf8_lossy(&req_bytes[..pos]);
                for line in header_str.lines() {
                    let lower = line.to_ascii_lowercase();
                    if lower.starts_with("content-length:") {
                        if let Some(val_str) = line.split(':').nth(1) {
                            content_length = val_str.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }
        }

        if let Some(start) = body_start_idx {
            if req_bytes.len() - start >= content_length {
                break;
            }
        }
    }

    if req_bytes.is_empty() {
        return;
    }

    let header_end = body_start_idx.unwrap_or(req_bytes.len());
    let req_headers = String::from_utf8_lossy(&req_bytes[..header_end]);
    let first_line = req_headers.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let raw_path = parts[1];
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    let body_str = if header_end <= req_bytes.len() {
        String::from_utf8_lossy(&req_bytes[header_end..]).to_string()
    } else {
        String::new()
    };

    if method == "GET" {
        match path {
            "/" | "/index.html" => {
                let content = std::fs::read_to_string("web/index.html")
                    .unwrap_or_else(|_| INDEX_HTML.to_string());
                respond_ok(&mut stream, "text/html; charset=utf-8", content.as_bytes())
            }
            "/style.css" => {
                let content = std::fs::read_to_string("web/style.css")
                    .unwrap_or_else(|_| STYLE_CSS.to_string());
                respond_ok(&mut stream, "text/css; charset=utf-8", content.as_bytes())
            }
            "/app.js" => {
                let content = std::fs::read_to_string("web/app.js")
                    .unwrap_or_else(|_| APP_JS.to_string());
                respond_ok(&mut stream, "application/javascript; charset=utf-8", content.as_bytes())
            }
            "/i18n.js" => {
                let content = std::fs::read_to_string("web/i18n.js")
                    .unwrap_or_else(|_| I18N_JS.to_string());
                respond_ok(&mut stream, "application/javascript; charset=utf-8", content.as_bytes())
            }
            "/locales/ru.json" => {
                let content = std::fs::read_to_string("web/locales/ru.json")
                    .unwrap_or_else(|_| LOCALE_RU.to_string());
                respond_ok(&mut stream, "application/json; charset=utf-8", content.as_bytes())
            }
            "/locales/en.json" => {
                let content = std::fs::read_to_string("web/locales/en.json")
                    .unwrap_or_else(|_| LOCALE_EN.to_string());
                respond_ok(&mut stream, "application/json; charset=utf-8", content.as_bytes())
            }
            "/favicon.ico" => {
                let bytes = std::fs::read("web/favicon.ico").unwrap_or_else(|_| FAVICON_ICO.to_vec());
                respond_ok(&mut stream, "image/x-icon", &bytes);
            }
            "/icon.png" => {
                let bytes = std::fs::read("web/icon.png").unwrap_or_else(|_| ICON_PNG.to_vec());
                respond_ok(&mut stream, "image/png", &bytes);
            }
            "/icon-192.png" => {
                let bytes = std::fs::read("web/icon-192.png").unwrap_or_else(|_| ICON_192_PNG.to_vec());
                respond_ok(&mut stream, "image/png", &bytes);
            }
            "/apple-touch-icon.png" => {
                let bytes = std::fs::read("web/apple-touch-icon.png").unwrap_or_else(|_| APPLE_TOUCH_ICON_PNG.to_vec());
                respond_ok(&mut stream, "image/png", &bytes);
            }
            "/manifest.json" => {
                let content = std::fs::read_to_string("web/manifest.json")
                    .unwrap_or_else(|_| MANIFEST_JSON.to_string());
                respond_ok(&mut stream, "application/manifest+json; charset=utf-8", content.as_bytes());
            }
            "/api/app/quit" => {
                respond_ok(&mut stream, "application/json", b"{\"status\":\"quitting\"}");
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    crate::window_manager::kill_app_browser_processes();
                    std::process::exit(0);
                });
            }
            "/api/window/hide" => {
                crate::window_manager::hide_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"hidden_to_tray\"}");
            }
            "/api/window/show" => {
                if !crate::window_manager::focus_existing_window() {
                    crate::open_or_focus_editor();
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"shown\"}");
            }
            "/api/window/minimize" => {
                crate::window_manager::minimize_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"minimized\"}");
            }
            "/api/window/maximize" => {
                crate::window_manager::toggle_maximize_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"maximized_toggled\"}");
            }
            "/api/window/move" => {
                if let Some(query) = raw_path.split('?').nth(1) {
                    let mut dx = 0;
                    let mut dy = 0;
                    for pair in query.split('&') {
                        let mut parts = pair.split('=');
                        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                            if k == "dx" { dx = v.parse::<i32>().unwrap_or(0); }
                            if k == "dy" { dy = v.parse::<i32>().unwrap_or(0); }
                        }
                    }
                    if dx != 0 || dy != 0 {
                        crate::window_manager::move_window_by(dx, dy);
                    }
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"moved\"}");
            }
            "/api/window/close" => {
                respond_ok(&mut stream, "application/json", b"{\"status\":\"closing\"}");
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    crate::window_manager::close_window();
                });
            }
            "/api/devices" => {
                let cached = {
                    if let Ok(lock) = CACHED_DEVICES.lock() {
                        if let Some((instant, ref json)) = *lock {
                            if instant.elapsed() < Duration::from_secs(15) {
                                Some(json.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                let json = if let Some(j) = cached {
                    j
                } else {
                    let mut all = devices::enumerate_endpoints("Render");
                    all.extend(devices::enumerate_endpoints("Capture"));
                    let j = serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string());
                    if let Ok(mut lock) = CACHED_DEVICES.lock() {
                        *lock = Some((Instant::now(), j.clone()));
                    }
                    j
                };
                respond_ok(&mut stream, "application/json", json.as_bytes());
            }
            p if p.starts_with("/api/config") => {
                let mut requested_file = "config.txt".to_string();
                if let Some(pos) = p.find('?') {
                    let query = &p[pos + 1..];
                    for param in query.split('&') {
                        if let Some((k, v)) = param.split_once('=') {
                            if k == "path" || k == "file" {
                                requested_file = v.to_string();
                            }
                        }
                    }
                }
                let clean_name = std::path::Path::new(&requested_file)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("config.txt");

                let apo_path = format!("C:\\Program Files\\EqualizerAPO\\config\\{}", clean_name);
                let content = std::fs::read_to_string(&apo_path)
                    .or_else(|_| std::fs::read_to_string(clean_name))
                    .or_else(|_| std::fs::read_to_string(&requested_file))
                    .unwrap_or_default();
                let resp = serde_json::json!({ "config": content, "filename": clean_name }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/live-status" => {
                let mut resp = serde_json::json!({
                    "active": false,
                    "heartbeat": 0,
                    "in_peak_db": -96.0,
                    "out_peak_db": -96.0,
                    "preamp_gain_db": 0.0,
                });

                if let Ok(handle) = resona_apo::SharedMemoryHandle::create_or_open() {
                    unsafe {
                        let sm = &*handle.ptr;
                        resp = serde_json::json!({
                            "active": sm.heartbeat > 0,
                            "heartbeat": sm.heartbeat,
                            "in_peak_db": sm.input_peak_db,
                            "out_peak_db": sm.output_peak_db,
                            "preamp_gain_db": sm.preamp_gain_db,
                            "sample_rate": sm.sample_rate,
                            "channels": sm.channels,
                        });
                    }
                }
                respond_ok(&mut stream, "application/json", resp.to_string().as_bytes());
            }
            "/api/autostart" => {
                let enabled = check_autostart_status();
                let resp = serde_json::json!({ "enabled": enabled }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/status" | "/api/stream/status" => {
                let active = if let Ok(lock) = STREAM_CONTROLLER.lock() {
                    lock.as_ref().map(|c| c.is_active()).unwrap_or(false)
                } else {
                    false
                };
                let wizard = WIZARD_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst);
                let resp = format!("{{\"active\":{},\"stream_active\":{},\"capture\":\"Microphone\",\"render\":\"VB-Audio Virtual Cable\",\"open_wizard\":{}}}", active, active, wizard);
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/wizard/trigger" => {
                WIZARD_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                respond_ok(&mut stream, "application/json", b"{\"status\":\"triggered\"}");
            }
            "/api/vbcable/status" => {
                let status = crate::vbcable::check_vbcable_installed();
                let resp = serde_json::to_string(&status).unwrap_or_default();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            p if p.starts_with("/api/update/check") => {
                let mut token: Option<String> = None;
                if let Some(pos) = p.find('?') {
                    let query = &p[pos + 1..];
                    for param in query.split('&') {
                        if let Some((k, v)) = param.split_once('=') {
                            if k == "token" && !v.trim().is_empty() {
                                token = Some(v.to_string());
                            }
                        }
                    }
                }
                let check_res = crate::updater::check_for_updates(token.as_deref());
                let resp = serde_json::to_string(&check_res).unwrap_or_default();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/update/progress" => {
                let prog = crate::updater::get_update_progress();
                let resp = serde_json::to_string(&prog).unwrap_or_default();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/info" => {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| ".".to_string());
                let vst_dir = if std::path::Path::new(r"D:\testik\vst_plugins").exists() {
                    r"D:\testik\vst_plugins\".to_string()
                } else {
                    format!("{}\\vst_plugins\\", exe_dir.trim_end_matches('\\'))
                };
                let resp = serde_json::json!({
                    "app_dir": exe_dir,
                    "vst_dir": vst_dir,
                    "version": crate::updater::CURRENT_VERSION
                }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            _ => respond_404(&mut stream),
        }
    } else if method == "POST" {
        match path {
            "/api/exit" => {
                println!("[ResonaDSP] Graceful shutdown requested. Stopping audio streams and exiting...");
                if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                    if let Some(ref mut ctrl) = *lock {
                        ctrl.stop();
                    }
                }
                thread::spawn(|| {
                    thread::sleep(Duration::from_millis(150));
                    std::process::exit(0);
                });
                respond_ok(&mut stream, "application/json", b"{\"status\":\"exiting\"}");
            }
            "/api/window/hide" => {
                crate::window_manager::hide_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"hidden_to_tray\"}");
            }
            "/api/window/show" => {
                if !crate::window_manager::focus_existing_window() {
                    crate::open_or_focus_editor();
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"shown\"}");
            }
            "/api/window/minimize" => {
                crate::window_manager::minimize_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"minimized\"}");
            }
            "/api/window/maximize" => {
                crate::window_manager::toggle_maximize_window();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"maximized_toggled\"}");
            }
            "/api/window/close" => {
                respond_ok(&mut stream, "application/json", b"{\"status\":\"closing\"}");
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    crate::window_manager::close_window();
                });
            }
            "/api/autostart" => {
                let mut enable = false;
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(e) = val.get("enabled").and_then(|v| v.as_bool()) {
                        enable = e;
                    }
                }
                let ok = set_autostart_status(enable);
                let resp = serde_json::json!({ "status": if ok { "ok" } else { "error" }, "enabled": enable }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/elevate" => {
                println!("[ResonaDSP] Requesting UAC Administrator elevation...");
                if let Ok(exe) = std::env::current_exe() {
                    let exe_str = exe.to_string_lossy().to_string();
                    let _ = std::process::Command::new("powershell")
                        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &format!("Start-Process '{}' -Verb RunAs", exe_str)])
                        .spawn();
                    thread::spawn(|| {
                        thread::sleep(Duration::from_millis(250));
                        std::process::exit(0);
                    });
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"elevating\"}");
            }
            "/api/save" => {
                let (config_content, target_filename) = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let content = val.get("config").and_then(|c| c.as_str()).map(|s| s.to_string()).unwrap_or_else(|| body_str.clone());
                    let filename = val.get("filename").and_then(|f| f.as_str()).map(|s| s.to_string()).unwrap_or_else(|| "config.txt".to_string());
                    (content, filename)
                } else {
                    (body_str.clone(), "config.txt".to_string())
                };

                let clean_name = std::path::Path::new(&target_filename)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("config.txt");

                let is_primary = clean_name.eq_ignore_ascii_case("config.txt");

                if is_primary {
                    let main_path = "C:\\Program Files\\EqualizerAPO\\config\\config.txt";
                    let backup_path = "config.txt";
                    let _ = std::fs::write(backup_path, &config_content);
                    sync_config_to_shared_mem(&config_content);

                    match std::fs::write(main_path, &config_content) {
                        Ok(_) => {
                            println!("[ResonaDSP] Primary configuration successfully synced to {}", main_path);
                            respond_ok(&mut stream, "application/json", b"{\"status\":\"saved\",\"target\":\"config.txt\"}");
                        }
                        Err(e) => {
                            eprintln!("[ResonaDSP] Warning writing to {}: {}", main_path, e);
                            let err_resp = format!("{{\"status\":\"saved_local\",\"target\":\"config.txt\",\"warning\":\"{}\"}}", e);
                            respond_ok(&mut stream, "application/json", err_resp.as_bytes());
                        }
                    }
                } else {
                    // Separate preset/include file: save to local and APO directories
                    let _ = std::fs::write(clean_name, &config_content);
                    let apo_preset_path = format!("C:\\Program Files\\EqualizerAPO\\config\\{}", clean_name);
                    let apo_saved = std::fs::write(&apo_preset_path, &config_content).is_ok();
                    println!("[ResonaDSP] Preset/Include '{}' saved (APO target synced: {})", clean_name, apo_saved);

                    // Real-time audio engine update:
                    // If config.txt includes this preset, sync the updated config.txt (with newly saved preset expanded)!
                    // Otherwise, sync the active preset being tweaked directly to shared memory!
                    let active_sound_config = if let Ok(main_txt) = std::fs::read_to_string(r"C:\Program Files\EqualizerAPO\config\config.txt").or_else(|_| std::fs::read_to_string("config.txt")) {
                        if main_txt.to_lowercase().contains(&clean_name.to_lowercase()) {
                            main_txt
                        } else {
                            config_content.clone()
                        }
                    } else {
                        config_content.clone()
                    };
                    sync_config_to_shared_mem(&active_sound_config);

                    let resp = serde_json::json!({
                        "status": "saved",
                        "target": clean_name,
                        "saved_in_apo": apo_saved
                    }).to_string();
                    respond_ok(&mut stream, "application/json", resp.as_bytes());
                }
            }
            "/api/launch-configurator" => {
                println!("[Equalizer FD CRAFT] Spawning DeviceSelector.exe via elevated process...");
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", "Start-Process 'C:\\Program Files\\EqualizerAPO\\DeviceSelector.exe' -Verb RunAs"])
                    .spawn();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"launched\"}");
            }
            "/api/apo/restart-engine" => {
                devices::restart_audio_engine();
                respond_ok(&mut stream, "application/json", b"{\"status\":\"restarted\"}");
            }
            "/api/wizard/trigger" => {
                WIZARD_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                respond_ok(&mut stream, "application/json", b"{\"status\":\"triggered\"}");
            }
            "/api/stream/start" => {
                let mut cap = "Microphone".to_string();
                let mut ren = "cable".to_string();

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(c) = val.get("capture").and_then(|s| s.as_str()) {
                        if !c.trim().is_empty() { cap = c.to_string(); }
                    }
                    if let Some(r) = val.get("render").and_then(|s| s.as_str()) {
                        if !r.trim().is_empty() { ren = r.to_string(); }
                    }
                }

                if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                    if lock.is_none() {
                        *lock = Some(eq_apo_device::stream::StreamController::new());
                    }
                    if let Some(ref mut ctrl) = *lock {
                        let _ = ctrl.start(cap, ren);
                    }
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"started\"}");
            }
            "/api/stream/stop" => {
                if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                    if let Some(ref mut ctrl) = *lock {
                        ctrl.stop();
                    }
                }
                respond_ok(&mut stream, "application/json", b"{\"status\":\"stopped\"}");
            }
            "/api/stream/toggle" => {
                let mut is_active = false;
                if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                    if let Some(ref mut ctrl) = *lock {
                        if ctrl.is_active() {
                            ctrl.stop();
                        } else {
                            let _ = ctrl.start("Microphone".to_string(), "cable".to_string());
                            is_active = true;
                        }
                    } else {
                        let mut new_ctrl = eq_apo_device::stream::StreamController::new();
                        let _ = new_ctrl.start("Microphone".to_string(), "cable".to_string());
                        is_active = true;
                        *lock = Some(new_ctrl);
                    }
                }
                let resp = format!("{{\"status\":\"{}\",\"active\":{},\"stream_active\":{}}}", if is_active { "started" } else { "stopped" }, is_active, is_active);
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/apo/apply-batch" => {
                let mut applied_count = 0;
                let mut errors = Vec::new();

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(list) = val.get("devices").and_then(|d| d.as_array()) {
                        for item in list {
                            let guid = item.get("guid").and_then(|g| g.as_str()).unwrap_or("");
                            let ep_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("Capture");
                            let enabled = item.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                            let mode = item.get("mode").and_then(|m| m.as_str()).unwrap_or("ALL");

                            if !guid.is_empty() {
                                let res = if enabled {
                                    devices::install_endpoint_hook(guid, ep_type, mode)
                                } else {
                                    devices::uninstall_endpoint_hook(guid, ep_type)
                                };

                                match res {
                                    Ok(_) => applied_count += 1,
                                    Err(e) => errors.push(format!("{}: {}", guid, e)),
                                }
                            }
                        }
                    }
                }

                devices::restart_audio_engine();
                invalidate_device_cache();

                let resp = serde_json::json!({
                    "status": if errors.is_empty() { "ok" } else { "partial_error" },
                    "applied": applied_count,
                    "errors": errors
                }).to_string();

                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/apo/backup" | "/api/devices/backup" => {
                let mut backup_path = String::new();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let guid = val.get("guid").and_then(|g| g.as_str()).unwrap_or("");
                    let ep_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("Capture");
                    let endpoints = devices::enumerate_endpoints(ep_type);
                    if let Some(ep) = endpoints.iter().find(|e| e.guid.eq_ignore_ascii_case(guid)) {
                        let out_dir = std::path::Path::new(r"d:\testik\replica");
                        if let Ok(p) = devices::create_registry_backup(ep, out_dir) {
                            backup_path = p;
                        }
                    }
                }

                let resp = serde_json::json!({
                    "status": if !backup_path.is_empty() { "created" } else { "failed" },
                    "file": backup_path
                }).to_string();

                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/update/install" => {
                let mut download_url = String::new();
                let mut expected_size = 0u64;
                let mut token = None;
                let mut asset_name: Option<String> = None;
                let mut is_installer = false;
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(u) = val.get("url").and_then(|v| v.as_str()) {
                        download_url = u.to_string();
                    }
                    if let Some(s) = val.get("size").and_then(|v| v.as_u64()) {
                        expected_size = s;
                    }
                    if let Some(t) = val.get("token").and_then(|v| v.as_str()) {
                        if !t.trim().is_empty() {
                            token = Some(t.to_string());
                        }
                    }
                    if let Some(a) = val.get("asset_name").and_then(|v| v.as_str()) {
                        if !a.trim().is_empty() {
                            asset_name = Some(a.to_string());
                        }
                    }
                    if let Some(inst) = val.get("is_installer").and_then(|v| v.as_bool()) {
                        is_installer = inst;
                    }
                }

                if download_url.is_empty() {
                    let resp = serde_json::json!({
                        "status": "error",
                        "message": "Не указана ссылка для скачивания обновления (download_url)"
                    }).to_string();
                    respond_ok(&mut stream, "application/json", resp.as_bytes());
                } else {
                    respond_ok(&mut stream, "application/json", b"{\"status\":\"started\",\"message\":\"\xD0\x97\xD0\xB0\xD0\xBF\xD1\x83\xD1\x81\xD0\xBA \xD1\x81\xD0\xBA\xD0\xB0\xD1\x87\xD0\xB8\xD0\xB2\xD0\xB0\xD0\xBD\xD0\xB8\xD1\x8F \xD0\xBE\xD0\xB1\xD0\xBD\xD0\xBE\xD0\xB2\xD0\xBB\xD0\xB5\xD0\xBD\xD0\xB8\xD1\x8F...\"}");
                    thread::spawn(move || {
                        // Stop audio stream gracefully prior to restart
                        if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
                            if let Some(ref mut ctrl) = *lock {
                                ctrl.stop();
                            }
                        }
                        if let Err(e) = crate::updater::download_and_install_update(&download_url, expected_size, token.as_deref(), asset_name.as_deref(), is_installer) {
                            eprintln!("[Equalizer FD CRAFT Updater] Error: {}", e);
                        }
                    });
                }
            }
            "/api/vbcable/install" => {
                let res = crate::vbcable::launch_vbcable_installer();
                let (status, msg) = match res {
                    Ok(m) => ("ok", m),
                    Err(e) => ("error", e),
                };
                let resp = serde_json::json!({ "status": status, "message": msg }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/vbcable/control-panel" => {
                let res = crate::vbcable::launch_vbcable_control_panel();
                let (status, msg) = match res {
                    Ok(m) => ("ok", m),
                    Err(e) => ("error", e),
                };
                let resp = serde_json::json!({ "status": status, "message": msg }).to_string();
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            _ => respond_404(&mut stream),
        }
    } else {
        respond_404(&mut stream);
    }
}

fn sync_config_to_shared_mem(config_str: &str) {
    let expanded = eq_apo_core::ConfigFile::expand_includes(config_str, 0);
    if let Ok(config) = eq_apo_core::ConfigFile::parse(&expanded, 48000.0) {
        if let Ok(handle) = resona_apo::SharedMemoryHandle::create_or_open() {
            unsafe {
                let sm = &mut *handle.ptr;
                sm.version = sm.version.wrapping_add(1);
                sm.enabled = 1;
                sm.preamp_gain_db = 0.0;
                sm.filter_count = 0;

                let bytes = expanded.as_bytes();
                let len = bytes.len().min(sm.config_bytes.len());
                sm.config_bytes[..len].copy_from_slice(&bytes[..len]);
                sm.config_len = len as u32;

                for directive in &config.directives {
                    match directive {
                        eq_apo_core::ConfigDirective::Preamp { gain_db, .. } => {
                            sm.preamp_gain_db += *gain_db as f32;
                        }
                        eq_apo_core::ConfigDirective::Filter { is_enabled, params, .. } => {
                            let idx = sm.filter_count as usize;
                            if idx < 16 {
                                let f_type = match params.filter_type {
                                    eq_apo_core::FilterType::Peaking => 0,
                                    eq_apo_core::FilterType::LowShelf => 1,
                                    eq_apo_core::FilterType::HighShelf => 2,
                                    eq_apo_core::FilterType::LowPass => 3,
                                    eq_apo_core::FilterType::HighPass => 4,
                                    eq_apo_core::FilterType::Notch => 5,
                                    _ => 0,
                                };
                                sm.filters[idx] = resona_apo::SharedFilter {
                                    filter_type: f_type,
                                    enabled: if *is_enabled { 1 } else { 0 },
                                    fc: params.frequency_hz as f32,
                                    gain_db: params.gain_db as f32,
                                    q: params.q_factor as f32,
                                };
                                sm.filter_count += 1;
                            }
                        }
                        eq_apo_core::ConfigDirective::GraphicEQ { points, .. } => {
                            for (i, p) in points.iter().enumerate().take(15) {
                                sm.geq_bands[i] = p.1 as f32;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn respond_ok(stream: &mut TcpStream, content_type: &str, body: &[u8]) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-cache, no-store, must-revalidate\r\nPragma: no-cache\r\nExpires: 0\r\nConnection: close\r\n\r\n",
        content_type,
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn respond_404(stream: &mut TcpStream) {
    let body = b"404 Not Found";
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

use std::os::windows::process::CommandExt;
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn check_autostart_status() -> bool {
    let out = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "EqualizerFDCraft"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match out {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

pub fn set_autostart_status(enable: bool) -> bool {
    if enable {
        if let Ok(exe) = std::env::current_exe() {
            let exe_str = exe.to_string_lossy().to_string();
            let res = std::process::Command::new("reg")
                .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "EqualizerFDCraft", "/t", "REG_SZ", "/d", &format!("\"{}\"", exe_str), "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            return res.map(|s| s.success()).unwrap_or(false);
        }
        false
    } else {
        let res = std::process::Command::new("reg")
            .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "EqualizerFDCraft", "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        res.map(|s| s.success()).unwrap_or(false)
    }
}
