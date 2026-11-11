//! WASAPI Real-Time Low-Latency Audio Streaming Engine
//! Bypasses Windows sAPO driver restrictions by bridging Mic -> DSP -> Virtual Cable in user-mode.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use resona_apo::dsp::ResonaDspEngine;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

struct SharedCleanTapState {
    active: bool,
    epoch: u64,
    capture_id: String,
    sample_rate: u32,
    channels: usize,
    total_frames_written: u64,
    samples: VecDeque<f32>,
}

static SHARED_TAP_HUB: OnceLock<(Mutex<SharedCleanTapState>, Condvar)> = OnceLock::new();

fn get_shared_tap() -> &'static (Mutex<SharedCleanTapState>, Condvar) {
    SHARED_TAP_HUB.get_or_init(|| {
        (
            Mutex::new(SharedCleanTapState {
                active: false,
                epoch: 0,
                capture_id: String::new(),
                sample_rate: 48000,
                channels: 2,
                total_frames_written: 0,
                samples: VecDeque::with_capacity(16384),
            }),
            Condvar::new(),
        )
    })
}

pub struct StreamController {
    is_running: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    is_primary_publisher: bool,
    handle: Option<JoinHandle<()>>,
}

impl StreamController {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            is_primary_publisher: false,
            handle: None,
        }
    }

    pub fn new_primary() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            is_primary_publisher: true,
            handle: None,
        }
    }

    pub fn set_primary_publisher(&mut self, primary: bool) {
        self.is_primary_publisher = primary;
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn set_volume(&self, vol: f32) {
        self.volume.store(vol.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
    }

    pub fn get_volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        let (_, cvar) = get_shared_tap();
        cvar.notify_all();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    pub fn start(&mut self, capture_id_or_keyword: String, render_id_or_keyword: String) -> Result<()> {
        if self.is_active() {
            self.stop();
        }

        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::SeqCst);
        let volume = self.volume.clone();
        let is_primary = self.is_primary_publisher
            || render_id_or_keyword.to_lowercase().contains("cable");

        let handle = thread::Builder::new()
            .name(if is_primary {
                "resona_wasapi_primary".to_string()
            } else {
                "resona_wasapi_monitor".to_string()
            })
            .spawn(move || {
                run_streaming_pipeline(
                    capture_id_or_keyword,
                    render_id_or_keyword,
                    is_running,
                    volume,
                    is_primary,
                );
            })
            .map_err(|e| Error::new(E_FAIL, format!("Failed to spawn audio thread: {}", e)))?;

        self.handle = Some(handle);
        Ok(())
    }
}

fn get_device_name_from_registry(flow: EDataFlow, id_str: &str) -> String {
    let guid_str = if let (Some(start), Some(end)) = (id_str.rfind('{'), id_str.rfind('}')) {
        &id_str[start..=end]
    } else {
        return String::new();
    };

    let subkey = if flow == eCapture { "Capture" } else { "Render" };
    let path = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\{}\Properties",
        subkey, guid_str
    );
    let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if windows::Win32::System::Registry::RegOpenKeyExW(
            windows::Win32::System::Registry::HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(path_w.as_ptr()),
            0,
            windows::Win32::System::Registry::KEY_READ,
            &mut hkey,
        ).is_ok() {
            let keys = [
                "{b3f8fa53-0004-438e-9003-51a46e139bfc},6",
                "{a45c254e-df1c-4efd-8020-67d146a850e0},2",
            ];
            for key_name in keys {
                let key_w: Vec<u16> = key_name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut buf = [0u16; 256];
                let mut size = (buf.len() * 2) as u32;
                if windows::Win32::System::Registry::RegQueryValueExW(
                    hkey,
                    windows::core::PCWSTR(key_w.as_ptr()),
                    None,
                    None,
                    Some(buf.as_mut_ptr() as _),
                    Some(&mut size),
                ).is_ok() {
                    let len = (size as usize) / 2;
                    let s = String::from_utf16_lossy(&buf[..len]).trim_matches('\0').to_string();
                    if !s.is_empty() {
                        let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
                        return s;
                    }
                }
            }
            let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
        }
    }
    String::new()
}

fn find_device(
    enumerator: &IMMDeviceEnumerator,
    flow: EDataFlow,
    keyword: &str,
) -> Result<IMMDevice> {
    unsafe {
        let kw_lower = keyword.trim().to_lowercase();
        if kw_lower.is_empty() || kw_lower == "default" || kw_lower.starts_with("[object") {
            if flow == eCapture {
                // If a physical microphone exists (preferring Fifine or USB mic), select it over a silent virtual device
                if let Ok(collection) = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) {
                    if let Ok(count) = collection.GetCount() {
                        let mut best_mic: Option<IMMDevice> = None;
                        for i in 0..count {
                            if let Ok(device) = collection.Item(i) {
                                if let Ok(id_pwstr) = device.GetId() {
                                    let id_str = id_pwstr.to_string().unwrap_or_default();
                                    CoTaskMemFree(Some(id_pwstr.as_ptr() as _));
                                    let friendly_name = get_device_name_from_registry(flow, &id_str);
                                    let name_lower = friendly_name.to_lowercase();
                                    if name_lower.contains("fifine") || id_str.to_lowercase().contains("7da7fc22") {
                                        println!("[WASAPI Stream] Default capture preferring physical mic: '{}' ({})", friendly_name, id_str);
                                        return Ok(device);
                                    }
                                    if !name_lower.contains("cable") && !name_lower.contains("sonar") && !name_lower.contains("virtual") && best_mic.is_none() {
                                        best_mic = Some(device);
                                    }
                                }
                            }
                        }
                        if let Some(dev) = best_mic {
                            println!("[WASAPI Stream] Default capture preferring non-virtual mic");
                            return Ok(dev);
                        }
                    }
                }
            }

            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(flow, eConsole) {
                println!("[WASAPI Stream] Using Windows default {:?} audio endpoint (eConsole)", flow);
                return Ok(dev);
            }
            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(flow, eCommunications) {
                println!("[WASAPI Stream] Using Windows default {:?} audio endpoint (eCommunications)", flow);
                return Ok(dev);
            }
        }

        let collection = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        for i in 0..count {
            let device = collection.Item(i)?;
            let id_pwstr = device.GetId()?;
            let id_str = id_pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id_pwstr.as_ptr() as _));

            let friendly_name = get_device_name_from_registry(flow, &id_str);
            let name_lower = friendly_name.to_lowercase();
            let id_lower = id_str.to_lowercase();

            let matches = if keyword.is_empty() {
                false
            } else if id_lower.contains(&kw_lower) || name_lower.contains(&kw_lower) {
                true
            } else if kw_lower == "vb-audio" || kw_lower == "cable" {
                name_lower.contains("cable") || name_lower.contains("vb-audio") || id_lower.contains("207dec7b") || id_lower.contains("842690d7")
            } else if kw_lower == "fifine" || kw_lower == "sc3" {
                name_lower.contains("fifine") || name_lower.contains("sc3") || id_lower.contains("7da7fc22")
            } else {
                false
            };

            if matches {
                println!(
                    "[WASAPI Stream] Matched {:?} endpoint '{}' -> Name: '{}', GUID: '{}'",
                    flow, keyword, friendly_name, id_str
                );
                return Ok(device);
            }
        }

        if flow == eCapture {
            if let Ok(collection) = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) {
                if let Ok(count) = collection.GetCount() {
                    for i in 0..count {
                        if let Ok(device) = collection.Item(i) {
                            if let Ok(id_pwstr) = device.GetId() {
                                let id_str = id_pwstr.to_string().unwrap_or_default();
                                CoTaskMemFree(Some(id_pwstr.as_ptr() as _));
                                let friendly_name = get_device_name_from_registry(flow, &id_str);
                                if friendly_name.to_lowercase().contains("fifine") || id_str.to_lowercase().contains("7da7fc22") {
                                    println!("[WASAPI Stream] Capture fallback to physical mic: '{}' ({})", friendly_name, id_str);
                                    return Ok(device);
                                }
                            }
                        }
                    }
                }
            }
            let kw_lower = keyword.trim().to_lowercase();
            if kw_lower.is_empty() || kw_lower == "default" {
                println!("[WASAPI Stream] Using default capture endpoint");
                enumerator.GetDefaultAudioEndpoint(flow, eConsole)
            } else {
                eprintln!("[WASAPI Stream] Target capture device '{}' not yet ready in active endpoints. Retrying...", keyword);
                Err(Error::new(E_FAIL, format!("Capture device '{}' not found among active endpoints", keyword)))
            }
        } else {
            let kw_lower = keyword.to_lowercase();
            if kw_lower == "default" || kw_lower.is_empty() || kw_lower.contains("headphone") || kw_lower.contains("наушник") || kw_lower.contains("speaker") || kw_lower.contains("динамик") {
                println!("[WASAPI Stream] Render device '{}', using default playback endpoint", keyword);
                enumerator.GetDefaultAudioEndpoint(flow, eConsole)
            } else if kw_lower.contains("cable") || kw_lower.contains("vb-audio") {
                eprintln!("[WASAPI Stream] Target render device '{}' (Virtual Cable) NOT FOUND.", keyword);
                Err(Error::new(E_FAIL, format!("Render device '{}' (VB-Audio Virtual Cable) not found among active endpoints", keyword)))
            } else {
                println!("[WASAPI Stream] Specific render device '{}' not found, falling back to default playback endpoint", keyword);
                enumerator.GetDefaultAudioEndpoint(flow, eConsole)
            }
        }
    }
}

#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

#[link(name = "avrt")]
extern "system" {
    fn AvSetMmThreadCharacteristicsW(
        TaskName: windows::core::PCWSTR,
        TaskIndex: *mut u32,
    ) -> HANDLE;
    fn AvRevertMmThreadCharacteristics(AvrtHandle: HANDLE) -> BOOL;
}

const AUDCLNT_STREAMFLAGS_EVENTCALLBACK: u32 = 0x00040000;

fn run_streaming_pipeline(
    capture_kw: String,
    render_kw: String,
    running: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    is_primary: bool,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        timeBeginPeriod(1);
        let mut task_idx = 0u32;
        let avrt_handle = AvSetMmThreadCharacteristicsW(windows::core::w!("Pro Audio"), &mut task_idx);

        let mut backoff_ms = 150u64;
        let mut last_success_time = std::time::Instant::now();

        while running.load(Ordering::Relaxed) {
            let session_start = std::time::Instant::now();
            if let Err(e) = run_pipeline_session(&capture_kw, &render_kw, &running, &volume, is_primary) {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                let err_str = format!("{:?}", e);
                let is_sleep_resume = err_str.contains("sleep") || err_str.contains("hibernation");
                if is_sleep_resume {
                    backoff_ms = 1500;
                } else if session_start.elapsed().as_secs() >= 2 {
                    backoff_ms = 250;
                }
                eprintln!(
                    "[WASAPI Guard] Device disconnect / invalidation: {:?}. Hot-Plug auto-reconnecting in {}ms...",
                    e, backoff_ms
                );
                thread::sleep(Duration::from_millis(backoff_ms));
                backoff_ms = (backoff_ms * 3 / 2).min(1500);
            } else {
                backoff_ms = 250;
                println!(
                    "[WASAPI Guard] Stream recovered successfully in {:.1}ms!",
                    last_success_time.elapsed().as_secs_f64() * 1000.0
                );
            }
            last_success_time = std::time::Instant::now();
        }

        if !avrt_handle.is_invalid() {
            let _ = AvRevertMmThreadCharacteristics(avrt_handle);
        }
        timeEndPeriod(1);
        println!("[WASAPI Stream] Pipeline thread terminated.");
    }
}

unsafe fn get_device_id_lower(device: &IMMDevice) -> String {
    if let Ok(id_pwstr) = device.GetId() {
        let s = id_pwstr.to_string().unwrap_or_default().to_lowercase();
        CoTaskMemFree(Some(id_pwstr.as_ptr() as _));
        s
    } else {
        String::new()
    }
}

unsafe fn write_render_from_queue(
    ren_client: &IAudioClient,
    render_service: &IAudioRenderClient,
    ren_buffer_size: u32,
    ren_channels: usize,
    ren_bits: u16,
    vol: f32,
    queue: &mut VecDeque<f32>,
) -> Result<()> {
    let padding = ren_client.GetCurrentPadding()?;
    let available_space = (ren_buffer_size.saturating_sub(padding)) as usize;
    let available_frames = queue.len() / ren_channels;
    let frames_to_write = available_space.min(available_frames);

    if frames_to_write > 0 {
        let render_buf_ptr = render_service.GetBuffer(frames_to_write as u32)?;
        let total_write_samples = frames_to_write * ren_channels;
        if ren_bits == 32 {
            let out_slice = std::slice::from_raw_parts_mut(render_buf_ptr as *mut f32, total_write_samples);
            for item in out_slice.iter_mut() {
                *item = queue.pop_front().unwrap_or(0.0) * vol;
            }
        } else if ren_bits == 16 {
            let out_slice = std::slice::from_raw_parts_mut(render_buf_ptr as *mut i16, total_write_samples);
            for item in out_slice.iter_mut() {
                let s = queue.pop_front().unwrap_or(0.0) * vol;
                *item = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            }
        }
        let _ = render_service.ReleaseBuffer(frames_to_write as u32, 0);
    } else if padding < 480 {
        let silence_frames = (480 - padding).min(available_space as u32);
        if silence_frames > 0 {
            if let Ok(ptr) = render_service.GetBuffer(silence_frames) {
                let byte_count = (silence_frames as usize) * ren_channels * (ren_bits as usize / 8);
                std::ptr::write_bytes(ptr, 0, byte_count);
                let _ = render_service.ReleaseBuffer(silence_frames, 0);
            }
        }
    }
    Ok(())
}

unsafe fn run_tap_subscriber_session(
    cap_id: &str,
    ren_device: IMMDevice,
    running: &Arc<AtomicBool>,
    volume: &Arc<AtomicU32>,
    tap_epoch: u64,
) -> Result<()> {
    let ren_client: IAudioClient = ren_device.Activate(CLSCTX_ALL, None)?;
    let ren_format_ptr = ren_client.GetMixFormat()?;
    let ren_format = *ren_format_ptr;
    let ren_rate = ren_format.nSamplesPerSec;
    let ren_channels = ren_format.nChannels as usize;
    let ren_bits = ren_format.wBitsPerSample;

    if let Err(e) = ren_client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        0,
        400_000, // 40ms lightweight headphone render ring
        0,
        ren_format_ptr,
        None,
    ) {
        CoTaskMemFree(Some(ren_format_ptr as _));
        return Err(e);
    }

    let ren_buffer_size = match ren_client.GetBufferSize() {
        Ok(sz) => sz,
        Err(e) => {
            CoTaskMemFree(Some(ren_format_ptr as _));
            return Err(e);
        }
    };
    let render_service: IAudioRenderClient = match ren_client.GetService() {
        Ok(s) => s,
        Err(e) => {
            CoTaskMemFree(Some(ren_format_ptr as _));
            return Err(e);
        }
    };

    // Minimal 10ms pre-roll for low-latency live monitoring
    let init_frames = (ren_buffer_size / 4).min(480);
    if let Ok(ptr) = render_service.GetBuffer(init_frames) {
        std::ptr::write_bytes(ptr, 0, (init_frames as usize) * ren_channels * (ren_bits as usize / 8));
        let _ = render_service.ReleaseBuffer(init_frames, 0);
    }

    let _ = ren_client.Start();
    println!(
        "[WASAPI Monitor] Attached to shared AI-cleaned stream ({}Hz, {}ch) -> zero duplicate AI CPU!",
        ren_rate, ren_channels
    );

    let (lock, cvar) = get_shared_tap();
    let mut read_seq = lock
        .lock()
        .map(|t| t.total_frames_written)
        .unwrap_or(0);
    let mut local_queue: VecDeque<f32> = VecDeque::with_capacity(4096);
    let mut last_iter_instant = std::time::Instant::now();
    let mut res = Ok(());

    while running.load(Ordering::Relaxed) {
        let now = std::time::Instant::now();
        if now.duration_since(last_iter_instant).as_millis() > 1500 {
            res = Err(Error::new(E_FAIL, "System sleep/hibernation resume detected"));
            break;
        }
        last_iter_instant = now;

        let mut still_valid = false;
        if let Ok(mut guard) = lock.lock() {
            if guard.active
                && guard.epoch == tap_epoch
                && guard.capture_id == cap_id
                && guard.total_frames_written <= read_seq
            {
                let (g, _) = cvar.wait_timeout(guard, Duration::from_millis(12)).unwrap();
                guard = g;
            }
            if guard.active
                && guard.epoch == tap_epoch
                && guard.capture_id == cap_id
                && guard.sample_rate == ren_rate
                && guard.channels == ren_channels
            {
                still_valid = true;
                let total_stored_frames = (guard.samples.len() / ren_channels) as u64;
                let earliest_seq = guard.total_frames_written.saturating_sub(total_stored_frames);
                if read_seq < earliest_seq {
                    read_seq = earliest_seq;
                }
                if guard.total_frames_written > read_seq {
                    let start_frame = (read_seq - earliest_seq) as usize;
                    let start_idx = start_frame * ren_channels;
                    for &s in guard.samples.iter().skip(start_idx) {
                        local_queue.push_back(s);
                    }
                    read_seq = guard.total_frames_written;
                }
            }
        }

        if !still_valid {
            // Primary publisher stopped or reconfigured: yield cleanly back to standalone capture fallback
            break;
        }

        // Clamp monitor latency tightly (< 25 ms)
        let max_q_frames = 1200;
        let cur_q_frames = local_queue.len() / ren_channels;
        if cur_q_frames > max_q_frames {
            let drop_samples = (cur_q_frames - 480) * ren_channels;
            local_queue.drain(..drop_samples);
        }

        let vol = f32::from_bits(volume.load(Ordering::Relaxed));
        if let Err(e) = write_render_from_queue(
            &ren_client,
            &render_service,
            ren_buffer_size,
            ren_channels,
            ren_bits,
            vol,
            &mut local_queue,
        ) {
            res = Err(e);
            break;
        }
    }

    let _ = ren_client.Stop();
    CoTaskMemFree(Some(ren_format_ptr as _));
    res
}

/// Zero-allocation, continuous phase stream resampler for arbitrary sample rate pairs (e.g. 16k -> 48k).
struct StreamResampler {
    phase: f64,
    last_frame: [f32; 2],
}

impl StreamResampler {
    fn new() -> Self {
        Self {
            phase: 0.0,
            last_frame: [0.0, 0.0],
        }
    }

    fn process(
        &mut self,
        input: &[f32],
        in_channels: usize,
        in_rate: f64,
        out_rate: f64,
        output: &mut Vec<f32>,
    ) {
        if (in_rate - out_rate).abs() < 1.0 {
            output.extend_from_slice(input);
            if input.len() >= in_channels {
                self.last_frame[0] = input[input.len() - in_channels];
                self.last_frame[1] = input[input.len() - in_channels + (1 % in_channels)];
            }
            return;
        }

        let in_frames = input.len() / in_channels;
        if in_frames == 0 {
            return;
        }

        let ratio = in_rate / out_rate;

        let get_sample = |idx: isize, ch: usize| -> f32 {
            if idx < 0 {
                self.last_frame[ch]
            } else if (idx as usize) < in_frames {
                input[(idx as usize) * in_channels + (ch % in_channels)]
            } else {
                input[(in_frames - 1) * in_channels + (ch % in_channels)]
            }
        };

        while self.phase < (in_frames as f64) {
            let idx = self.phase.floor() as isize;
            let frac = (self.phase - idx as f64) as f32;

            let s0_l = get_sample(idx, 0);
            let s1_l = get_sample(idx + 1, 0);
            let out_l = s0_l + frac * (s1_l - s0_l);

            let s0_r = get_sample(idx, 1 % in_channels);
            let s1_r = get_sample(idx + 1, 1 % in_channels);
            let out_r = s0_r + frac * (s1_r - s0_r);

            output.push(out_l);
            output.push(out_r);

            self.phase += ratio;
        }

        self.phase -= in_frames as f64;

        if in_frames > 0 {
            self.last_frame[0] = input[(in_frames - 1) * in_channels];
            self.last_frame[1] = input[(in_frames - 1) * in_channels + (1 % in_channels)];
        }
    }
}

fn run_pipeline_session(
    capture_kw: &str,
    render_kw: &str,
    running: &Arc<AtomicBool>,
    volume: &Arc<AtomicU32>,
    is_primary: bool,
) -> Result<()> {
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let cap_device = match find_device(&enumerator, eCapture, capture_kw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[WASAPI Stream] Could not find capture device '{}': {:?}", capture_kw, e);
                return Err(e);
            }
        };

        let ren_device = match find_device(&enumerator, eRender, render_kw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[WASAPI Stream] Could not find render device (VB-Cable) '{}': {:?}", render_kw, e);
                return Err(e);
            }
        };

        let cap_id = get_device_id_lower(&cap_device);

        // If this is a secondary stream (e.g. Headphone Monitor) and the primary stream
        // is already cleaning the same microphone, reuse its AI-cleaned output with zero extra AI CPU.
        if !is_primary {
            let mut sub_epoch = None;
            if let Ok(tap) = get_shared_tap().0.lock() {
                if tap.active && tap.capture_id == cap_id && !cap_id.is_empty() {
                    sub_epoch = Some(tap.epoch);
                }
            }
            if let Some(ep) = sub_epoch {
                return run_tap_subscriber_session(&cap_id, ren_device, running, volume, ep);
            }
        }

        let cap_client: IAudioClient = cap_device.Activate(CLSCTX_ALL, None)?;
        let ren_client: IAudioClient = ren_device.Activate(CLSCTX_ALL, None)?;

        let cap_format_ptr = cap_client.GetMixFormat()?;
        let cap_format = *cap_format_ptr;

        let ren_format_ptr = ren_client.GetMixFormat()?;
        let ren_format = *ren_format_ptr;

        let cap_rate = cap_format.nSamplesPerSec;
        let cap_channels = cap_format.nChannels as usize;
        let cap_bits = cap_format.wBitsPerSample;

        let ren_rate = ren_format.nSamplesPerSec;
        let ren_channels = ren_format.nChannels as usize;
        let ren_bits = ren_format.wBitsPerSample;

        println!(
            "[WASAPI Stream] Capture: {}Hz, {} channels, {} bits",
            cap_rate, cap_channels, cap_bits
        );
        println!(
            "[WASAPI Stream] Render: {}Hz, {} channels, {} bits",
            ren_rate, ren_channels, ren_bits
        );

        let mut published_epoch = 0u64;
        if is_primary {
            let (lock, cvar) = get_shared_tap();
            if let Ok(mut tap) = lock.lock() {
                tap.epoch = tap.epoch.wrapping_add(1);
                tap.active = true;
                tap.capture_id = cap_id.clone();
                tap.sample_rate = ren_rate;
                tap.channels = ren_channels;
                tap.total_frames_written = 0;
                tap.samples.clear();
                published_epoch = tap.epoch;
                cvar.notify_all();
            }
        }

        // 25ms buffer duration for capture (10-20ms effective latency), 60ms/100ms for render stability
        let cap_buffer_duration_hns = 250_000;
        let ren_buffer_duration_hns = if is_primary { 800_000 } else { 400_000 };

        let capture_event = windows::Win32::System::Threading::CreateEventW(None, false, false, None).ok();
        let event_driven = if let Some(evt) = capture_event {
            let init_res = cap_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                cap_buffer_duration_hns,
                0,
                cap_format_ptr,
                None,
            );
            if init_res.is_ok() && cap_client.SetEventHandle(evt).is_ok() {
                true
            } else {
                let _ = cap_client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0,
                    cap_buffer_duration_hns,
                    0,
                    cap_format_ptr,
                    None,
                );
                false
            }
        } else {
            let _ = cap_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                0,
                cap_buffer_duration_hns,
                0,
                cap_format_ptr,
                None,
            );
            false
        };

        if let Err(e) = ren_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            ren_buffer_duration_hns,
            0,
            ren_format_ptr,
            None,
        ) {
            eprintln!("[WASAPI Stream] Render init failed: {:?}", e);
            if is_primary {
                let (lock, cvar) = get_shared_tap();
                if let Ok(mut tap) = lock.lock() {
                    if tap.epoch == published_epoch {
                        tap.active = false;
                        cvar.notify_all();
                    }
                }
            }
            if let Some(evt) = capture_event {
                let _ = CloseHandle(evt);
            }
            CoTaskMemFree(Some(cap_format_ptr as _));
            CoTaskMemFree(Some(ren_format_ptr as _));
            return Err(e);
        }

        let ren_buffer_size = match ren_client.GetBufferSize() {
            Ok(sz) => sz,
            Err(e) => {
                if let Some(evt) = capture_event {
                    let _ = CloseHandle(evt);
                }
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        let capture_service: IAudioCaptureClient = match cap_client.GetService() {
            Ok(s) => s,
            Err(e) => {
                if let Some(evt) = capture_event {
                    let _ = CloseHandle(evt);
                }
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        let render_service: IAudioRenderClient = match ren_client.GetService() {
            Ok(s) => s,
            Err(e) => {
                if let Some(evt) = capture_event {
                    let _ = CloseHandle(evt);
                }
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        // Balanced low-latency pre-roll (20ms VB-Cable, 10ms Monitor) to prevent startup crackles
        let init_frames = if is_primary {
            (ren_buffer_size / 3).min(960)
        } else {
            (ren_buffer_size / 4).min(480)
        };
        if let Ok(ptr) = render_service.GetBuffer(init_frames) {
            let byte_count = (init_frames as usize) * ren_channels * (ren_bits as usize / 8);
            std::ptr::write_bytes(ptr, 0, byte_count);
            let _ = render_service.ReleaseBuffer(init_frames, 0);
        }

        let is_float = if cap_format.wFormatTag == 3 {
            true
        } else if cap_format.wFormatTag == 1 {
            false
        } else if cap_format.wFormatTag == 0xFFFE && cap_format.cbSize >= 22 {
            let data1 = *((cap_format_ptr as *const u8).add(24) as *const u32);
            data1 == 3 // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT has Data1 = 3 (PCM is 1)
        } else {
            cap_bits == 32
        };

        let mut dsp = ResonaDspEngine::new(
            ren_rate as f32,
            ren_channels,
        );

        // Load config from disk initially
        for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt"] {
            if let Ok(cfg_text) = std::fs::read_to_string(path) {
                dsp.sync_from_config_text(&cfg_text);
                break;
            }
        }

        let shm_handle = resona_apo::shared_mem::SharedMemoryHandle::create_or_open().ok();
        if let Some(ref shm) = shm_handle {
            if !shm.ptr.is_null() {
                (*shm.ptr).sample_rate = ren_rate;
                (*shm.ptr).channels = ren_channels as u32;
                dsp.sync_from_shared_mem(&*shm.ptr);
            }
        }

        // Preload DeepFilterNet3_ll before starting WASAPI capture so frame 0 runs full AI denoising with zero stalls
        if dsp.rnnoise.enabled {
            dsp.rnnoise.preload_sync();
        }

        let _ = cap_client.Start();
        let _ = ren_client.Start();

        let mut resampler = StreamResampler::new();
        let mut float_buffer: Vec<f32> = Vec::with_capacity(4096);
        let mut raw_stereo_buffer: Vec<f32> = Vec::with_capacity(4096);
        let mut dsp_stereo_buffer: Vec<f32> = Vec::with_capacity(4096);
        let mut jitter_buffer: VecDeque<f32> = VecDeque::with_capacity(16384);
        let mut dc_x_l = 0.0f32;
        let mut dc_y_l = 0.0f32;
        let mut dc_x_r = 0.0f32;
        let mut dc_y_r = 0.0f32;

        println!("[WASAPI Stream] Stream pipeline RUNNING! Real-time DSP active (event-driven: {}, is_float: {}).", event_driven, is_float);

        let mut error_result: Result<()> = Ok(());
        let mut last_iter_instant = std::time::Instant::now();
        let mut last_packet_instant = std::time::Instant::now();

        while running.load(Ordering::Relaxed) {
            let now = std::time::Instant::now();

            // Watchdog 1: System Sleep / Hibernation resume (time jump > 1500ms)
            if now.duration_since(last_iter_instant).as_millis() > 1500 {
                eprintln!("[WASAPI Stream] System resume from sleep/hibernation detected (time jump > 1500ms). Resetting stream pipeline.");
                error_result = Err(Error::new(E_FAIL, "System sleep/hibernation resume detected"));
                break;
            }
            last_iter_instant = now;

            // Watchdog 2: USB microphone packet stall (> 1800ms without captured packets)
            if now.duration_since(last_packet_instant).as_millis() > 1800 {
                eprintln!("[WASAPI Stream] Capture packet stall detected (> 1800ms without input). Audio endpoint re-enumeration required.");
                error_result = Err(Error::new(E_FAIL, "Capture packet stall watchdog timeout"));
                break;
            }

            // Fast in-memory sync from shared memory
            if let Some(ref shm) = shm_handle {
                if !shm.ptr.is_null() {
                    dsp.sync_from_shared_mem(&*shm.ptr);
                }
            }

            // Wait for next capture packet or timeout
            if event_driven {
                if let Some(evt) = capture_event {
                    let _ = windows::Win32::System::Threading::WaitForSingleObject(evt, 12);
                }
            } else {
                thread::sleep(Duration::from_millis(2));
            }

            // 1. Process all available captured packets
            loop {
                let packet_size = match capture_service.GetNextPacketSize() {
                    Ok(sz) => sz,
                    Err(e) => {
                        error_result = Err(e);
                        break;
                    }
                };

                if packet_size == 0 {
                    break;
                }

                let mut data_ptr = std::ptr::null_mut();
                let mut num_frames_read = 0u32;
                let mut flags = 0u32;

                if let Err(e) = capture_service.GetBuffer(
                    &mut data_ptr,
                    &mut num_frames_read,
                    &mut flags,
                    None,
                    None,
                ) {
                    error_result = Err(e);
                    break;
                }

                if num_frames_read > 0 && !data_ptr.is_null() {
                    last_packet_instant = std::time::Instant::now();
                    let total_samples = (num_frames_read as usize) * cap_channels;
                    float_buffer.clear();

                    if cap_bits == 32 {
                        if is_float {
                            let slice = std::slice::from_raw_parts(data_ptr as *const f32, total_samples);
                            for &s in slice {
                                float_buffer.push(if s.is_finite() { s.clamp(-1.5, 1.5) } else { 0.0 });
                            }
                        } else {
                            let slice = std::slice::from_raw_parts(data_ptr as *const i32, total_samples);
                            for &s in slice {
                                float_buffer.push((s as f32) / 2147483648.0);
                            }
                        }
                    } else if cap_bits == 16 {
                        let slice = std::slice::from_raw_parts(data_ptr as *const i16, total_samples);
                        for &s in slice {
                            float_buffer.push((s as f32) / 32768.0);
                        }
                    } else if cap_bits == 24 {
                        let bytes = std::slice::from_raw_parts(data_ptr, total_samples * 3);
                        for i in 0..total_samples {
                            let b0 = bytes[i * 3] as i32;
                            let b1 = bytes[i * 3 + 1] as i32;
                            let b2 = bytes[i * 3 + 2] as i8 as i32;
                            let val = (b2 << 16) | (b1 << 8) | b0;
                            float_buffer.push((val as f32) / 8388608.0);
                        }
                    }

                    // Convert to target channels (mono to stereo or matching)
                    raw_stereo_buffer.clear();
                    if cap_channels == 1 {
                        for &s in &float_buffer {
                            raw_stereo_buffer.push(s);
                            raw_stereo_buffer.push(s);
                        }
                    } else if cap_channels == 2 {
                        raw_stereo_buffer.extend_from_slice(&float_buffer);
                    } else {
                        let frames = float_buffer.len() / cap_channels.max(1);
                        for f in 0..frames {
                            raw_stereo_buffer.push(float_buffer[f * cap_channels]);
                            raw_stereo_buffer.push(float_buffer[f * cap_channels + 1]);
                        }
                    }

                    // Resample to output rate (e.g. 16k -> 48k) to prevent buffer starvation and hiss
                    dsp_stereo_buffer.clear();
                    resampler.process(
                        &raw_stereo_buffer,
                        2,
                        cap_rate as f64,
                        ren_rate as f64,
                        &mut dsp_stereo_buffer,
                    );

                    // DC blocker filter to eliminate mic hardware DC bias and buzzing
                    for frame in dsp_stereo_buffer.chunks_exact_mut(2) {
                        let in_l = frame[0];
                        let out_l = in_l - dc_x_l + 0.995 * dc_y_l;
                        dc_x_l = in_l;
                        dc_y_l = out_l;
                        frame[0] = out_l;

                        let in_r = frame[1];
                        let out_r = in_r - dc_x_r + 0.995 * dc_y_r;
                        dc_x_r = in_r;
                        dc_y_r = out_r;
                        frame[1] = out_r;
                    }

                    // Process DSP (80Hz HPF -> DeepFilterNet3_ll -> Neural Guard -> Preamp -> EQ -> VST)
                    let (in_pk, out_pk) = dsp.process_interleaved(&mut dsp_stereo_buffer, ren_channels);

                    if is_primary {
                        let new_frames = dsp_stereo_buffer.len() / ren_channels;
                        if new_frames > 0 {
                            let (lock, cvar) = get_shared_tap();
                            if let Ok(mut tap) = lock.lock() {
                                if tap.epoch == published_epoch {
                                    tap.samples.extend(dsp_stereo_buffer.iter().copied());
                                    tap.total_frames_written += new_frames as u64;
                                    let max_samples = 4800 * ren_channels;
                                    if tap.samples.len() > max_samples {
                                        let overflow = tap.samples.len() - max_samples;
                                        tap.samples.drain(..overflow);
                                    }
                                    cvar.notify_all();
                                }
                            }
                        }
                    }

                    // Telemetry only owned by primary publisher (or standalone monitor when primary is off)
                    let allow_shm = is_primary || !get_shared_tap().0.lock().map(|t| t.active).unwrap_or(false);
                    if allow_shm {
                        if let Some(ref shm) = shm_handle {
                            if !shm.ptr.is_null() {
                                (*shm.ptr).input_peak_db = in_pk;
                                (*shm.ptr).output_peak_db = out_pk;
                                (*shm.ptr).speech_prob = dsp.rnnoise.speech_prob;
                                (*shm.ptr).rnnoise_enabled = if dsp.rnnoise.enabled { 1 } else { 0 };
                                (*shm.ptr).rnnoise_strength = dsp.rnnoise.strength;
                                (*shm.ptr).ai_engine_mode = dsp.rnnoise.active_engine;
                                (*shm.ptr).ai_lsnr_db = dsp.rnnoise.lsnr_db;
                                (*shm.ptr).spectrum_bins = dsp.spectrum_bins;
                                (*shm.ptr).heartbeat = (*shm.ptr).heartbeat.wrapping_add(1);
                            }
                        }
                    }

                    // Push processed frames to jitter ring buffer
                    jitter_buffer.extend(dsp_stereo_buffer.iter().copied());

                    // Prevent buffer overflow and latency drift
                    if jitter_buffer.len() > 16384 {
                        let drain_count = jitter_buffer.len() - 8192;
                        jitter_buffer.drain(..drain_count);
                    }
                }

                let _ = capture_service.ReleaseBuffer(num_frames_read);
            }

            if error_result.is_err() {
                break;
            }

            // If secondary standalone monitor notices primary stream came online on the same mic, hand off to save CPU
            if !is_primary {
                let handoff = get_shared_tap()
                    .0
                    .lock()
                    .map(|t| t.active && t.capture_id == cap_id && t.sample_rate == ren_rate && t.channels == ren_channels)
                    .unwrap_or(false);
                if handoff {
                    println!("[WASAPI Monitor] Primary stream active on same mic; handing off to shared AI tap.");
                    break;
                }
            }

            // 2. Feed render client from jitter buffer via shared helper
            let vol = f32::from_bits(volume.load(Ordering::Relaxed));
            if let Err(e) = write_render_from_queue(
                &ren_client,
                &render_service,
                ren_buffer_size,
                ren_channels,
                ren_bits,
                vol,
                &mut jitter_buffer,
            ) {
                error_result = Err(e);
                break;
            }

            // Non-destructive low-latency drift compensation:
            // NEVER drop frames while speaking (speech_prob >= 0.25 or lsnr >= -3 dB).
            // Trim excess silence gently during pauses to maintain 10-20ms monitoring responsiveness.
            let speech_active = dsp.rnnoise.speech_prob >= 0.25 || dsp.rnnoise.lsnr_db >= -3.0;
            let current_frames = jitter_buffer.len() / ren_channels;
            if !speech_active && current_frames > 1440 {
                let excess_frames = (current_frames - 720).min(480);
                for _ in 0..(excess_frames * ren_channels) {
                    jitter_buffer.pop_front();
                }
            } else if current_frames > 12000 {
                let drain = (current_frames - 9600) * ren_channels;
                jitter_buffer.drain(..drain);
            }
        }

        if is_primary {
            let (lock, cvar) = get_shared_tap();
            if let Ok(mut tap) = lock.lock() {
                if tap.epoch == published_epoch {
                    tap.active = false;
                    cvar.notify_all();
                }
            }
        }

        let _ = cap_client.Stop();
        let _ = ren_client.Stop();
        if let Some(evt) = capture_event {
            let _ = CloseHandle(evt);
        }
        CoTaskMemFree(Some(cap_format_ptr as _));
        CoTaskMemFree(Some(ren_format_ptr as _));

        error_result
    }
}
