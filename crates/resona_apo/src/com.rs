//! Windows Core Audio APO COM Implementation
//! Supports IAudioProcessingObject, IAudioProcessingObjectRT, IAudioProcessingObjectConfiguration, IAudioSystemEffects2

use crate::dsp::ResonaDspEngine;
use crate::logger;
use crate::shared_mem::{ResonaSharedMemory, SharedMemoryHandle};
use crate::types::*;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

static DLL_REF_COUNT: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
pub struct IAudioProcessingObjectVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Reset: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    pub GetLatency: unsafe extern "system" fn(*mut c_void, *mut HNSTIME) -> HRESULT,
    pub GetRegistrationProperties: unsafe extern "system" fn(*mut c_void, *mut *mut APO_REG_PROPERTIES) -> HRESULT,
    pub Initialize: unsafe extern "system" fn(*mut c_void, u32, *const u8) -> HRESULT,
    pub IsInputFormatSupported: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
    pub IsOutputFormatSupported: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
    pub GetInputChannelCount: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
}

#[repr(C)]
pub struct IAudioProcessingObjectRTVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub APOProcess: unsafe extern "system" fn(*mut c_void, u32, *const *mut APO_CONNECTION_PROPERTY, u32, *const *mut APO_CONNECTION_PROPERTY),
    pub CalcInputFrames: unsafe extern "system" fn(*mut c_void, u32) -> u32,
    pub CalcOutputFrames: unsafe extern "system" fn(*mut c_void, u32) -> u32,
}

#[repr(C)]
pub struct IAudioProcessingObjectConfigVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub LockForProcess: unsafe extern "system" fn(*mut c_void, u32, *const *const APO_CONNECTION_DESCRIPTOR, u32, *const *const APO_CONNECTION_DESCRIPTOR) -> HRESULT,
    pub UnlockForProcess: unsafe extern "system" fn(*mut c_void) -> HRESULT,
}

#[repr(C)]
pub struct IAudioSystemEffects2Vtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub GetEffectsList: unsafe extern "system" fn(*mut c_void, *mut *mut GUID, *mut u32, HANDLE) -> HRESULT,
}

#[repr(C)]
pub struct IClassFactoryVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub CreateInstance: unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub LockServer: unsafe extern "system" fn(*mut c_void, BOOL) -> HRESULT,
}

static APO_VTBL: IAudioProcessingObjectVtbl = IAudioProcessingObjectVtbl {
    QueryInterface: apo_query_interface,
    AddRef: apo_add_ref,
    Release: apo_release,
    Reset: apo_reset,
    GetLatency: apo_get_latency,
    GetRegistrationProperties: apo_get_registration_properties,
    Initialize: apo_initialize,
    IsInputFormatSupported: apo_is_input_format_supported,
    IsOutputFormatSupported: apo_is_output_format_supported,
    GetInputChannelCount: apo_get_input_channel_count,
};

static RT_VTBL: IAudioProcessingObjectRTVtbl = IAudioProcessingObjectRTVtbl {
    QueryInterface: rt_query_interface,
    AddRef: rt_add_ref,
    Release: rt_release,
    APOProcess: rt_apo_process,
    CalcInputFrames: rt_calc_input_frames,
    CalcOutputFrames: rt_calc_output_frames,
};

static CONFIG_VTBL: IAudioProcessingObjectConfigVtbl = IAudioProcessingObjectConfigVtbl {
    QueryInterface: config_query_interface,
    AddRef: config_add_ref,
    Release: config_release,
    LockForProcess: config_lock_for_process,
    UnlockForProcess: config_unlock_for_process,
};

static EFFECTS_VTBL: IAudioSystemEffects2Vtbl = IAudioSystemEffects2Vtbl {
    QueryInterface: effects_query_interface,
    AddRef: effects_add_ref,
    Release: effects_release,
    GetEffectsList: effects_get_effects_list,
};

static FACTORY_VTBL: IClassFactoryVtbl = IClassFactoryVtbl {
    QueryInterface: factory_query_interface,
    AddRef: factory_add_ref,
    Release: factory_release,
    CreateInstance: factory_create_instance,
    LockServer: factory_lock_server,
};

#[repr(C)]
pub struct ResonaApoCom {
    pub lpVtbl_APO: *const IAudioProcessingObjectVtbl,
    pub lpVtbl_RT: *const IAudioProcessingObjectRTVtbl,
    pub lpVtbl_Config: *const IAudioProcessingObjectConfigVtbl,
    pub lpVtbl_Effects: *const IAudioSystemEffects2Vtbl,

    pub ref_count: AtomicU32,
    pub clsid: GUID,
    pub dsp_engine: Mutex<Option<ResonaDspEngine>>,
    pub shared_mem: Mutex<Option<SharedMemoryHandle>>,
    pub sample_rate: AtomicU32,
    pub channels: AtomicU32,
    pub is_locked: AtomicBool,
}

impl ResonaApoCom {
    pub fn new(clsid: GUID) -> *mut Self {
        DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
        let sm = SharedMemoryHandle::open_existing().ok();

        let mut dsp = ResonaDspEngine::new(48000.0, 2);
        for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt"] {
            if let Ok(text) = std::fs::read_to_string(path) {
                dsp.sync_from_config_text(&text);
                break;
            }
        }

        let obj = Box::new(Self {
            lpVtbl_APO: &APO_VTBL,
            lpVtbl_RT: &RT_VTBL,
            lpVtbl_Config: &CONFIG_VTBL,
            lpVtbl_Effects: &EFFECTS_VTBL,

            ref_count: AtomicU32::new(0),
            clsid,
            dsp_engine: Mutex::new(Some(dsp)),
            shared_mem: Mutex::new(sm),
            sample_rate: AtomicU32::new(48000),
            channels: AtomicU32::new(2),
            is_locked: AtomicBool::new(false),
        });

        logger::log(&format!("Created ResonaApoCom instance for CLSID {:?}", clsid));
        Box::into_raw(obj)
    }
}

// === IUnknown & IAudioProcessingObject implementation ===

unsafe extern "system" fn apo_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    let apo = &mut *(this as *mut ResonaApoCom);
    let iid = &*riid;

    if *iid == IID_IUNKNOWN || *iid == IID_IAUDIO_PROCESSING_OBJECT {
        *ppv = &mut apo.lpVtbl_APO as *mut _ as *mut c_void;
    } else if *iid == IID_IAUDIO_PROCESSING_OBJECT_RT {
        *ppv = &mut apo.lpVtbl_RT as *mut _ as *mut c_void;
    } else if *iid == IID_IAUDIO_PROCESSING_OBJECT_CONFIG {
        *ppv = &mut apo.lpVtbl_Config as *mut _ as *mut c_void;
    } else if *iid == IID_IAUDIO_SYSTEM_EFFECTS || *iid == IID_IAUDIO_SYSTEM_EFFECTS2 {
        *ppv = &mut apo.lpVtbl_Effects as *mut _ as *mut c_void;
    } else {
        *ppv = ptr::null_mut();
        return E_NOINTERFACE;
    }

    apo.ref_count.fetch_add(1, Ordering::SeqCst);
    S_OK
}

unsafe extern "system" fn apo_add_ref(this: *mut c_void) -> u32 {
    let apo = &*(this as *mut ResonaApoCom);
    apo.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn apo_release(this: *mut c_void) -> u32 {
    let apo = &*(this as *mut ResonaApoCom);
    let prev = apo.ref_count.fetch_sub(1, Ordering::SeqCst);
    if prev <= 1 {
        logger::log("Destroying ResonaApoCom instance");
        let _ = Box::from_raw(this as *mut ResonaApoCom);
        DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn apo_reset(_this: *mut c_void) -> HRESULT {
    S_OK
}

unsafe extern "system" fn apo_get_latency(_this: *mut c_void, p_time: *mut HNSTIME) -> HRESULT {
    if !p_time.is_null() {
        *p_time = 0; // Zero algorithmic latency for biquad filters
    }
    S_OK
}

#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemAlloc(cb: usize) -> *mut c_void;
}

unsafe extern "system" fn apo_get_registration_properties(
    this: *mut c_void,
    pp_reg_props: *mut *mut APO_REG_PROPERTIES,
) -> HRESULT {
    if pp_reg_props.is_null() {
        return E_POINTER;
    }
    let apo = &*(this as *mut ResonaApoCom);

    // COM CoTaskMemAlloc required for GetRegistrationProperties
    let mem = CoTaskMemAlloc(std::mem::size_of::<APO_REG_PROPERTIES>());
    if mem.is_null() {
        return E_OUTOFMEMORY;
    }
    let p = mem as *mut APO_REG_PROPERTIES;
    std::ptr::write(p, APO_REG_PROPERTIES {
        clsid: apo.clsid,
        flags: APO_FLAG_DEFAULT,
        szFriendlyName: [0u16; 256],
        szCopyrightInfo: [0u16; 256],
        u32MajorVersion: 1,
        u32MinorVersion: 0,
        u32MinInputConnections: 1,
        u32MaxInputConnections: 1,
        u32MinOutputConnections: 1,
        u32MaxOutputConnections: 1,
        u32MaxInstances: 0xFFFFFFFF,
        u32NumPreProcessConnectionFormats: 0,
        ppPreProcessConnectionFormats: ptr::null_mut(),
    });

    let name = "ResonaDSP Audio Processing Object\0".encode_utf16();
    for (i, c) in name.enumerate().take(255) {
        (*p).szFriendlyName[i] = c;
    }

    *pp_reg_props = p;
    S_OK
}

unsafe extern "system" fn apo_initialize(
    _this: *mut c_void,
    _cb_data_size: u32,
    _pby_data: *const u8,
) -> HRESULT {
    logger::log("IAudioProcessingObject::Initialize called");
    S_OK
}

unsafe extern "system" fn apo_is_input_format_supported(
    _this: *mut c_void,
    _p_opposite_format: *mut c_void,
    p_requested_input_format: *mut c_void,
    pp_supported_input_format: *mut *mut c_void,
) -> HRESULT {
    if !pp_supported_input_format.is_null() {
        *pp_supported_input_format = ptr::null_mut();
    }
    if !p_requested_input_format.is_null() {
        S_OK
    } else {
        S_FALSE
    }
}

unsafe extern "system" fn apo_is_output_format_supported(
    _this: *mut c_void,
    _p_opposite_format: *mut c_void,
    p_requested_output_format: *mut c_void,
    pp_supported_output_format: *mut *mut c_void,
) -> HRESULT {
    if !pp_supported_output_format.is_null() {
        *pp_supported_output_format = ptr::null_mut();
    }
    if !p_requested_output_format.is_null() {
        S_OK
    } else {
        S_FALSE
    }
}

unsafe extern "system" fn apo_get_input_channel_count(
    this: *mut c_void,
    pu32_channel_count: *mut u32,
) -> HRESULT {
    if pu32_channel_count.is_null() {
        return E_POINTER;
    }
    let apo = &*(this as *mut ResonaApoCom);
    *pu32_channel_count = apo.channels.load(Ordering::Relaxed);
    S_OK
}

// === IAudioProcessingObjectRT implementation ===

unsafe extern "system" fn rt_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let apo = (this as *mut u8).sub(8) as *mut c_void;
    apo_query_interface(apo, riid, ppv)
}

unsafe extern "system" fn rt_add_ref(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(8) as *mut c_void;
    apo_add_ref(apo)
}

unsafe extern "system" fn rt_release(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(8) as *mut c_void;
    apo_release(apo)
}

static PROCESS_COUNTER: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn rt_apo_process(
    this: *mut c_void,
    u32_num_input_connections: u32,
    pp_input_connections: *const *mut APO_CONNECTION_PROPERTY,
    u32_num_output_connections: u32,
    pp_output_connections: *const *mut APO_CONNECTION_PROPERTY,
) {
    let apo = &mut *((this as *mut u8).sub(8) as *mut ResonaApoCom);

    if u32_num_input_connections == 0 || u32_num_output_connections == 0 {
        return;
    }
    let in_prop_ptr = *pp_input_connections;
    let out_prop_ptr = *pp_output_connections;
    if in_prop_ptr.is_null() || out_prop_ptr.is_null() {
        return;
    }

    let in_prop = &*in_prop_ptr;
    let out_prop = &mut *out_prop_ptr;

    let frames = in_prop.u32ValidFrameCount;
    let in_buf = in_prop.pBuffer as *const f32;
    let out_buf = out_prop.pBuffer as *mut f32;

    if in_buf.is_null() || out_buf.is_null() || frames == 0 {
        return;
    }

    let channels = apo.channels.load(Ordering::Relaxed) as usize;
    let total_samples = (frames as usize) * channels;

    // Copy in to out if distinct pointers
    if in_buf != out_buf {
        ptr::copy_nonoverlapping(in_buf, out_buf, total_samples);
    }

    let out_slice = std::slice::from_raw_parts_mut(out_buf, total_samples);

    // 1. Check shared memory if attached
    let mut synced_from_shm = false;
    if let Ok(mut sm_guard) = apo.shared_mem.try_lock() {
        if sm_guard.is_none() {
            if let Ok(handle) = SharedMemoryHandle::open_existing() {
                *sm_guard = Some(handle);
            }
        }

        if let Some(ref shm) = *sm_guard {
            let sm_ref = &*shm.ptr;
            if sm_ref.enabled != 0 {
                if let Ok(mut dsp_guard) = apo.dsp_engine.try_lock() {
                    if let Some(ref mut dsp) = *dsp_guard {
                        dsp.sync_from_shared_mem(sm_ref);
                        synced_from_shm = true;
                    }
                }
            }
        }
    }

    // 2. Fallback: check config.txt every ~100 buffers if not synced from shared memory
    if !synced_from_shm {
        let cnt = PROCESS_COUNTER.fetch_add(1, Ordering::Relaxed);
        if cnt % 100 == 0 {
            if let Ok(mut dsp_guard) = apo.dsp_engine.try_lock() {
                if let Some(ref mut dsp) = *dsp_guard {
                    for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt"] {
                        if let Ok(text) = std::fs::read_to_string(path) {
                            dsp.sync_from_config_text(&text);
                            break;
                        }
                    }
                }
            }
        }
    }

    // 3. Process DSP filters and soft limiter
    if let Ok(mut dsp_guard) = apo.dsp_engine.try_lock() {
        if let Some(ref mut dsp) = *dsp_guard {
            let (in_pk, out_pk) = dsp.process_interleaved(out_slice, channels);

            // Update shared memory live feedback if attached
            if let Ok(sm_guard) = apo.shared_mem.try_lock() {
                if let Some(ref shm) = *sm_guard {
                    let sm_mut = &mut *shm.ptr;
                    sm_mut.input_peak_db = in_pk;
                    sm_mut.output_peak_db = out_pk;
                    sm_mut.speech_prob = dsp.rnnoise.speech_prob;
                    sm_mut.rnnoise_enabled = if dsp.rnnoise.enabled { 1 } else { 0 };
                    sm_mut.rnnoise_strength = dsp.rnnoise.strength;
                    sm_mut.spectrum_bins = dsp.spectrum_bins;
                    sm_mut.heartbeat = sm_mut.heartbeat.wrapping_add(1);
                }
            }
        }
    }

    out_prop.u32ValidFrameCount = frames;
    out_prop.u32BufferFlags = in_prop.u32BufferFlags;
}

unsafe extern "system" fn rt_calc_input_frames(_this: *mut c_void, u32_output_frame_count: u32) -> u32 {
    u32_output_frame_count
}

unsafe extern "system" fn rt_calc_output_frames(_this: *mut c_void, u32_input_frame_count: u32) -> u32 {
    u32_input_frame_count
}

// === IAudioProcessingObjectConfiguration implementation ===

unsafe extern "system" fn config_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let apo = (this as *mut u8).sub(16) as *mut c_void;
    apo_query_interface(apo, riid, ppv)
}

unsafe extern "system" fn config_add_ref(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(16) as *mut c_void;
    apo_add_ref(apo)
}

unsafe extern "system" fn config_release(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(16) as *mut c_void;
    apo_release(apo)
}

unsafe extern "system" fn config_lock_for_process(
    this: *mut c_void,
    u32_num_input_connections: u32,
    pp_input_connection_descriptors: *const *const APO_CONNECTION_DESCRIPTOR,
    _u32_num_output_connections: u32,
    _pp_output_connection_descriptors: *const *const APO_CONNECTION_DESCRIPTOR,
) -> HRESULT {
    let apo = &mut *((this as *mut u8).sub(16) as *mut ResonaApoCom);
    let mut sample_rate = 48000.0f32;
    let mut channels = 2usize;

    if u32_num_input_connections > 0 && !pp_input_connection_descriptors.is_null() {
        let desc_ptr = *pp_input_connection_descriptors;
        if !desc_ptr.is_null() {
            let desc = &*desc_ptr;
            if !desc.pFormat.is_null() {
                // pFormat is IAudioMediaType*
                let vtbl_ptr = *(desc.pFormat as *const *const usize);
                if !vtbl_ptr.is_null() {
                    let get_format_fn: unsafe extern "system" fn(*mut c_void) -> *const WAVEFORMATEX =
                        std::mem::transmute(*vtbl_ptr.add(5));
                    let wf = get_format_fn(desc.pFormat);
                    if !wf.is_null() {
                        let w = &*wf;
                        if w.nSamplesPerSec > 0 {
                            sample_rate = w.nSamplesPerSec as f32;
                        }
                        if w.nChannels > 0 {
                            channels = w.nChannels as usize;
                        }
                    }
                }
            }
        }
    }

    apo.sample_rate.store(sample_rate as u32, Ordering::SeqCst);
    apo.channels.store(channels as u32, Ordering::SeqCst);
    apo.is_locked.store(true, Ordering::SeqCst);

    if let Ok(mut dsp_guard) = apo.dsp_engine.lock() {
        let mut dsp = ResonaDspEngine::new(sample_rate, channels);
        for path in &[r"d:\testik\config.txt", r"C:\Program Files\EqualizerAPO\config\config.txt"] {
            if let Ok(text) = std::fs::read_to_string(path) {
                dsp.sync_from_config_text(&text);
                break;
            }
        }
        *dsp_guard = Some(dsp);
    }

    logger::log(&format!(
        "LockForProcess: SampleRate={:.0} Hz, Channels={}",
        sample_rate, channels
    ));

    S_OK
}

unsafe extern "system" fn config_unlock_for_process(this: *mut c_void) -> HRESULT {
    let apo = &mut *((this as *mut u8).sub(16) as *mut ResonaApoCom);
    apo.is_locked.store(false, Ordering::SeqCst);
    logger::log("UnlockForProcess called");
    S_OK
}

// === IAudioSystemEffects2 implementation ===

unsafe extern "system" fn effects_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let apo = (this as *mut u8).sub(24) as *mut c_void;
    apo_query_interface(apo, riid, ppv)
}

unsafe extern "system" fn effects_add_ref(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(24) as *mut c_void;
    apo_add_ref(apo)
}

unsafe extern "system" fn effects_release(this: *mut c_void) -> u32 {
    let apo = (this as *mut u8).sub(24) as *mut c_void;
    apo_release(apo)
}

unsafe extern "system" fn effects_get_effects_list(
    _this: *mut c_void,
    pp_effects_ids: *mut *mut GUID,
    pc_effects: *mut u32,
    _event: HANDLE,
) -> HRESULT {
    if !pc_effects.is_null() {
        *pc_effects = 0;
    }
    if !pp_effects_ids.is_null() {
        *pp_effects_ids = ptr::null_mut();
    }
    S_OK
}

// === IClassFactory implementation ===

#[repr(C)]
pub struct ResonaClassFactory {
    pub lpVtbl: *const IClassFactoryVtbl,
    pub ref_count: AtomicU32,
    pub clsid: GUID,
}

unsafe extern "system" fn factory_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    let fact = &mut *(this as *mut ResonaClassFactory);
    let iid = &*riid;

    if *iid == IID_IUNKNOWN || *iid == IID_ICLASS_FACTORY {
        *ppv = this;
        fact.ref_count.fetch_add(1, Ordering::SeqCst);
        S_OK
    } else {
        *ppv = ptr::null_mut();
        E_NOINTERFACE
    }
}

unsafe extern "system" fn factory_add_ref(this: *mut c_void) -> u32 {
    let fact = &*(this as *mut ResonaClassFactory);
    fact.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn factory_release(this: *mut c_void) -> u32 {
    let fact = &*(this as *mut ResonaClassFactory);
    let prev = fact.ref_count.fetch_sub(1, Ordering::SeqCst);
    if prev <= 1 {
        let _ = Box::from_raw(this as *mut ResonaClassFactory);
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn factory_create_instance(
    this: *mut c_void,
    p_unk_outer: *mut c_void,
    riid: *const GUID,
    ppv_object: *mut *mut c_void,
) -> HRESULT {
    if !p_unk_outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    if riid.is_null() || ppv_object.is_null() {
        return E_POINTER;
    }

    let fact = &*(this as *mut ResonaClassFactory);
    let apo = ResonaApoCom::new(fact.clsid);
    apo_query_interface(apo as *mut c_void, riid, ppv_object)
}

unsafe extern "system" fn factory_lock_server(_this: *mut c_void, _f_lock: BOOL) -> HRESULT {
    S_OK
}

// === Standard COM DLL Entry Points ===

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    *ppv = ptr::null_mut();

    let clsid = &*rclsid;
    if *clsid != CLSID_RESONA_PRE_MIX
        && *clsid != CLSID_RESONA_POST_MIX
        && *clsid != CLSID_EQUALIZER_APO_PRE_MIX
        && *clsid != CLSID_EQUALIZER_APO_POST_MIX
    {
        return CLASS_E_NOAGGREGATION;
    }

    let factory = Box::new(ResonaClassFactory {
        lpVtbl: &FACTORY_VTBL,
        ref_count: AtomicU32::new(0),
        clsid: *clsid,
    });

    factory_query_interface(Box::into_raw(factory) as *mut c_void, riid, ppv)
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    S_OK
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK
}
