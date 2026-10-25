//! WASAPI Real-Time Low-Latency Audio Streaming Engine
//! Bypasses Windows sAPO driver restrictions by bridging Mic -> DSP -> Virtual Cable in user-mode.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use resona_apo::dsp::ResonaDspEngine;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

pub struct StreamController {
    is_running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl StreamController {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
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

        let handle = thread::Builder::new()
            .name("resona_wasapi_router".to_string())
            .spawn(move || {
                run_streaming_pipeline(capture_id_or_keyword, render_id_or_keyword, is_running);
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
        let collection = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;
        let kw_lower = keyword.to_lowercase();

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
            println!("[WASAPI Stream] Capture device '{}' not found by keyword, falling back to default mic", keyword);
            enumerator.GetDefaultAudioEndpoint(flow, eConsole)
        } else {
            eprintln!("[WASAPI Stream] ERROR: Target render device '{}' (Virtual Cable) NOT FOUND. Audio stream will NOT output to speakers/headphones to prevent self-listening loopback.", keyword);
            Err(Error::new(E_FAIL, format!("Render device '{}' (VB-Audio Virtual Cable) not found among active endpoints", keyword)))
        }
    }
}

fn run_streaming_pipeline(
    capture_kw: String,
    render_kw: String,
    running: Arc<AtomicBool>,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        while running.load(Ordering::Relaxed) {
            if let Err(e) = run_pipeline_session(&capture_kw, &render_kw, &running) {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                eprintln!("[WASAPI Stream] Pipeline error / disconnect: {:?}. Auto-reconnecting in 400ms...", e);
                thread::sleep(Duration::from_millis(400));
            }
        }
        println!("[WASAPI Stream] Pipeline thread terminated.");
    }
}

fn run_pipeline_session(
    capture_kw: &str,
    render_kw: &str,
    running: &Arc<AtomicBool>,
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

        // 20ms buffer duration for comfortable headroom preventing underrun
        let buffer_duration_hns = 200_000;

        if let Err(e) = cap_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            buffer_duration_hns,
            0,
            cap_format_ptr,
            None,
        ) {
            eprintln!("[WASAPI Stream] Capture init failed: {:?}", e);
            CoTaskMemFree(Some(cap_format_ptr as _));
            CoTaskMemFree(Some(ren_format_ptr as _));
            return Err(e);
        }

        if let Err(e) = ren_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            buffer_duration_hns,
            0,
            ren_format_ptr,
            None,
        ) {
            eprintln!("[WASAPI Stream] Render init failed: {:?}", e);
            CoTaskMemFree(Some(cap_format_ptr as _));
            CoTaskMemFree(Some(ren_format_ptr as _));
            return Err(e);
        }

        let ren_buffer_size = match ren_client.GetBufferSize() {
            Ok(sz) => sz,
            Err(e) => {
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        let capture_service: IAudioCaptureClient = match cap_client.GetService() {
            Ok(s) => s,
            Err(e) => {
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        let render_service: IAudioRenderClient = match ren_client.GetService() {
            Ok(s) => s,
            Err(e) => {
                CoTaskMemFree(Some(cap_format_ptr as _));
                CoTaskMemFree(Some(ren_format_ptr as _));
                return Err(e);
            }
        };

        let _ = cap_client.Start();
        let _ = ren_client.Start();

        let mut dsp = ResonaDspEngine::new(
            cap_rate as f32,
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
                (*shm.ptr).sample_rate = cap_rate;
                (*shm.ptr).channels = ren_channels as u32;
                dsp.sync_from_shared_mem(&*shm.ptr);
            }
        }

        let mut float_buffer: Vec<f32> = Vec::with_capacity(4096);
        let mut dsp_stereo_buffer: Vec<f32> = Vec::with_capacity(4096);
        let mut jitter_buffer: VecDeque<f32> = VecDeque::with_capacity(16384);

        println!("[WASAPI Stream] Stream pipeline RUNNING! Real-time DSP active.");

        let mut error_result: Result<()> = Ok(());

        while running.load(Ordering::Relaxed) {
            // Fast in-memory sync from shared memory
            if let Some(ref shm) = shm_handle {
                if !shm.ptr.is_null() {
                    dsp.sync_from_shared_mem(&*shm.ptr);
                }
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
                    let total_samples = (num_frames_read as usize) * cap_channels;
                    float_buffer.clear();

                    if cap_bits == 32 {
                        let slice = std::slice::from_raw_parts(data_ptr as *const f32, total_samples);
                        float_buffer.extend_from_slice(slice);
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

                    // Convert to target channels (e.g. mono to stereo)
                    dsp_stereo_buffer.clear();
                    if cap_channels == ren_channels {
                        dsp_stereo_buffer.extend_from_slice(&float_buffer);
                    } else if cap_channels == 1 && ren_channels == 2 {
                        for &s in &float_buffer {
                            dsp_stereo_buffer.push(s);
                            dsp_stereo_buffer.push(s);
                        }
                    } else {
                        dsp_stereo_buffer.extend_from_slice(&float_buffer[..total_samples.min(num_frames_read as usize * ren_channels)]);
                    }

                    // Process DSP (Preamp + Filters + GraphicEQ + Boss RC-505 + VST Plugins)
                    let (in_pk, out_pk) = dsp.process_interleaved(&mut dsp_stereo_buffer, ren_channels);

                    if let Some(ref shm) = shm_handle {
                        if !shm.ptr.is_null() {
                            (*shm.ptr).input_peak_db = in_pk;
                            (*shm.ptr).output_peak_db = out_pk;
                            (*shm.ptr).heartbeat = (*shm.ptr).heartbeat.wrapping_add(1);
                        }
                    }

                    // Push processed frames to jitter ring buffer
                    jitter_buffer.extend(dsp_stereo_buffer.iter().copied());
                }

                let _ = capture_service.ReleaseBuffer(num_frames_read);
            }

            if error_result.is_err() {
                break;
            }

            // 2. Feed render client from jitter buffer without overflow/underflow
            let padding = match ren_client.GetCurrentPadding() {
                Ok(p) => p,
                Err(e) => {
                    error_result = Err(e);
                    break;
                }
            };

            let available_space = (ren_buffer_size.saturating_sub(padding)) as usize;
            let available_frames = jitter_buffer.len() / ren_channels;
            let frames_to_write = available_space.min(available_frames);

            if frames_to_write > 0 {
                let render_buf_ptr = match render_service.GetBuffer(frames_to_write as u32) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        error_result = Err(e);
                        break;
                    }
                };

                let total_write_samples = frames_to_write * ren_channels;
                if ren_bits == 32 {
                    let out_slice = std::slice::from_raw_parts_mut(
                        render_buf_ptr as *mut f32,
                        total_write_samples,
                    );
                    for item in out_slice.iter_mut() {
                        *item = jitter_buffer.pop_front().unwrap_or(0.0);
                    }
                } else if ren_bits == 16 {
                    let out_slice = std::slice::from_raw_parts_mut(
                        render_buf_ptr as *mut i16,
                        total_write_samples,
                    );
                    for item in out_slice.iter_mut() {
                        let s = jitter_buffer.pop_front().unwrap_or(0.0);
                        *item = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    }
                }

                if let Err(e) = render_service.ReleaseBuffer(frames_to_write as u32, 0) {
                    error_result = Err(e);
                    break;
                }
            }

            // Prevent buffer drift from accumulating latency (> 60ms)
            let max_jitter_samples = 3000 * ren_channels;
            if jitter_buffer.len() > max_jitter_samples {
                let excess = jitter_buffer.len() - (1500 * ren_channels);
                jitter_buffer.drain(0..excess);
            }

            thread::sleep(Duration::from_millis(2));
        }

        let _ = cap_client.Stop();
        let _ = ren_client.Stop();
        CoTaskMemFree(Some(cap_format_ptr as _));
        CoTaskMemFree(Some(ren_format_ptr as _));

        error_result
    }
}
