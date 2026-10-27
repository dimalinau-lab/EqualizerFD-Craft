//! Shared Memory Inter-Process Communication (IPC)
//! Maps Global\ResonaDSP_SharedMemory with low-integrity / everyone access
//! Enables zero-latency parameter updates and live VU-meter feedback between ResonaDSP.exe and audiodg.exe

use std::ffi::c_void;
use std::ptr;

pub const SHARED_MEM_MAGIC: u32 = 0x5245534F; // "RESO"
pub const SHARED_MEM_NAME: &str = "Global\\ResonaDSP_SharedMemory";
pub const LOCAL_MEM_NAME: &str = "Local\\ResonaDSP_SharedMemory";

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SharedFilter {
    pub filter_type: u32, // 0: PK, 1: LS, 2: HS, 3: LP, 4: HP, 5: Notch
    pub enabled: u32,
    pub fc: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for SharedFilter {
    fn default() -> Self {
        Self {
            filter_type: 0,
            enabled: 0,
            fc: 1000.0,
            gain_db: 0.0,
            q: 1.4142,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ResonaSharedMemory {
    pub magic: u32,
    pub version: u32,
    pub enabled: u32,
    pub preamp_gain_db: f32,
    pub filter_count: u32,
    pub filters: [SharedFilter; 16],
    pub geq_bands: [f32; 15],
    pub input_peak_db: f32,
    pub output_peak_db: f32,
    pub heartbeat: u64,
    pub sample_rate: u32,
    pub channels: u32,
    pub speech_prob: f32,
    pub rnnoise_enabled: u32,
    pub rnnoise_strength: f32,
    pub spectrum_bins: [f32; 32],
    pub config_len: u32,
    pub config_bytes: [u8; 8192],
}

impl Default for ResonaSharedMemory {
    fn default() -> Self {
        Self {
            magic: SHARED_MEM_MAGIC,
            version: 1,
            enabled: 1,
            preamp_gain_db: 0.0,
            filter_count: 0,
            filters: [SharedFilter::default(); 16],
            geq_bands: [0.0; 15],
            input_peak_db: -96.0,
            output_peak_db: -96.0,
            heartbeat: 0,
            sample_rate: 48000,
            channels: 2,
            speech_prob: 0.0,
            rnnoise_enabled: 0,
            rnnoise_strength: 1.0,
            spectrum_bins: [0.0; 32],
            config_len: 0,
            config_bytes: [0u8; 8192],
        }
    }
}

#[cfg(windows)]
#[link(name = "advapi32")]
extern "system" {
    fn CreateFileMappingW(
        hFile: *mut c_void,
        lpFileMappingAttributes: *mut c_void,
        flProtect: u32,
        dwMaximumSizeHigh: u32,
        dwMaximumSizeLow: u32,
        lpName: *const u16,
    ) -> *mut c_void;

    fn OpenFileMappingW(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        lpName: *const u16,
    ) -> *mut c_void;

    fn MapViewOfFile(
        hFileMappingObject: *mut c_void,
        dwDesiredAccess: u32,
        dwFileOffsetHigh: u32,
        dwFileOffsetLow: u32,
        dwNumberOfBytesToMap: usize,
    ) -> *mut c_void;

    fn CloseHandle(hObject: *mut c_void) -> i32;

    fn InitializeSecurityDescriptor(
        pSecurityDescriptor: *mut c_void,
        dwRevision: u32,
    ) -> i32;

    fn SetSecurityDescriptorDacl(
        pSecurityDescriptor: *mut c_void,
        bDaclPresent: i32,
        pDacl: *mut c_void,
        bDaclDefaulted: i32,
    ) -> i32;
}

const PAGE_READWRITE: u32 = 0x04;
const FILE_MAP_ALL_ACCESS: u32 = 0xF001F;
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;

pub struct SharedMemoryHandle {
    pub ptr: *mut ResonaSharedMemory,
    pub handle: *mut c_void,
}

impl SharedMemoryHandle {
    /// Creates or opens the shared memory block with NULL DACL (allowing access across services & users)
    pub fn create_or_open() -> Result<Self, String> {
        let size = std::mem::size_of::<ResonaSharedMemory>();

        #[cfg(windows)]
        unsafe {
            // Setup NULL DACL so audiodg (running as LOCAL SERVICE) can access
            let mut sec_desc = vec![0u8; 64];
            InitializeSecurityDescriptor(sec_desc.as_mut_ptr() as *mut c_void, 1);
            SetSecurityDescriptorDacl(sec_desc.as_mut_ptr() as *mut c_void, 1, ptr::null_mut(), 0);

            #[repr(C)]
            struct SECURITY_ATTRIBUTES {
                nLength: u32,
                lpSecurityDescriptor: *mut c_void,
                bInheritHandle: i32,
            }

            let mut sa = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: sec_desc.as_mut_ptr() as *mut c_void,
                bInheritHandle: 0,
            };

            let name_wide: Vec<u16> = SHARED_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();

            let mut h_map = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                &mut sa as *mut _ as *mut c_void,
                PAGE_READWRITE,
                0,
                size as u32,
                name_wide.as_ptr(),
            );

            if h_map.is_null() {
                // Fallback to OpenFileMappingW
                h_map = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, name_wide.as_ptr());
            }

            if h_map.is_null() {
                // Local fallback
                let local_wide: Vec<u16> = LOCAL_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();
                h_map = CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    &mut sa as *mut _ as *mut c_void,
                    PAGE_READWRITE,
                    0,
                    size as u32,
                    local_wide.as_ptr(),
                );
            }

            if h_map.is_null() {
                return Err("Failed to create or open file mapping".to_string());
            }

            let p_view = MapViewOfFile(h_map, FILE_MAP_ALL_ACCESS, 0, 0, size);
            if p_view.is_null() {
                CloseHandle(h_map);
                return Err("Failed to map view of file".to_string());
            }

            let sm = p_view as *mut ResonaSharedMemory;
            if (*sm).magic != SHARED_MEM_MAGIC {
                ptr::write(sm, ResonaSharedMemory::default());
            }

            Ok(Self { ptr: sm, handle: h_map })
        }

        #[cfg(not(windows))]
        {
            Err("Only supported on Windows".to_string())
        }
    }

    /// Open existing shared memory from audiodg or client
    pub fn open_existing() -> Result<Self, String> {
        let size = std::mem::size_of::<ResonaSharedMemory>();

        #[cfg(windows)]
        unsafe {
            let name_wide: Vec<u16> = SHARED_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let mut h_map = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, name_wide.as_ptr());

            if h_map.is_null() {
                let local_wide: Vec<u16> = LOCAL_MEM_NAME.encode_utf16().chain(std::iter::once(0)).collect();
                h_map = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, local_wide.as_ptr());
            }

            if h_map.is_null() {
                return Err("Shared memory not found".to_string());
            }

            let p_view = MapViewOfFile(h_map, FILE_MAP_ALL_ACCESS, 0, 0, size);
            if p_view.is_null() {
                CloseHandle(h_map);
                return Err("Failed to map view of file".to_string());
            }

            Ok(Self { ptr: p_view as *mut ResonaSharedMemory, handle: h_map })
        }

        #[cfg(not(windows))]
        {
            Err("Only supported on Windows".to_string())
        }
    }
}
