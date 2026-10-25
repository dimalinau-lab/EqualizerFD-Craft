//! VST 2.4 High-Performance Audio Plugin Host for ResonaAPO & Equalizer FD CRAFT
//! Loads 64-bit VST 2.4 DLLs and executes processReplacing in real-time.

use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;

#[repr(C)]
pub struct AEffect {
    pub magic: i32,           // 'VstP' (0x56737450)
    pub _pad0: i32,
    pub dispatcher: extern "C" fn(*mut AEffect, i32, i32, isize, *mut c_void, f32) -> isize,
    pub process: *mut c_void, // Deprecated
    pub set_parameter: extern "C" fn(*mut AEffect, i32, f32),
    pub get_parameter: extern "C" fn(*mut AEffect, i32) -> f32,
    pub num_programs: i32,
    pub num_params: i32,
    pub num_inputs: i32,
    pub num_outputs: i32,
    pub flags: i32,
    pub _pad1: i32,
    pub resvd1: usize,
    pub resvd2: usize,
    pub initial_delay: i32,
    pub real_qualities: i32,
    pub off_qualities: i32,
    pub io_ratio: f32,
    pub object: *mut c_void,
    pub user: *mut c_void,
    pub unique_id: i32,
    pub version: i32,
    pub process_replacing: extern "C" fn(*mut AEffect, *const *const f32, *const *mut f32, i32),
    pub process_double_replacing: *mut c_void,
    pub future: [u8; 56],
}

type HostCallback = extern "C" fn(*mut c_void, i32, i32, isize, *mut c_void, f32) -> isize;
type VstPluginMainFn = extern "C" fn(HostCallback) -> *mut AEffect;

extern "C" fn default_host_callback(
    _effect: *mut c_void,
    opcode: i32,
    _index: i32,
    _value: isize,
    _ptr: *mut c_void,
    _opt: f32,
) -> isize {
    // audioMasterVersion = 1
    if opcode == 1 {
        return 2400;
    }
    0
}

#[cfg(windows)]
extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    fn FreeLibrary(hModule: *mut c_void) -> i32;
}

pub struct VstPluginInstance {
    hmodule: *mut c_void,
    pub effect: *mut AEffect,
    pub library_path: String,
    pub vst_id: String,
    pub enabled: bool,
    pub sample_rate: f32,
    pub num_inputs: usize,
    pub num_outputs: usize,
    temp_in: Vec<Vec<f32>>,
    temp_out: Vec<Vec<f32>>,
    in_ptrs: Vec<*const f32>,
    out_ptrs: Vec<*mut f32>,
}

unsafe impl Send for VstPluginInstance {}
unsafe impl Sync for VstPluginInstance {}

impl VstPluginInstance {
    pub fn load(dll_path: &str, sample_rate: f32, block_size: usize) -> Result<Self, String> {
        #[cfg(not(windows))]
        {
            return Err("VST hosting is only supported on Windows".to_string());
        }

        #[cfg(windows)]
        {
            if !Path::new(dll_path).exists() {
                return Err(format!("VST plugin DLL not found: {}", dll_path));
            }

            let path_w: Vec<u16> = dll_path.encode_utf16().chain(std::iter::once(0)).collect();
            let hmodule = unsafe { LoadLibraryW(path_w.as_ptr()) };
            if hmodule.is_null() {
                return Err(format!("Failed to load VST DLL '{}'", dll_path));
            }

            // Look up entry point: VSTPluginMain or main
            let main_fn_ptr = unsafe {
                let p = GetProcAddress(hmodule, b"VSTPluginMain\0".as_ptr());
                if !p.is_null() {
                    p
                } else {
                    GetProcAddress(hmodule, b"main\0".as_ptr())
                }
            };

            if main_fn_ptr.is_null() {
                unsafe { FreeLibrary(hmodule) };
                return Err(format!("Entry point VSTPluginMain not found in '{}'", dll_path));
            }

            let main_fn: VstPluginMainFn = unsafe { std::mem::transmute(main_fn_ptr) };
            let effect = main_fn(default_host_callback);

            if effect.is_null() {
                unsafe { FreeLibrary(hmodule) };
                return Err(format!("VSTPluginMain returned NULL for '{}'", dll_path));
            }

            unsafe {
                if (*effect).magic != 0x56737450 {
                    // 'VstP'
                    FreeLibrary(hmodule);
                    return Err(format!("Invalid VST magic in '{}'", dll_path));
                }

                let disp = (*effect).dispatcher;
                // effSetSampleRate = 10
                disp(effect, 10, 0, 0, std::ptr::null_mut(), sample_rate);
                // effSetBlockSize = 11
                disp(effect, 11, 0, block_size as isize, std::ptr::null_mut(), 0.0);
                // effMainsChanged = 12 (1 = start)
                disp(effect, 12, 0, 1, std::ptr::null_mut(), 0.0);
            }

            let num_inputs = unsafe { (*effect).num_inputs.max(1) as usize };
            let num_outputs = unsafe { (*effect).num_outputs.max(1) as usize };

            let mut temp_in = Vec::with_capacity(num_inputs);
            for _ in 0..num_inputs {
                temp_in.push(vec![0.0f32; block_size.max(4096)]);
            }

            let mut temp_out = Vec::with_capacity(num_outputs);
            for _ in 0..num_outputs {
                temp_out.push(vec![0.0f32; block_size.max(4096)]);
            }

            let in_ptrs = vec![std::ptr::null(); num_inputs];
            let out_ptrs = vec![std::ptr::null_mut(); num_outputs];

            Ok(Self {
                hmodule,
                effect,
                library_path: dll_path.to_string(),
                vst_id: String::new(),
                enabled: true,
                sample_rate,
                num_inputs,
                num_outputs,
                temp_in,
                temp_out,
                in_ptrs,
                out_ptrs,
            })
        }
    }

    pub fn set_parameter(&mut self, index: i32, value: f32) {
        if !self.effect.is_null() {
            let clamped = value.clamp(0.0, 1.0);
            unsafe {
                ((*self.effect).set_parameter)(self.effect, index, clamped);
            }
        }
    }

    pub fn get_parameter(&self, index: i32) -> f32 {
        if !self.effect.is_null() {
            unsafe { ((*self.effect).get_parameter)(self.effect, index) }
        } else {
            0.0
        }
    }

    pub fn apply_json_params(&mut self, vst_id: &str, params: &serde_json::Value) {
        self.vst_id = vst_id.to_string();
        let map = match params.as_object() {
            Some(m) => m,
            None => return,
        };

        match vst_id.to_lowercase().as_str() {
            "reacomp" => {
                // [0] Thresh (-60..0 dB)
                if let Some(t) = map.get("thresh").and_then(|v| v.as_f64()) {
                    let norm = ((t + 60.0) / 60.0) as f32;
                    self.set_parameter(0, norm);
                }
                // [1] Ratio (1..20:1)
                if let Some(r) = map.get("ratio").and_then(|v| v.as_f64()) {
                    let norm = ((r - 1.0) / 19.0) as f32;
                    self.set_parameter(1, norm);
                }
                // [2] Attack (0..50 ms)
                if let Some(a) = map.get("attack").and_then(|v| v.as_f64()) {
                    let norm = (a / 50.0) as f32;
                    self.set_parameter(2, norm);
                }
                // [3] Release (0..500 ms)
                if let Some(rel) = map.get("release").and_then(|v| v.as_f64()) {
                    let norm = (rel / 500.0) as f32;
                    self.set_parameter(3, norm);
                }
                // [14] Knee (0..12 dB)
                if let Some(k) = map.get("knee").and_then(|v| v.as_f64()) {
                    let norm = (k / 12.0) as f32;
                    self.set_parameter(14, norm);
                }
                // [11] Wet / Makeup (0..24 dB)
                if let Some(m) = map.get("makeup").and_then(|v| v.as_f64()) {
                    let norm = ((m + 60.0) / 84.0) as f32;
                    self.set_parameter(11, norm);
                }
            }
            "reagate" => {
                // [0] Thresh (-60..0 dB)
                if let Some(t) = map.get("thresh").and_then(|v| v.as_f64()) {
                    let norm = ((t + 60.0) / 60.0) as f32;
                    self.set_parameter(0, norm);
                }
                // [1] Attack (0..50 ms)
                if let Some(a) = map.get("attack").and_then(|v| v.as_f64()) {
                    let norm = (a / 50.0) as f32;
                    self.set_parameter(1, norm);
                }
                // [2] Release (10..600 ms)
                if let Some(rel) = map.get("release").and_then(|v| v.as_f64()) {
                    let norm = (rel / 600.0) as f32;
                    self.set_parameter(2, norm);
                }
                // [3] Pre-open (0..20 ms)
                if let Some(po) = map.get("preopen").and_then(|v| v.as_f64()) {
                    let norm = (po / 20.0) as f32;
                    self.set_parameter(3, norm);
                }
                // [4] Hold (0..300 ms)
                if let Some(h) = map.get("hold").and_then(|v| v.as_f64()) {
                    let norm = (h / 300.0) as f32;
                    self.set_parameter(4, norm);
                }
            }
            "readelay" => {
                // [0] Wet
                if let Some(mix) = map.get("mix").and_then(|v| v.as_f64()) {
                    self.set_parameter(0, (mix / 100.0) as f32);
                }
                // [3] Length
                if let Some(t) = map.get("time").and_then(|v| v.as_f64()) {
                    self.set_parameter(3, (t / 1000.0) as f32);
                }
                // [5] Feedback
                if let Some(fb) = map.get("feedback").and_then(|v| v.as_f64()) {
                    self.set_parameter(5, (fb / 100.0) as f32);
                }
                // [6] Lowpass
                if let Some(lp) = map.get("lowpass").and_then(|v| v.as_f64()) {
                    self.set_parameter(6, (lp / 20000.0) as f32);
                }
            }
            "reafir" => {
                // Param 1 = Mode: 0=EQ, 1=Gate, 2=Subtract (Noise Reduction)
                self.set_parameter(1, 2.0 / 4.0); // Subtract mode
                if let Some(red) = map.get("reduction").and_then(|v| v.as_f64()) {
                    let norm = (red / 40.0) as f32;
                    self.set_parameter(4, norm);
                }
                if let Some(fl) = map.get("floor").and_then(|v| v.as_f64()) {
                    let norm = ((fl + 80.0) / 60.0) as f32;
                    self.set_parameter(2, norm);
                }
            }
            "reaxcomp" => {
                // Band 1: Param 2 (Thresh), Param 3 (Ratio), Param 8 (Makeup)
                if let Some(t) = map.get("lowthresh").and_then(|v| v.as_f64()) {
                    let norm = ((t + 60.0) / 60.0) as f32;
                    self.set_parameter(2, norm);
                }
                // Band 2: Param 14 (Thresh), Param 15 (Ratio)
                if let Some(t) = map.get("midthresh").and_then(|v| v.as_f64()) {
                    let norm = ((t + 60.0) / 60.0) as f32;
                    self.set_parameter(14, norm);
                }
                if let Some(m) = map.get("overallmakeup").and_then(|v| v.as_f64()) {
                    let norm = (m / 18.0) as f32;
                    self.set_parameter(8, 1.0);
                    self.set_parameter(1, norm);
                }
            }
            _ => {
                // Generic numeric mapping
                for (k, v) in map {
                    if let Ok(idx) = k.parse::<i32>() {
                        if let Some(val) = v.as_f64() {
                            self.set_parameter(idx, val as f32);
                        }
                    }
                }
            }
        }
    }

    /// Process in-place interleaved float audio buffer
    pub fn process_interleaved(&mut self, buffer: &mut [f32], channels: usize, num_frames: usize) {
        if !self.enabled || self.effect.is_null() || num_frames == 0 {
            return;
        }

        let needed_inputs = self.num_inputs;
        let needed_outputs = self.num_outputs;

        // Ensure temp buffers are sized
        for buf in &mut self.temp_in {
            if buf.len() < num_frames {
                buf.resize(num_frames, 0.0);
            }
        }
        for buf in &mut self.temp_out {
            if buf.len() < num_frames {
                buf.resize(num_frames, 0.0);
            }
        }

        // 1. Deinterleave input channels
        for c in 0..needed_inputs {
            let dst_slice = &mut self.temp_in[c][..num_frames];
            if c < channels {
                for f in 0..num_frames {
                    dst_slice[f] = buffer[f * channels + c];
                }
            } else if channels == 1 && c == 1 {
                // Duplicate mono mic into right channel for stereo VST
                for f in 0..num_frames {
                    dst_slice[f] = buffer[f];
                }
            } else {
                // Clear extra auxiliary/sidechain channels
                dst_slice.fill(0.0);
            }
        }

        // 2. Prepare pointers
        for c in 0..needed_inputs {
            self.in_ptrs[c] = self.temp_in[c].as_ptr();
        }
        for c in 0..needed_outputs {
            self.out_ptrs[c] = self.temp_out[c].as_mut_ptr();
        }

        // 3. Execute VST processReplacing
        unsafe {
            let proc_fn = (*self.effect).process_replacing;
            proc_fn(
                self.effect,
                self.in_ptrs.as_ptr(),
                self.out_ptrs.as_mut_ptr(),
                num_frames as i32,
            );
        }

        // 4. Re-interleave output channels back into buffer
        for f in 0..num_frames {
            for c in 0..channels {
                let out_c = if c < needed_outputs { c } else { 0 };
                buffer[f * channels + c] = self.temp_out[out_c][f];
            }
        }
    }
}

impl Drop for VstPluginInstance {
    fn drop(&mut self) {
        if !self.effect.is_null() {
            unsafe {
                let disp = (*self.effect).dispatcher;
                // effMainsChanged = 12 (0 = stop)
                disp(self.effect, 12, 0, 0, std::ptr::null_mut(), 0.0);
                // effClose = 2
                disp(self.effect, 2, 0, 0, std::ptr::null_mut(), 0.0);
            }
            self.effect = std::ptr::null_mut();
        }

        #[cfg(windows)]
        if !self.hmodule.is_null() {
            unsafe {
                FreeLibrary(self.hmodule);
            }
            self.hmodule = std::ptr::null_mut();
        }
    }
}
