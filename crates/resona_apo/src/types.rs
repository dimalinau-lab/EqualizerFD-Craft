//! Windows APO and COM Type Definitions
//! Clean-room implementation matching Windows SDK audioenginebaseapo.h

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;

pub type HRESULT = i32;
pub type HNSTIME = i64;
pub type BOOL = i32;
pub type HANDLE = *mut c_void;

pub const S_OK: HRESULT = 0;
pub const S_FALSE: HRESULT = 1;
pub const E_NOTIMPL: HRESULT = -2147467263; // 0x80004001
pub const E_NOINTERFACE: HRESULT = -2147467262; // 0x80004002
pub const E_POINTER: HRESULT = -2147467261; // 0x80004003
pub const E_FAIL: HRESULT = -2147467259; // 0x80004005
pub const E_INVALIDARG: HRESULT = -2147024809; // 0x80070057
pub const CLASS_E_NOAGGREGATION: HRESULT = -2147221232; // 0x80040110
pub const E_OUTOFMEMORY: HRESULT = -2147024882; // 0x8007000E

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl GUID {
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self { data1, data2, data3, data4 }
    }
}

// ResonaAPO COM CLSIDs
pub const CLSID_RESONA_PRE_MIX: GUID = GUID::new(
    0xB9276901, 0x4965, 0x4DA5, [0xB6, 0x2D, 0x8D, 0x75, 0xDF, 0xB1, 0x00, 0x01]
);
pub const CLSID_RESONA_POST_MIX: GUID = GUID::new(
    0xB9276902, 0x4965, 0x4DA5, [0xB6, 0x2D, 0x8D, 0x75, 0xDF, 0xB1, 0x00, 0x02]
);

// Equalizer FD CRAFT Compatible CLSIDs
pub const CLSID_EQUALIZER_APO_PRE_MIX: GUID = GUID::new(
    0xEACD2258, 0xFCAC, 0x4FF4, [0xB3, 0x6D, 0x41, 0x9E, 0x92, 0x4A, 0x6D, 0x79]
);
pub const CLSID_EQUALIZER_APO_POST_MIX: GUID = GUID::new(
    0xEC1CC9CE, 0xFAED, 0x4822, [0x82, 0x8A, 0x82, 0xA8, 0x1A, 0x6F, 0x01, 0x8F]
);

// Standard COM IIDs
pub const IID_IUNKNOWN: GUID = GUID::new(
    0x00000000, 0x0000, 0x0000, [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]
);
pub const IID_ICLASS_FACTORY: GUID = GUID::new(
    0x00000001, 0x0000, 0x0000, [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]
);

// Windows Core Audio APO IIDs
pub const IID_IAUDIO_PROCESSING_OBJECT: GUID = GUID::new(
    0xFD7F26EE, 0x624D, 0x4576, [0x97, 0xF4, 0x9D, 0x51, 0x2E, 0x9D, 0x65, 0x03]
);
pub const IID_IAUDIO_PROCESSING_OBJECT_RT: GUID = GUID::new(
    0x9E1D6A6D, 0xDDBC, 0x4E95, [0xA4, 0xC7, 0xAD, 0x64, 0xBA, 0xA4, 0xA5, 0xE1]
);
pub const IID_IAUDIO_PROCESSING_OBJECT_CONFIG: GUID = GUID::new(
    0x0E5ED805, 0xAB92, 0x4BED, [0x86, 0x88, 0xA8, 0x36, 0x2C, 0xEF, 0xD4, 0xA1]
);
pub const IID_IAUDIO_SYSTEM_EFFECTS: GUID = GUID::new(
    0x5FAEE82E, 0xE457, 0x4C65, [0xB9, 0x86, 0x2C, 0x04, 0x14, 0x53, 0xA9, 0x55]
);
pub const IID_IAUDIO_SYSTEM_EFFECTS2: GUID = GUID::new(
    0xBAFE9B1D, 0x6784, 0x48F8, [0x98, 0xCF, 0x32, 0xC9, 0x8E, 0xE1, 0xA3, 0xD1]
);

// APO Flags
pub const APO_FLAG_NONE: u32 = 0x00000000;
pub const APO_FLAG_INPLACE: u32 = 0x00000001;
pub const APO_FLAG_SAMPLESPERFRAME_MUST_MATCH: u32 = 0x00000002;
pub const APO_FLAG_FRAMESPERSECOND_MUST_MATCH: u32 = 0x00000004;
pub const APO_FLAG_BITSPERSAMPLE_MUST_MATCH: u32 = 0x00000008;
pub const APO_FLAG_DEFAULT: u32 = APO_FLAG_SAMPLESPERFRAME_MUST_MATCH
    | APO_FLAG_FRAMESPERSECOND_MUST_MATCH
    | APO_FLAG_BITSPERSAMPLE_MUST_MATCH
    | APO_FLAG_INPLACE;

// Buffer Flags
pub const BUFFER_INVALID: u32 = 0;
pub const BUFFER_VALID: u32 = 1;
pub const BUFFER_SILENT: u32 = 2;

#[repr(C)]
pub struct APO_REG_PROPERTIES {
    pub clsid: GUID,
    pub flags: u32,
    pub szFriendlyName: [u16; 256],
    pub szCopyrightInfo: [u16; 256],
    pub u32MajorVersion: u32,
    pub u32MinorVersion: u32,
    pub u32MinInputConnections: u32,
    pub u32MaxInputConnections: u32,
    pub u32MinOutputConnections: u32,
    pub u32MaxOutputConnections: u32,
    pub u32MaxInstances: u32,
    pub u32NumPreProcessConnectionFormats: u32,
    pub ppPreProcessConnectionFormats: *mut *mut c_void,
}

#[repr(C)]
pub struct APO_CONNECTION_DESCRIPTOR {
    pub Type: u32,
    pub pBuffer: usize,
    pub u32MaxFrameCount: u32,
    pub pFormat: *mut c_void,
    pub u32Signature: u32,
}

#[repr(C)]
pub struct APO_CONNECTION_PROPERTY {
    pub pBuffer: usize,
    pub u32ValidFrameCount: u32,
    pub u32BufferFlags: u32,
    pub u32Signature: u32,
}

#[repr(C)]
pub struct WAVEFORMATEX {
    pub wFormatTag: u16,
    pub nChannels: u16,
    pub nSamplesPerSec: u32,
    pub nAvgBytesPerSec: u32,
    pub nBlockAlign: u16,
    pub wBitsPerSample: u16,
    pub cbSize: u16,
}
