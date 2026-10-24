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
static MF_CAMERA_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os = "windows")]
static MF_LAST_ACTIVE_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(target_os = "windows")]
static MF_CAMERA_APP: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
#[cfg(target_os = "windows")]
struct MonitorWrapper(Option<windows::Win32::Media::MediaFoundation::IMFSensorActivityMonitor>);
#[cfg(target_os = "windows")]
unsafe impl Send for MonitorWrapper {}
#[cfg(target_os = "windows")]
unsafe impl Sync for MonitorWrapper {}

#[cfg(target_os = "windows")]
static MONITOR_HANDLE: std::sync::Mutex<MonitorWrapper> = std::sync::Mutex::new(MonitorWrapper(None));

#[cfg(target_os = "windows")]
use windows_core::implement;
#[cfg(target_os = "windows")]
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl,
    MFCreateSensorActivityMonitor, MFStartup, MF_VERSION,
};

#[cfg(target_os = "windows")]
pub fn clean_app_name(raw: &str) -> String {
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
            "teams" | "msteams" => "Microsoft Teams".to_string(),
            "discord" => "Discord".to_string(),
            "obs64" | "obs" => "OBS Studio".to_string(),
            "code" => "VS Code".to_string(),
            "camera" | "windowscamera" => "Windows Camera".to_string(),
            "whatsapp" | "whatsappdesktop" | "whatsapp.root" => "WhatsApp".to_string(),
            "screensketch" | "snippingtool" => "Snipping Tool".to_string(),
            "windowssoundrecorder" | "soundrecorder" => "Sound Recorder".to_string(),
            other => {
                let mut chars = other.chars();
                match chars.next() {
                    None => raw.to_string(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        }
    } else {
        if let Some(idx) = raw.find('_') {
            let prefix = &raw[..idx];
            let token = if let Some(dot_idx) = prefix.rfind('.') {
                &prefix[dot_idx + 1..]
            } else {
                prefix
            };
            match token.to_lowercase().as_str() {
                "whatsappdesktop" | "whatsapp" => "WhatsApp".to_string(),
                "windowscamera" => "Windows Camera".to_string(),
                "windowssoundrecorder" => "Sound Recorder".to_string(),
                "screensketch" => "Snipping Tool".to_string(),
                "msteams" => "Microsoft Teams".to_string(),
                "immersivecontrolpanel" => "Windows Settings".to_string(),
                _ => token.to_string(),
            }
        } else {
            raw.to_string()
        }
    }
}

#[cfg(target_os = "windows")]
fn get_app_name_from_pid(pid: u32) -> String {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !h_proc.is_null() {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(h_proc, 0, buf.as_mut_ptr(), &mut len) != 0 {
                CloseHandle(h_proc);
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                return clean_app_name(&full);
            }
            CloseHandle(h_proc);
        }
    }
    String::new()
}

#[cfg(target_os = "windows")]
#[implement(IMFSensorActivitiesReportCallback)]
struct OrbitSensorActivityCallback;

#[cfg(target_os = "windows")]
impl IMFSensorActivitiesReportCallback_Impl for OrbitSensorActivityCallback_Impl {
    fn OnActivitiesReport(
        &self,
        sensoractivitiesreport: windows_core::Ref<'_, IMFSensorActivitiesReport>,
    ) -> windows_core::Result<()> {
        if let Ok(report) = sensoractivitiesreport.ok() {
            let mut any_streaming = false;
            let mut active_apps: Vec<String> = Vec::new();

            let count = unsafe { report.GetCount().unwrap_or(0) };
            for i in 0..count {
                if let Ok(act) = unsafe { report.GetActivityReport(i) } {
                    let mut name_buf = [0u16; 128];
                    let mut written = 0u32;
                    let friendly_sensor = if unsafe {
                        act.GetFriendlyName(&mut name_buf, &mut written)
                    }
                    .is_ok() && written > 0
                    {
                        let len = (written as usize).min(name_buf.len());
                        String::from_utf16_lossy(&name_buf[..len]).trim_end_matches('\0').to_string()
                    } else {
                        String::new()
                    };

                    let proc_count = unsafe { act.GetProcessCount().unwrap_or(0) };
                    for j in 0..proc_count {
                        if let Ok(proc_rep) = unsafe { act.GetProcessActivity(j) } {
                            let streaming = unsafe { proc_rep.GetStreamingState().unwrap_or_default() };
                            let pid = unsafe { proc_rep.GetProcessId().unwrap_or(0) };
                            if streaming.as_bool() {
                                any_streaming = true;
                                if pid > 0 {
                                    let name = get_app_name_from_pid(pid);
                                    if !name.is_empty() && !active_apps.contains(&name) {
                                        active_apps.push(name);
                                    }
                                }
                            }
                        }
                    }

                    if any_streaming && active_apps.is_empty() && !friendly_sensor.is_empty() {
                        active_apps.push(friendly_sensor);
                    }
                }
            }

            MF_CAMERA_ACTIVE.store(any_streaming, std::sync::atomic::Ordering::SeqCst);
            if any_streaming {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                MF_LAST_ACTIVE_MS.store(now_ms, std::sync::atomic::Ordering::SeqCst);
                let app_str = if active_apps.is_empty() {
                    "Camera".to_string()
                } else {
                    active_apps.join(", ")
                };
                if let Ok(mut lock) = MF_CAMERA_APP.lock() {
                    *lock = Some(app_str);
                }
            } else {
                MF_LAST_ACTIVE_MS.store(0, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut lock) = MF_CAMERA_APP.lock() {
                    *lock = None;
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn init_camera_monitor() {
    unsafe {
        if MFStartup(MF_VERSION, 0).is_ok() {
            let cb: IMFSensorActivitiesReportCallback = OrbitSensorActivityCallback.into();
            if let Ok(monitor) = MFCreateSensorActivityMonitor(&cb) {
                if monitor.Start().is_ok() {
                    if let Ok(mut lock) = MONITOR_HANDLE.lock() {
                        lock.0 = Some(monitor);
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn init_camera_monitor() {}

#[cfg(target_os = "windows")]
pub fn inspect_privacy_indicators() -> PrivacyState {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_READ,
    };

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

    let mut cam_ok = false;
    let mut mic_ok = false;

    for &hive in &[HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let mut cam_key = 0 as HKEY;
        unsafe {
            if RegOpenKeyExW(hive, cam_path.as_ptr(), 0, KEY_READ, &mut cam_key) == ERROR_SUCCESS {
                check_subkeys(cam_key, &mut cam_apps);
                RegCloseKey(cam_key);
                cam_ok = true;
            }
        }

        let mut mic_key = 0 as HKEY;
        unsafe {
            if RegOpenKeyExW(hive, mic_path.as_ptr(), 0, KEY_READ, &mut mic_key) == ERROR_SUCCESS {
                check_subkeys(mic_key, &mut mic_apps);
                RegCloseKey(mic_key);
                mic_ok = true;
            }
        }
    }

    // Supplementary Media Foundation Sensor Activity check:
    // Instantly catches any Windows 11 camera streaming in Chrome, Brave, Zoom, Discord, OBS, Camera, etc.
    // Media Foundation dispatches OnActivitiesReport every ~1s while streaming; expire after 3.5s if idle.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let last_mf_ms = MF_LAST_ACTIVE_MS.load(std::sync::atomic::Ordering::Relaxed);
    if MF_CAMERA_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
        && last_mf_ms > 0
        && now_ms.saturating_sub(last_mf_ms) < 3500
    {
        if let Ok(lock) = MF_CAMERA_APP.lock() {
            let app_name = lock.clone().unwrap_or_else(|| "Camera".to_string());
            if !cam_apps.contains(&app_name) {
                cam_apps.push(app_name);
            }
        } else if cam_apps.is_empty() {
            cam_apps.push("Camera".to_string());
        }
        cam_ok = true;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_privacy() {
        init_camera_monitor();
        let state = inspect_privacy_indicators();
        eprintln!(
            "PRIVACY_STATE_TEST_RESULT: camera_active={}, mic_active={}, cam_app={:?}, mic_app={:?}",
            state.camera_active, state.microphone_active, state.camera_app, state.microphone_app
        );
        assert!(state.is_reliable);
    }
}
