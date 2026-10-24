#![windows_subsystem = "windows"]

pub mod devices;
pub mod server;
pub mod tray;
pub mod updater;
pub mod vbcable;

use eq_apo_core::{AudioPipeline, ConfigFile};
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const APP_PORT: u16 = 49200; // Dedicated audio application port
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "gui" || args[1] == "editor" {
        // Default desktop application mode (double-click or 'editor')
        updater::cleanup_previous_update();
        launch_desktop_program();
        return;
    }

    let command = &args[1];
    match command.as_str() {
        "stream" => {
            let cap_kw = args.get(2).map(|s| s.as_str()).unwrap_or("fifine");
            let ren_kw = args.get(3).map(|s| s.as_str()).unwrap_or("VB-Audio");
            println!("===========================================================");
            println!("  ResonaDSP Real-Time Audio Bridge (WASAPI)");
            println!("  Capture Device Keyword: '{}'", cap_kw);
            println!("  Render Device Keyword:  '{}'", ren_kw);
            println!("===========================================================");
            let mut ctrl = eq_apo_device::stream::StreamController::new();
            if let Err(e) = ctrl.start(cap_kw.to_string(), ren_kw.to_string()) {
                eprintln!("Failed to start stream: {:?}", e);
                return;
            }
            println!("Bridge is RUNNING. Press [ENTER] to stop.");
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            ctrl.stop();
            return;
        }
        "install-hooks" => {
            install_driver_hooks();
            return;
        }
        "devices" => {
            let filter_type = args.get(2).map(|s| s.as_str()).unwrap_or("All");
            let mut all_endpoints = Vec::new();

            if filter_type.eq_ignore_ascii_case("Render") || filter_type.eq_ignore_ascii_case("All") {
                all_endpoints.extend(devices::enumerate_endpoints("Render"));
            }
            if filter_type.eq_ignore_ascii_case("Capture") || filter_type.eq_ignore_ascii_case("All") {
                all_endpoints.extend(devices::enumerate_endpoints("Capture"));
            }

            println!("Found {} Windows Audio Endpoints:", all_endpoints.len());
            println!("{:<10} {:<38} {:<35} {:<12}", "TYPE", "GUID", "DEVICE NAME", "APO STATUS");
            println!("{:-<10} {:-<38} {:-<35} {:-<12}", "", "", "", "");

            for ep in &all_endpoints {
                let status = if ep.apo_installed {
                    "ACTIVE (APO)"
                } else if ep.has_fx_properties {
                    "Driver Native"
                } else {
                    "No FX"
                };
                let display_name: String = ep.name.chars().take(34).collect();
                println!("{:<10} {:<38} {:<35} {:<12}", ep.endpoint_type, ep.guid, display_name, status);
            }
        }
        "backup" => {
            if args.len() < 4 {
                eprintln!("Usage: ResonaDSP backup <guid> <Render|Capture>");
                return;
            }
            let guid = &args[2];
            let ep_type = &args[3];
            let endpoints = devices::enumerate_endpoints(ep_type);
            if let Some(ep) = endpoints.iter().find(|e| e.guid.eq_ignore_ascii_case(guid)) {
                let out_dir = Path::new(r"d:\testik\replica");
                match devices::create_registry_backup(ep, out_dir) {
                    Ok(path) => println!("Backup successfully written to '{}'", path),
                    Err(e) => eprintln!("Backup failed: {}", e),
                }
            } else {
                eprintln!("Device GUID '{}' not found in {}", guid, ep_type);
            }
        }
        "parse" => {
            if args.len() < 3 {
                eprintln!("Error: missing config path");
                return;
            }
            let path = &args[2];
            let content = fs::read_to_string(path).expect("Failed to read config file");
            let config = ConfigFile::parse(&content, 48000.0).expect("Failed to parse config");
            println!("Successfully parsed {} directives from '{}':", config.directives.len(), path);
            for (i, d) in config.directives.iter().enumerate() {
                println!("  [{:02}] {:?}", i + 1, d);
            }
        }
        "response" => {
            if args.len() < 3 {
                eprintln!("Error: missing config path");
                return;
            }
            let path = &args[2];
            let content = fs::read_to_string(path).expect("Failed to read config file");
            let config = ConfigFile::parse(&content, 48000.0).expect("Failed to parse config");
            let pipeline = AudioPipeline::from_config(&config, 48000.0);
            let curve = pipeline.generate_response_curve(200, 10.0, 20000.0);
            let json = serde_json::to_string_pretty(&curve).expect("Failed to serialize response curve");

            if args.len() >= 4 {
                let out_path = &args[3];
                fs::write(out_path, &json).expect("Failed to write output JSON");
                println!("Frequency response (200 points) written to '{}'", out_path);
            } else {
                println!("{}", json);
            }
        }
        "bench" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(r"C:\Program Files\EqualizerAPO\config\config.txt");
            let duration_sec: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);
            let content = fs::read_to_string(path).unwrap_or_else(|_| "Preamp: 0.0 dB\nFilter: ON PK Fc 1000 Hz Gain 4.0 dB Q 1.41\n".to_string());
            let config = ConfigFile::parse(&content, 48000.0).expect("Failed to parse config");
            let mut pipeline = AudioPipeline::from_config(&config, 48000.0);

            let buffer_size = 512;
            let mut buffer = vec![0.5f64; buffer_size * 2]; // interleaved stereo

            println!("Running DSP throughput benchmark for {}s...", duration_sec);
            let start = Instant::now();
            let mut total_samples = 0u64;

            while start.elapsed().as_secs() < duration_sec {
                pipeline.process_interleaved_stereo(&mut buffer);
                total_samples += (buffer_size * 2) as u64;
            }

            let elapsed = start.elapsed().as_secs_f64();
            let samples_per_sec = (total_samples as f64) / elapsed;
            let real_time_factor = samples_per_sec / (48000.0 * 2.0);

            println!("===========================================================");
            println!("Benchmark Results:");
            println!("  Elapsed time:        {:.3} s", elapsed);
            println!("  Total stereo samples: {}", total_samples);
            println!("  Throughput:          {:.2} Million samples/sec", samples_per_sec / 1_000_000.0);
            println!("  Real-time speedup:   {:.1}x real-time (48kHz stereo)", real_time_factor);
            println!("  DSP Latency margin:  CPU usage at real-time is {:.3}%", 100.0 / real_time_factor);
            println!("===========================================================");
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!("Unknown command: '{}'. Run with --help for usage.", other);
        }
    }
}

use windows::core::w;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;

pub fn is_another_instance_running() -> bool {
    unsafe {
        let _ = CreateMutexW(None, false, w!("Global\\EqualizerFDCraftInstance"));
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}

pub fn open_or_focus_editor() {
    let app_url = format!("http://127.0.0.1:{}", APP_PORT);
    launch_browser_window(&app_url);
}

pub fn launch_desktop_program() {
    if is_another_instance_running() {
        open_or_focus_editor();
        return;
    }

    // 1. Start system tray notification area icon & menu
    tray::start_tray_service();

    // 2. Start background embedded server on port 49200
    let _ = server::start_embedded_server(APP_PORT);

    // 3. Launch standalone desktop application window
    open_or_focus_editor();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

fn launch_browser_window(app_url: &str) {
    let edge_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];
    let chrome_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| r"C:\Users\Default\AppData\Local".to_string());
    let profile_dir = format!(r"{}\EqualizerFDCraft\profile", local_app_data);
    let _ = std::fs::create_dir_all(&profile_dir);

    let app_arg = format!("--app={}", app_url);
    let profile_arg = format!("--user-data-dir={}", profile_dir);

    let window_args = [
        app_arg.as_str(),
        profile_arg.as_str(),
        "--window-size=1280,820",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-features=Translate,OptimizationHints,MediaRouter",
    ];

    for path in edge_paths.iter().chain(chrome_paths.iter()) {
        if Path::new(path).exists() {
            let res = Command::new(path)
                .args(&window_args)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            if res.is_ok() {
                return;
            }
        }
    }

    // Fallback: PowerShell hidden launch without CMD flash
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &format!("Start-Process '{}'", app_url)])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn install_driver_hooks() {
    println!("===========================================================");
    println!("  [ResonaDSP Driver Hook Manager]");
    println!("===========================================================");

    let devices = [
        "{7da7fc22-59d9-45e2-b9ca-6c007f3faabe}",
        "{7547e798-7249-44e5-8334-66eca33d19ab}",
    ];

    for guid in devices {
        if let Err(e) = devices::install_endpoint_hook(guid, "Capture", "ALL") {
            eprintln!("[Auto-Healing Guard] Failed to hook {}: {}", guid, e);
        }
    }

    devices::restart_audio_engine();
    println!("[ResonaDSP Driver Hook Manager] Native ResonaAPO.dll hooks installed successfully!");
}

fn print_help() {
    println!("===========================================================");
    println!("Equalizer FD CRAFT Studio & DSP Engine (Windows)");
    println!("===========================================================");
    println!("Usage:");
    println!("  EqualizerFD-Craft.exe                      # Launch desktop application window");
    println!("  EqualizerFD-Craft.exe editor               # Same as above");
    println!("  EqualizerFD-Craft.exe install-hooks        # Install APO driver hooks (elevated)");
    println!("  EqualizerFD-Craft.exe devices [Render|Capture] # Enumerate audio endpoints & APO status");
    println!("  EqualizerFD-Craft.exe backup <guid> <type> # Export .reg backup of device FxProperties");
    println!("  EqualizerFD-Craft.exe bench [config] [sec] # Run real-time DSP throughput benchmark");
    println!("  EqualizerFD-Craft.exe parse <config_path>  # Parse config and dump AST");
    println!("  EqualizerFD-Craft.exe response <config>    # Export frequency response curve JSON");
    println!("===========================================================");
}
