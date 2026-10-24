//! ResonaAPO - Native Windows System Audio Processing Object (sAPO)
//! High-performance real-time audio interceptor and DSP processor

pub mod boss_rc505;
pub mod com;
pub mod dsp;
pub mod logger;
pub mod shared_mem;
pub mod types;

pub use com::{DllCanUnloadNow, DllGetClassObject, DllRegisterServer, DllUnregisterServer};
pub use shared_mem::{ResonaSharedMemory, SharedFilter, SharedMemoryHandle};
pub use types::{CLSID_RESONA_POST_MIX, CLSID_RESONA_PRE_MIX};

#[cfg(windows)]
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;

    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            logger::log("DllMain: Attached to process");
        }
        DLL_PROCESS_DETACH => {
            logger::log("DllMain: Detached from process");
        }
        _ => {}
    }
    1
}
