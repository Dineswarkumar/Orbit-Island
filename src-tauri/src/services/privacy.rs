use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyState {
    pub camera_active: bool,
    pub camera_app: Option<String>,
    pub microphone_active: bool,
    pub microphone_app: Option<String>,
    pub is_reliable: bool,
}

impl Default for PrivacyState {
    fn default() -> Self {
        Self {
            camera_active: false,
            camera_app: None,
            microphone_active: false,
            microphone_app: None,
            is_reliable: true,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn inspect_privacy_indicators() -> PrivacyState {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_READ,
    };

    fn clean_app_name(raw: &str) -> String {
        let clean_path = raw.replace('#', "\\");
        let path = std::path::Path::new(&clean_path);
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            match file_stem.to_lowercase().as_str() {
                "chrome" => "Google Chrome".to_string(),
                "msedge" => "Microsoft Edge".to_string(),
                "brave" => "Brave Browser".to_string(),
                "firefox" => "Mozilla Firefox".to_string(),
                "zoom" => "Zoom".to_string(),
                "slack" => "Slack".to_string(),
                "teams" => "Microsoft Teams".to_string(),
                "discord" => "Discord".to_string(),
                "obs64" | "obs" => "OBS Studio".to_string(),
                "code" => "VS Code".to_string(),
                "camera" => "Windows Camera".to_string(),
                other => {
                    // Capitalize first letter of executable
                    let mut chars = other.chars();
                    match chars.next() {
                        None => raw.to_string(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                }
            }
        } else {
            // Check for package names like Microsoft.WindowsCamera_8wekyb3d8bbwe
            if let Some(idx) = raw.find('_') {
                let prefix = &raw[..idx];
                if let Some(dot_idx) = prefix.rfind('.') {
                    prefix[dot_idx + 1..].to_string()
                } else {
                    prefix.to_string()
                }
            } else {
                raw.to_string()
            }
        }
    }

    unsafe fn check_subkeys(root: HKEY, out_apps: &mut Vec<String>) -> bool {
        unsafe {
            let mut index = 0u32;
            let mut key_name_buf = [0u16; 512];
            let mut had_success = false;

            loop {
                let mut name_len = key_name_buf.len() as u32;
                let status = RegEnumKeyExW(
                    root,
                    index,
                    key_name_buf.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if status != ERROR_SUCCESS {
                    break;
                }
                index += 1;
                had_success = true;

                let subkey_name = OsString::from_wide(&key_name_buf[..name_len as usize])
                    .to_string_lossy()
                    .to_string();

                if subkey_name.eq_ignore_ascii_case("NonPackaged") {
                    // Check NonPackaged subkey recursively
                    let wide_np: Vec<u16> = "NonPackaged\0".encode_utf16().collect();
                    let mut np_key = 0 as HKEY;
                    if RegOpenKeyExW(root, wide_np.as_ptr(), 0, KEY_READ, &mut np_key) == ERROR_SUCCESS {
                        check_subkeys(np_key, out_apps);
                        RegCloseKey(np_key);
                    }
                    continue;
                }

                let mut app_key = 0 as HKEY;
                let mut wide_sub: Vec<u16> = subkey_name.encode_utf16().collect();
                wide_sub.push(0);

                if RegOpenKeyExW(root, wide_sub.as_ptr(), 0, KEY_READ, &mut app_key) == ERROR_SUCCESS {
                    let mut start: u64 = 0;
                    let mut stop: u64 = 0;
                    let mut size = std::mem::size_of::<u64>() as u32;

                    let wide_start: Vec<u16> = "LastUsedTimeStart\0".encode_utf16().collect();
                    let wide_stop: Vec<u16> = "LastUsedTimeStop\0".encode_utf16().collect();

                    let res_start = RegQueryValueExW(
                        app_key,
                        wide_start.as_ptr(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut start as *mut u64 as *mut u8,
                        &mut size,
                    );

                    size = std::mem::size_of::<u64>() as u32;
                    let res_stop = RegQueryValueExW(
                        app_key,
                        wide_stop.as_ptr(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut stop as *mut u64 as *mut u8,
                        &mut size,
                    );

                    RegCloseKey(app_key);

                    if res_start == ERROR_SUCCESS && start > 0 {
                        let is_active = if res_stop == ERROR_SUCCESS {
                            stop == 0 || start > stop
                        } else {
                            true
                        };

                        if is_active {
                            let friendly = clean_app_name(&subkey_name);
                            if !out_apps.contains(&friendly) {
                                out_apps.push(friendly);
                            }
                        }
                    }
                }
            }

            had_success
        }
    }

    let mut cam_apps = Vec::new();
    let mut mic_apps = Vec::new();

    let cam_path: Vec<u16> =
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\webcam\0"
            .encode_utf16()
            .collect();
    let mic_path: Vec<u16> =
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\0"
            .encode_utf16()
            .collect();

    let mut cam_key = 0 as HKEY;
    let cam_ok = unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, cam_path.as_ptr(), 0, KEY_READ, &mut cam_key) == ERROR_SUCCESS {
            check_subkeys(cam_key, &mut cam_apps);
            RegCloseKey(cam_key);
            true
        } else {
            false
        }
    };

    let mut mic_key = 0 as HKEY;
    let mic_ok = unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, mic_path.as_ptr(), 0, KEY_READ, &mut mic_key) == ERROR_SUCCESS {
            check_subkeys(mic_key, &mut mic_apps);
            RegCloseKey(mic_key);
            true
        } else {
            false
        }
    };

    PrivacyState {
        camera_active: !cam_apps.is_empty(),
        camera_app: if cam_apps.is_empty() {
            None
        } else {
            Some(cam_apps.join(", "))
        },
        microphone_active: !mic_apps.is_empty(),
        microphone_app: if mic_apps.is_empty() {
            None
        } else {
            Some(mic_apps.join(", "))
        },
        is_reliable: cam_ok || mic_ok,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn inspect_privacy_indicators() -> PrivacyState {
    PrivacyState::default()
}
