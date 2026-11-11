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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppSettings {
    #[serde(default)]
    pub monitor_active: bool,
    #[serde(default)]
    pub monitor_capture: String,
    #[serde(default = "default_monitor_render")]
    pub monitor_render: String,
    #[serde(default = "default_monitor_vol")]
    pub monitor_volume: f32,
}

fn default_monitor_render() -> String {
    "default".to_string()
}

fn default_monitor_vol() -> f32 {
    0.8
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            monitor_active: false,
            monitor_capture: String::new(),
            monitor_render: default_monitor_render(),
            monitor_volume: default_monitor_vol(),
        }
    }
}

pub fn get_settings_path() -> std::path::PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let dir = std::path::PathBuf::from(local).join("EqualizerFDCraft");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("settings.json");
    }
    std::path::PathBuf::from("settings.json")
}

pub fn load_app_settings() -> AppSettings {
    let p = get_settings_path();
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&txt) {
            return s;
        }
    }
    if let Ok(txt) = std::fs::read_to_string("settings.json") {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&txt) {
            return s;
        }
    }
    AppSettings::default()
}

pub fn save_app_settings(s: &AppSettings) {
    let p = get_settings_path();
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(&p, &txt);
        let _ = std::fs::write("settings.json", &txt);
    }
}

static CACHED_DEVICES: Mutex<Option<(Instant, String)>> = Mutex::new(None);
pub static STREAM_CONTROLLER: Mutex<Option<eq_apo_device::stream::StreamController>> = Mutex::new(None);
pub static MONITOR_CONTROLLER: Mutex<Option<eq_apo_device::stream::StreamController>> = Mutex::new(None);
static MONITOR_TARGET: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));
pub static WIZARD_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn invalidate_device_cache() {
    if let Ok(mut lock) = CACHED_DEVICES.lock() {
        *lock = None;
    }
}

pub fn is_monitor_active() -> bool {
    if let Ok(lock) = MONITOR_CONTROLLER.lock() {
        if let Some(ref ctrl) = *lock {
            return ctrl.is_active();
        }
    }
    false
}

pub fn is_stream_active() -> bool {
    if let Ok(lock) = STREAM_CONTROLLER.lock() {
        if let Some(ref ctrl) = *lock {
            return ctrl.is_active();
        }
    }
    false
}

pub fn is_server_alive(port: u16) -> bool {
    if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(300),
    ) {
        use std::io::Write;
        let _ = stream.write_all(b"GET /api/status HTTP/1.0\r\n\r\n");
        return true;
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
                let _ = ctrl.start("fifine".to_string(), "cable".to_string());
                return true;
            }
        }
    }
    false
}

pub fn restart_stream_bridge() {
    println!("[ResonaDSP Stream Bridge] Performing scheduled restart of WASAPI stream bridge...");
    if let Ok(mut lock) = STREAM_CONTROLLER.lock() {
        if let Some(ref mut ctrl) = *lock {
            ctrl.stop();
        }
        let mut ctrl = eq_apo_device::stream::StreamController::new();
        match ctrl.start("fifine".to_string(), "cable".to_string()) {
            Ok(_) => {
                println!("[ResonaDSP Stream Bridge] WASAPI stream bridge successfully re-established!");
                *lock = Some(ctrl);
            }
            Err(e) => {
                eprintln!("[ResonaDSP Stream Bridge] Failed to restart audio bridge: {:?}", e);
            }
        }
    }
}

pub fn start_embedded_server(port: u16) -> Result<(), String> {
    let mut listener = None;
    for attempt in 1..=40 {
        let bind_res = (|| -> Result<TcpListener, std::io::Error> {
            let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
            let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
            let _ = socket.set_reuse_address(true);
            socket.bind(&addr.into())?;
            socket.listen(128)?;
            Ok(socket.into())
        })();

        match bind_res {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => {
                eprintln!("[ResonaDSP Server] Port {} not ready (attempt {}/40: {:?}), retrying in 200ms...", port, attempt, e);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    let listener = listener.ok_or_else(|| format!("Failed to bind to port {} after 40 attempts", port))?;

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
        let mut ctrl = eq_apo_device::stream::StreamController::new_primary();
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

    // Auto-restore saved headphone direct monitoring (sidetone)
    let saved_settings = load_app_settings();
    if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
        *t_lock = (saved_settings.monitor_capture.clone(), saved_settings.monitor_render.clone());
    }
    if saved_settings.monitor_active && !saved_settings.monitor_render.is_empty() {
        let mut cap = saved_settings.monitor_capture.clone();
        if cap.is_empty() {
            for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt", "config.txt"] {
                if let Ok(cfg_txt) = std::fs::read_to_string(path) {
                    for line in cfg_txt.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("Device:") {
                            if let Some(open) = trimmed.find('{') {
                                if let Some(close) = trimmed.find('}') {
                                    if close > open {
                                        cap = trimmed[open..=close].to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if !cap.is_empty() {
                        break;
                    }
                }
            }
        }
        if cap.is_empty() {
            cap = "fifine".to_string();
        }
        let ren = saved_settings.monitor_render.clone();
        let vol = saved_settings.monitor_volume;

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(800));
            let mut ctrl = eq_apo_device::stream::StreamController::new();
            ctrl.set_volume(vol);
            match ctrl.start(cap.clone(), ren.clone()) {
                Ok(_) => {
                    println!("[ResonaDSP Monitor] Restored active direct monitoring ({} -> {}) at volume {:.0}%!", cap, ren, vol * 100.0);
                    if let Ok(mut lock) = MONITOR_CONTROLLER.lock() {
                        *lock = Some(ctrl);
                    }
                    if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                        *t_lock = (cap, ren);
                    }
                }
                Err(e) => {
                    println!("[ResonaDSP Monitor] Direct monitor auto-restore deferred: {:?}", e);
                }
            }
        });
    }

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

    if method == "GET" || method == "HEAD" {
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
                    "speech_prob": 0.0,
                    "rnnoise_enabled": false,
                    "rnnoise_strength": 1.0,
                    "ai_engine_mode": 0,
                    "ai_lsnr_db": -15.0,
                    "spectrum": vec![0.0f32; 32],
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
                            "speech_prob": sm.speech_prob,
                            "rnnoise_enabled": sm.rnnoise_enabled != 0,
                            "rnnoise_strength": sm.rnnoise_strength,
                            "ai_engine_mode": sm.ai_engine_mode,
                            "ai_lsnr_db": sm.ai_lsnr_db,
                            "spectrum": &sm.spectrum_bins[..],
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
            "/api/monitor/status" => {
                let is_active = is_monitor_active();
                let settings = load_app_settings();
                let (cap, ren) = MONITOR_TARGET.lock().map(|t| t.clone()).unwrap_or_default();
                let chosen_cap = if !cap.is_empty() { cap } else { settings.monitor_capture };
                let chosen_ren = if !ren.is_empty() { ren } else if !settings.monitor_render.is_empty() { settings.monitor_render } else { "default".to_string() };
                let vol = MONITOR_CONTROLLER.lock().ok().and_then(|l| l.as_ref().map(|c| c.get_volume())).unwrap_or(settings.monitor_volume);
                let resp = format!(
                    "{{\"status\":\"{}\",\"active\":{},\"capture\":{:?},\"render\":{:?},\"volume\":{:.3}}}",
                    if is_active { "started" } else { "stopped" },
                    is_active,
                    chosen_cap,
                    chosen_ren,
                    vol
                );
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
            "/api/diagnostic-guard" => {
                let stream_active = if let Ok(lock) = STREAM_CONTROLLER.lock() {
                    lock.as_ref().map(|c| c.is_active()).unwrap_or(false)
                } else {
                    false
                };
                let vbcable = crate::vbcable::check_vbcable_installed();
                let devices = crate::devices::enumerate_endpoints("Capture");
                let has_fifine = devices.iter().any(|d| d.name.to_lowercase().contains("fifine") || d.name.to_lowercase().contains("sc3"));
                let has_cable = vbcable.installed;
                let healthy = stream_active && has_cable;

                let resp = serde_json::json!({
                    "status": if healthy { "healthy" } else { "attention" },
                    "stream_active": stream_active,
                    "has_physical_mic": has_fifine,
                    "has_vbcable": has_cable,
                    "reconnect_latency_ms": 150,
                    "discord_guard": {
                        "input_device_target": "CABLE Output (VB-Audio Virtual Cable)",
                        "discord_krisp_recommended": "OFF (Resona AI Active)",
                        "discord_echo_cancellation": "OFF",
                        "discord_noise_suppression": "None",
                    },
                    "tips": [
                        "В Discord выберите 'CABLE Output (VB-Audio Virtual Cable)' в качестве микрофона.",
                        "Отключите встроенный Krisp в Discord — Resona AI уже убирает все шумы с нулевой задержкой.",
                        "Отключите 'Эхоподавление' в Discord для максимальной прозрачности и естественности голоса."
                    ]
                });
                respond_ok(&mut stream, "application/json", resp.to_string().as_bytes());
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
                        *lock = Some(eq_apo_device::stream::StreamController::new_primary());
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
                        let mut new_ctrl = eq_apo_device::stream::StreamController::new_primary();
                        let _ = new_ctrl.start("Microphone".to_string(), "cable".to_string());
                        is_active = true;
                        *lock = Some(new_ctrl);
                    }
                }
                let resp = format!("{{\"status\":\"{}\",\"active\":{},\"stream_active\":{}}}", if is_active { "started" } else { "stopped" }, is_active, is_active);
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/monitor/start" => {
                let mut cap = String::new();
                let mut ren = "default".to_string();

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(c) = val.get("capture").and_then(|s| s.as_str()) {
                        if !c.trim().is_empty() { cap = c.to_string(); }
                    }
                    if let Some(r) = val.get("render").and_then(|s| s.as_str()) {
                        if !r.trim().is_empty() { ren = r.to_string(); }
                    }
                }

                if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                    *t_lock = (cap.clone(), ren.clone());
                }

                if let Ok(mut lock) = MONITOR_CONTROLLER.lock() {
                    if lock.is_none() {
                        *lock = Some(eq_apo_device::stream::StreamController::new());
                    }
                    if let Some(ref mut ctrl) = *lock {
                        let settings = load_app_settings();
                        ctrl.set_volume(settings.monitor_volume);
                        let _ = ctrl.start(cap.clone(), ren.clone());
                    }
                }
                let mut settings = load_app_settings();
                settings.monitor_active = true;
                settings.monitor_capture = cap.clone();
                settings.monitor_render = ren.clone();
                save_app_settings(&settings);

                respond_ok(&mut stream, "application/json", b"{\"status\":\"started\",\"active\":true}");
            }
            "/api/monitor/stop" => {
                if let Ok(mut lock) = MONITOR_CONTROLLER.lock() {
                    if let Some(ref mut ctrl) = *lock {
                        ctrl.stop();
                    }
                }
                if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                    t_lock.0 = String::new();
                }
                let mut settings = load_app_settings();
                settings.monitor_active = false;
                save_app_settings(&settings);

                respond_ok(&mut stream, "application/json", b"{\"status\":\"stopped\",\"active\":false}");
            }
            "/api/monitor/set-device" => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(r) = val.get("render").and_then(|s| s.as_str()) {
                        let trimmed = r.trim().to_string();
                        let mut settings = load_app_settings();
                        settings.monitor_render = trimmed.clone();
                        save_app_settings(&settings);
                        if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                            t_lock.1 = trimmed;
                        }
                    }
                }
                respond_ok(&mut stream, "application/json", b"{\"success\":true}");
            }
            "/api/monitor/toggle" => {
                let mut cap = String::new();
                let mut ren = "default".to_string();
                let mut mode = "toggle".to_string();

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(c) = val.get("capture").and_then(|s| s.as_str()) {
                        let trimmed = c.trim();
                        if !trimmed.is_empty() && trimmed != "default" && !trimmed.starts_with("[object") {
                            cap = trimmed.to_string();
                        }
                    }
                    if let Some(r) = val.get("render").and_then(|s| s.as_str()) {
                        let trimmed = r.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with("[object") {
                            ren = trimmed.to_string();
                        }
                    }
                    if let Some(m) = val.get("mode").and_then(|s| s.as_str()) {
                        mode = m.to_string();
                    }
                }

                if cap.is_empty() {
                    for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt", "config.txt"] {
                        if let Ok(cfg_txt) = std::fs::read_to_string(path) {
                            for line in cfg_txt.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("Device:") {
                                    if let Some(open) = trimmed.find('{') {
                                        if let Some(close) = trimmed.find('}') {
                                            if close > open {
                                                cap = trimmed[open..=close].to_string();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if !cap.is_empty() {
                                break;
                            }
                        }
                    }
                    if cap.is_empty() {
                        cap = "fifine".to_string();
                    }
                }

                let mut is_active = false;
                if let Ok(mut lock) = MONITOR_CONTROLLER.lock() {
                    let should_stop = match mode.as_str() {
                        "start" => false,
                        "stop" => true,
                        _ => {
                            if let Some(ref ctrl) = *lock {
                                if ctrl.is_active() {
                                    let (prev_cap, prev_ren) = MONITOR_TARGET.lock().map(|t| t.clone()).unwrap_or_default();
                                    if cap.is_empty() && (ren == "default" || ren.is_empty()) {
                                        true
                                    } else {
                                        (cap == prev_cap) && (ren == prev_ren)
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    };

                    if should_stop {
                        if let Some(ref mut ctrl) = *lock {
                            ctrl.stop();
                        }
                        if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                            t_lock.0 = String::new();
                        }
                        let mut settings = load_app_settings();
                        settings.monitor_active = false;
                        if !ren.is_empty() {
                            settings.monitor_render = ren.clone();
                        }
                        save_app_settings(&settings);
                    } else {
                        if lock.is_none() {
                            *lock = Some(eq_apo_device::stream::StreamController::new());
                        }
                        if let Some(ref mut ctrl) = *lock {
                            let mut settings = load_app_settings();
                            ctrl.set_volume(settings.monitor_volume);
                            let _ = ctrl.start(cap.clone(), ren.clone());
                            is_active = true;
                            settings.monitor_active = true;
                            settings.monitor_capture = cap.clone();
                            settings.monitor_render = ren.clone();
                            save_app_settings(&settings);
                        }
                        if let Ok(mut t_lock) = MONITOR_TARGET.lock() {
                            *t_lock = (cap.clone(), ren.clone());
                        }
                    }
                }

                let resp = format!(
                    "{{\"status\":\"{}\",\"active\":{},\"capture\":{:?},\"render\":{:?}}}",
                    if is_active { "started" } else { "stopped" },
                    is_active,
                    cap,
                    ren
                );
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            p if p.starts_with("/api/monitor/volume") => {
                let mut vol: f32 = 1.0;
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(v) = val.get("volume").or_else(|| val.get("vol")).and_then(|v| v.as_f64()) {
                        vol = v as f32;
                    }
                }
                if let Some(pos) = raw_path.find('?') {
                    let query = &raw_path[pos + 1..];
                    for param in query.split('&') {
                        if let Some((k, v)) = param.split_once('=') {
                            if k == "vol" || k == "volume" {
                                if let Ok(parsed) = v.parse::<f32>() {
                                    vol = parsed;
                                }
                            }
                        }
                    }
                }
                vol = vol.clamp(0.0, 2.0);
                if let Ok(lock) = MONITOR_CONTROLLER.lock() {
                    if let Some(ref ctrl) = *lock {
                        ctrl.set_volume(vol);
                    }
                }
                let mut settings = load_app_settings();
                settings.monitor_volume = vol;
                save_app_settings(&settings);

                let resp = format!("{{\"success\":true,\"volume\":{}}}", vol);
                respond_ok(&mut stream, "application/json", resp.as_bytes());
            }
            "/api/monitor/status" => {
                let is_active = is_monitor_active();
                let settings = load_app_settings();
                let (cap, ren) = MONITOR_TARGET.lock().map(|t| t.clone()).unwrap_or_default();
                let chosen_cap = if !cap.is_empty() { cap } else { settings.monitor_capture };
                let chosen_ren = if !ren.is_empty() { ren } else if !settings.monitor_render.is_empty() { settings.monitor_render } else { "default".to_string() };
                let vol = MONITOR_CONTROLLER.lock().ok().and_then(|l| l.as_ref().map(|c| c.get_volume())).unwrap_or(settings.monitor_volume);
                let resp = format!(
                    "{{\"status\":\"{}\",\"active\":{},\"capture\":{:?},\"render\":{:?},\"volume\":{:.3}}}",
                    if is_active { "started" } else { "stopped" },
                    is_active,
                    chosen_cap,
                    chosen_ren,
                    vol
                );
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
    // 1. Check high-priority task in Windows Task Scheduler
    let task_check = std::process::Command::new("schtasks")
        .args(["/Query", "/TN", "EqualizerFDCraft_AutoStart"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    if let Ok(ref o) = task_check {
        if o.status.success() {
            return true;
        }
    }

    // 2. Check standard HKCU Run registry key as backup
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

            // 1. Disable Windows Explorer Startup Delay (0 ms instead of default 10-30s delay)
            let _ = std::process::Command::new("reg")
                .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize", "/v", "StartupDelayInMSec", "/t", "REG_DWORD", "/d", "0", "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .status();

            // 2. Register in HKCU Run as backup
            let _ = std::process::Command::new("reg")
                .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "EqualizerFDCraft", "/t", "REG_SZ", "/d", &format!("\"{}\"", exe_str), "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .status();

            // 3. Register high-priority instant Task in Task Scheduler (Priority 4, RunLevel Highest, Delay 0s)
            let xml = format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Equalizer FD CRAFT High-Priority Real-Time Audio Engine AutoStart</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <Delay>PT0S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <DisallowStartOnRemoteAppSession>false</DisallowStartOnRemoteAppSession>
    <UseUnifiedSchedulingEngine>true</UseUnifiedSchedulingEngine>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
    </Exec>
  </Actions>
</Task>"#, exe_str);

            let temp_dir = std::env::temp_dir();
            let xml_path = temp_dir.join("EqualizerFDCraft_Task.xml");
            let utf16: Vec<u16> = xml.encode_utf16().collect();
            let bytes: Vec<u8> = utf16.into_iter().flat_map(|w| w.to_le_bytes()).collect();
            let mut file_bytes = vec![0xFF, 0xFE]; // UTF-16 LE BOM
            file_bytes.extend(bytes);

            if std::fs::write(&xml_path, file_bytes).is_ok() {
                let direct = std::process::Command::new("schtasks")
                    .args(["/Create", "/TN", "EqualizerFDCraft_AutoStart", "/XML", &xml_path.to_string_lossy(), "/F"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();

                if direct.map(|s| s.success()).unwrap_or(false) {
                    return true;
                }

                // If elevated rights needed, trigger elevated via PowerShell
                let pw_script = format!(
                    "Start-Process schtasks.exe -ArgumentList '/Create /TN \"EqualizerFDCraft_AutoStart\" /XML \"{}\" /F' -Verb RunAs -WindowStyle Hidden -Wait",
                    xml_path.to_string_lossy().replace("'", "''")
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &pw_script])
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();
            }

            return true;
        }
        false
    } else {
        // Delete Task Scheduler task
        let direct = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", "EqualizerFDCraft_AutoStart", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if !direct.map(|s| s.success()).unwrap_or(false) {
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", "Start-Process schtasks.exe -ArgumentList '/Delete /TN \"EqualizerFDCraft_AutoStart\" /F' -Verb RunAs -WindowStyle Hidden -Wait"])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
        }

        // Delete HKCU Run key
        let res = std::process::Command::new("reg")
            .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "EqualizerFDCraft", "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        res.map(|s| s.success()).unwrap_or(false)
    }
}
