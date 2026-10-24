//! Windows Audio Endpoint and APO Hooking Manager
//! Full native clean-room implementation of Equalizer FD CRAFT DeviceSelector / Configurator

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn cmd_hidden(prog: &str) -> Command {
    let mut c = Command::new(prog);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

pub const EQUALIZER_APO_PRE_MIX_CLSID: &str = "{EACD2258-FCAC-4FF4-B36D-419E924A6D79}";
pub const EQUALIZER_APO_POST_MIX_CLSID: &str = "{EC1CC9CE-FAED-4822-828A-82A81A6F018F}";

pub const RESONA_APO_PRE_MIX_CLSID: &str = "{B9276901-4965-4DA5-B62D-8D75DFB10001}";
pub const RESONA_APO_POST_MIX_CLSID: &str = "{B9276902-4965-4DA5-B62D-8D75DFB10002}";

pub const PKEY_DEVICE_FRIENDLY_NAME: &str = "{a45c254e-df1c-4efd-8020-67d146a850e0},2";
pub const DEVPKEY_DEVICE_DESC: &str = "{b3f8fa53-0004-438e-9003-51a46e139bfc},6";
pub const PKEY_BUS_TYPE: &str = "{a45c254e-df1c-4efd-8020-67d146a850e0},24";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEndpointInfo {
    pub guid: String,
    pub name: String,
    pub raw_name: String,
    pub hw_desc: String,
    pub connector: String,
    pub endpoint_type: String, // "Render" or "Capture"
    pub has_fx_properties: bool,
    pub apo_installed: bool,
    pub hook_mode: String,
    pub active_sapo_clsid: Option<String>,
}

fn decode_cmd_output(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        if !s.contains('\u{FFFD}') {
            return s.to_string();
        }
    }

    #[cfg(windows)]
    {
        extern "system" {
            fn MultiByteToWideChar(
                code_page: u32,
                flags: u32,
                lp_multi_byte_str: *const u8,
                cb_multi_byte: i32,
                lp_wide_char_str: *mut u16,
                cch_wide_char: i32,
            ) -> i32;
        }

        // Try CP_OEMCP (1)
        let len = unsafe {
            MultiByteToWideChar(1, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0)
        };
        if len > 0 {
            let mut buf = vec![0u16; len as usize];
            unsafe {
                MultiByteToWideChar(1, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), len);
            }
            let s = String::from_utf16_lossy(&buf);
            if !s.contains('\u{FFFD}') {
                return s;
            }
        }

        // Fallback to CP_ACP (0)
        let len_acp = unsafe {
            MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0)
        };
        if len_acp > 0 {
            let mut buf = vec![0u16; len_acp as usize];
            unsafe {
                MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), len_acp);
            }
            return String::from_utf16_lossy(&buf);
        }
    }

    String::from_utf8_lossy(bytes).to_string()
}

pub fn enumerate_endpoints(endpoint_type: &str) -> Vec<AudioEndpointInfo> {
    let mut endpoints = Vec::new();
    let root_path = format!(r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}", endpoint_type);

    let output = cmd_hidden("reg")
        .args(["query", &root_path])
        .output();

    let output_str = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => return endpoints,
    };

    for line in output_str.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(&root_path) {
            continue;
        }

        // Subkey is e.g. HKLM\...\Audio\Render\{guid}
        if let Some(guid) = trimmed.strip_prefix(&format!(r"{}\", root_path)) {
            let guid = guid.trim();
            let state_query = cmd_hidden("reg")
                .args(["query", trimmed, "/v", "DeviceState"])
                .output();

            let is_active = match state_query {
                Ok(st) => {
                    let st_str = String::from_utf8_lossy(&st.stdout);
                    st_str.contains("0x1")
                }
                Err(_) => false,
            };

            if !is_active {
                continue;
            }

            let prop_path = format!(r"{}\Properties", trimmed);
            let prop_query = cmd_hidden("reg")
                .args(["query", &prop_path])
                .output();

            let mut friendly_name = String::new();
            let mut hw_desc = String::new();
            let mut conn_type = String::new();

            if let Ok(p_out) = prop_query {
                let p_str = decode_cmd_output(&p_out.stdout);
                for p_line in p_str.lines() {
                    let p_line_trim = p_line.trim();
                    let parts: Vec<&str> = p_line_trim.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "REG_SZ" {
                        let val = parts[2..].join(" ");
                        if parts[0].eq_ignore_ascii_case(PKEY_DEVICE_FRIENDLY_NAME) {
                            friendly_name = val;
                        } else if parts[0].eq_ignore_ascii_case(DEVPKEY_DEVICE_DESC) {
                            hw_desc = val;
                        } else if parts[0].eq_ignore_ascii_case(PKEY_BUS_TYPE) {
                            conn_type = val;
                        }
                    }
                }
            }

            if friendly_name.is_empty() {
                continue;
            }

            // Determine user-friendly hardware descriptor & connector
            let connector = if conn_type.eq_ignore_ascii_case("USB") || hw_desc.to_lowercase().contains("usb") {
                "USB Audio".to_string()
            } else if hw_desc.to_lowercase().contains("realtek") || conn_type.contains("HDAUDIO") || conn_type.contains("INTE") {
                "Realtek HD Audio".to_string()
            } else if hw_desc.to_lowercase().contains("sonar") || hw_desc.to_lowercase().contains("cable") || hw_desc.to_lowercase().contains("virtual") {
                "Virtual Endpoint".to_string()
            } else if conn_type.contains("BTH") || hw_desc.to_lowercase().contains("bluetooth") {
                "Bluetooth Audio".to_string()
            } else {
                "Audio Endpoint".to_string()
            };

            // Display name: combine friendly name + hardware model (e.g. "Microphone (fifine SC3)")
            let display_name = if !hw_desc.is_empty() && !friendly_name.to_lowercase().contains(&hw_desc.to_lowercase()) {
                format!("{} ({})", friendly_name, hw_desc)
            } else {
                friendly_name.clone()
            };

            let fx_path = format!(r"{}\FxProperties", trimmed);
            let fx_query = cmd_hidden("reg")
                .args(["query", &fx_path])
                .output();

            let mut has_fx = false;
            let mut apo_installed = false;
            let mut hook_mode = "Driver default".to_string();
            let mut active_clsid = None;

            if let Ok(fx_out) = fx_query {
                if fx_out.status.success() {
                    has_fx = true;
                    let fx_str = String::from_utf8_lossy(&fx_out.stdout).to_uppercase();

                    let is_resona = fx_str.contains(&RESONA_APO_PRE_MIX_CLSID.to_uppercase());
                    let is_eq_apo = fx_str.contains(&EQUALIZER_APO_PRE_MIX_CLSID.to_uppercase());

                    if is_resona {
                        apo_installed = true;
                        hook_mode = "Installed (Native ResonaAPO)".to_string();
                        active_clsid = Some(RESONA_APO_PRE_MIX_CLSID.to_string());
                    } else if is_eq_apo {
                        apo_installed = true;
                        hook_mode = "Installed (EqualizerFD-Craft)".to_string();
                        active_clsid = Some(EQUALIZER_APO_PRE_MIX_CLSID.to_string());
                    }
                }
            }

            endpoints.push(AudioEndpointInfo {
                guid: guid.to_string(),
                name: display_name,
                raw_name: friendly_name,
                hw_desc,
                connector,
                endpoint_type: endpoint_type.to_string(),
                has_fx_properties: has_fx,
                apo_installed,
                hook_mode,
                active_sapo_clsid: active_clsid,
            });
        }
    }

    endpoints
}

#[cfg(windows)]
mod winreg_native {
    use std::ffi::c_void;

    pub type HKEY = *mut c_void;
    pub type LSTATUS = i32;

    pub const HKEY_LOCAL_MACHINE: HKEY = 0x80000002u32 as usize as HKEY;
    pub const KEY_SET_VALUE: u32 = 0x0002;
    pub const KEY_QUERY_VALUE: u32 = 0x0001;
    pub const REG_SZ: u32 = 1;
    pub const REG_DWORD: u32 = 4;
    pub const ERROR_SUCCESS: LSTATUS = 0;

    #[link(name = "advapi32")]
    extern "system" {
        pub fn RegOpenKeyExW(
            hKey: HKEY,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut HKEY,
        ) -> LSTATUS;

        pub fn RegSetValueExW(
            hKey: HKEY,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> LSTATUS;

        pub fn RegDeleteValueW(
            hKey: HKEY,
            lpValueName: *const u16,
        ) -> LSTATUS;

        pub fn RegCloseKey(
            hKey: HKEY,
        ) -> LSTATUS;
    }

    pub fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn open_key(sub_key: &str, access: u32) -> Result<HKEY, LSTATUS> {
        let wide = to_wide(sub_key);
        let mut hkey: HKEY = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, wide.as_ptr(), 0, access, &mut hkey)
        };
        if status == ERROR_SUCCESS {
            Ok(hkey)
        } else {
            Err(status)
        }
    }

    pub fn set_str(hkey: HKEY, name: &str, value: &str) -> Result<(), LSTATUS> {
        let name_w = to_wide(name);
        let val_w = to_wide(value);
        let cb_data = (val_w.len() * 2) as u32;
        let status = unsafe {
            RegSetValueExW(
                hkey,
                name_w.as_ptr(),
                0,
                REG_SZ,
                val_w.as_ptr() as *const u8,
                cb_data,
            )
        };
        if status == ERROR_SUCCESS { Ok(()) } else { Err(status) }
    }

    pub fn set_dword(hkey: HKEY, name: &str, value: u32) -> Result<(), LSTATUS> {
        let name_w = to_wide(name);
        let status = unsafe {
            RegSetValueExW(
                hkey,
                name_w.as_ptr(),
                0,
                REG_DWORD,
                &value as *const u32 as *const u8,
                4,
            )
        };
        if status == ERROR_SUCCESS { Ok(()) } else { Err(status) }
    }

    pub fn delete_val(hkey: HKEY, name: &str) -> Result<(), LSTATUS> {
        let name_w = to_wide(name);
        let status = unsafe { RegDeleteValueW(hkey, name_w.as_ptr()) };
        if status == ERROR_SUCCESS { Ok(()) } else { Err(status) }
    }

    pub fn close_key(hkey: HKEY) {
        unsafe {
            let _ = RegCloseKey(hkey);
        }
    }
}

pub fn install_endpoint_hook(guid: &str, endpoint_type: &str, _mode: &str) -> Result<(), String> {
    let pre_clsid = RESONA_APO_PRE_MIX_CLSID;
    let post_clsid = RESONA_APO_POST_MIX_CLSID;

    #[cfg(windows)]
    {
        use winreg_native::*;

        let fx_sub = format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\{}\FxProperties",
            endpoint_type, guid
        );
        let h_fx = open_key(&fx_sub, KEY_SET_VALUE | KEY_QUERY_VALUE)
            .map_err(|e| format!("Не удалось открыть FxProperties (код {}): проверьте права реестра", e))?;

        // 1. LFX (PreMix)
        let _ = set_str(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},1", pre_clsid);
        // 2. GFX (PostMix)
        let _ = set_str(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},2", post_clsid);
        // 3. SFX (Windows 10/11 Stream Effects)
        let _ = set_str(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5", pre_clsid);
        // 4. MFX (Mode / Communications / Discord)
        let _ = set_str(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},6", pre_clsid);
        // 5. EFX (Endpoint Effects)
        let _ = set_str(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7", post_clsid);

        // 6. Enable System Effects flags
        let _ = set_dword(h_fx, "{1da5d803-d01f-450f-9081-68b9264c2f30},2", 0);
        let _ = set_dword(h_fx, "{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5", 0);
        close_key(h_fx);

        // 7. Enable System Effects in Properties
        let prop_sub = format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\{}\Properties",
            endpoint_type, guid
        );
        if let Ok(h_prop) = open_key(&prop_sub, KEY_SET_VALUE | KEY_QUERY_VALUE) {
            let _ = set_dword(h_prop, "{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},7", 0);
            close_key(h_prop);
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

pub fn uninstall_endpoint_hook(guid: &str, endpoint_type: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg_native::*;

        let fx_sub = format!(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\{}\FxProperties",
            endpoint_type, guid
        );
        if let Ok(h_fx) = open_key(&fx_sub, KEY_SET_VALUE | KEY_QUERY_VALUE) {
            let _ = delete_val(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},1");
            let _ = delete_val(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},2");
            let _ = delete_val(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5");
            let _ = delete_val(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},6");
            let _ = delete_val(h_fx, "{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7");
            close_key(h_fx);
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

pub fn restart_audio_engine() {
    let killed = cmd_hidden("taskkill")
        .args(["/F", "/IM", "audiodg.exe"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !killed {
        let pwsh_bin = if cmd_hidden("pwsh").arg("-v").output().is_ok() {
            "pwsh"
        } else {
            "powershell"
        };

        let _ = cmd_hidden(pwsh_bin)
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Start-Process cmd.exe -ArgumentList '/c net stop audiosrv /y && taskkill /F /IM audiodg.exe && net start audiosrv' -Verb RunAs -WindowStyle Hidden",
            ])
            .status();
    } else {
        let _ = cmd_hidden("net").args(["start", "audiosrv"]).status();
    }
}

pub fn create_registry_backup(endpoint: &AudioEndpointInfo, out_dir: &Path) -> Result<String, String> {
    let reg_path = format!(
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\{}\FxProperties",
        endpoint.endpoint_type, endpoint.guid
    );

    let safe_name: String = endpoint.name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let file_name = format!("backup_{}_{}.reg", endpoint.endpoint_type, safe_name);
    let file_path = out_dir.join(file_name);

    let output = cmd_hidden("reg")
        .args(["export", &reg_path, &file_path.to_string_lossy(), "/y"])
        .output()
        .map_err(|e| format!("Failed to run reg export: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("reg export returned error: {}", err));
    }

    Ok(file_path.to_string_lossy().to_string())
}
