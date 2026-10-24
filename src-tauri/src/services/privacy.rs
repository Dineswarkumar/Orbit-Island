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
static MF_CAMERA_PIDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
#[cfg(target_os = "windows")]
struct MonitorWrapper(Option<windows::Win32::Media::MediaFoundation::IMFSensorActivityMonitor>);
#[cfg(target_os = "windows")]
unsafe impl Send for MonitorWrapper {}
#[cfg(target_os = "windows")]
unsafe impl Sync for MonitorWrapper {}

#[cfg(target_os = "windows")]
static MONITOR_HANDLE: std::sync::Mutex<MonitorWrapper> = std::sync::Mutex::new(MonitorWrapper(None));

#[cfg(target_os = "windows")]
use windows_core::{implement, Interface};
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eCapture, IMMDeviceEnumerator, MMDeviceEnumerator,
    IAudioSessionManager2, IAudioSessionControl2,
    AudioSessionStateActive, DEVICE_STATE_ACTIVE,
};
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
#[cfg(target_os = "windows")]
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl,
    MFCreateSensorActivityMonitor, MFStartup, MF_VERSION,
};

#[cfg(target_os = "windows")]
pub fn clean_app_name(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("windowscamera") || (lower.contains("camera") && !lower.contains("obs")) {
        return "Windows Camera".to_string();
    }
    if lower.contains("whatsapp") {
        return "WhatsApp".to_string();
    }
    if lower.contains("screensketch") || lower.contains("snippingtool") {
        return "Snipping Tool".to_string();
    }
    if lower.contains("soundrecorder") || lower.contains("windowssoundrecorder") {
        return "Sound Recorder".to_string();
    }
    if lower.contains("teams") {
        return "Microsoft Teams".to_string();
    }
    if lower.contains("discord") {
        return "Discord".to_string();
    }
    if lower.contains("zoom") {
        return "Zoom".to_string();
    }
    if lower.contains("obs64") || lower.contains("obs-studio") || lower == "obs" {
        return "OBS Studio".to_string();
    }
    if lower.contains("chrome") {
        return "Google Chrome".to_string();
    }
    if lower.contains("msedge") {
        return "Microsoft Edge".to_string();
    }
    if lower.contains("brave") {
        return "Brave Browser".to_string();
    }
    if lower.contains("firefox") {
        return "Mozilla Firefox".to_string();
    }
    if lower.contains("slack") {
        return "Slack".to_string();
    }
    if lower.contains("immersivecontrolpanel") {
        return "Windows Settings".to_string();
    }

    if raw.is_empty() {
        return String::new();
    }

    // If it's a file path or registry key with # or \
    let clean_path = raw.replace('#', "\\");
    let path = std::path::Path::new(&clean_path);
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
        if !file_stem.is_empty() {
            let mut chars = file_stem.chars();
            return match chars.next() {
                None => raw.to_string(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            };
        }
    }
    raw.to_string()
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
fn is_pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !h.is_null() {
            CloseHandle(h);
            true
        } else {
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn inspect_wasapi_microphones(out_apps: &mut Vec<String>) -> bool {
    let mut any_mic_active = false;
    unsafe {
        let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        else {
            return false;
        };

        // Enumerate all active capture endpoints (microphones, line inputs, headsets)
        let Ok(devices) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
            return false;
        };

        let count = devices.GetCount().unwrap_or(0);
        for i in 0..count {
            if let Ok(device) = devices.Item(i) {
                // Check active audio sessions on this microphone endpoint
                if let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) {
                    if let Ok(session_enum) = manager.GetSessionEnumerator() {
                        let session_count = session_enum.GetCount().unwrap_or(0);
                        for s in 0..session_count {
                            if let Ok(control) = session_enum.GetSession(s) {
                                if let Ok(state) = control.GetState() {
                                    if state == AudioSessionStateActive {
                                        any_mic_active = true;
                                        if let Ok(control2) = control.cast::<IAudioSessionControl2>() {
                                            if let Ok(pid) = control2.GetProcessId() {
                                                if pid > 0 {
                                                    let app_name = get_app_name_from_pid(pid);
                                                    if !app_name.is_empty() && !out_apps.contains(&app_name) {
                                                        out_apps.push(app_name);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Check meter peak as a secondary physical confirmation
                if let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) {
                    if let Ok(peak) = meter.GetPeakValue() {
                        if peak > 0.005 {
                            any_mic_active = true;
                            if out_apps.is_empty() {
                                out_apps.push("Microphone".to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    any_mic_active
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
            let mut active_pids: Vec<u32> = Vec::new();

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
                                    if !active_pids.contains(&pid) {
                                        active_pids.push(pid);
                                    }
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
                if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
                    *lock = active_pids;
                }
            } else {
                MF_LAST_ACTIVE_MS.store(0, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut lock) = MF_CAMERA_APP.lock() {
                    *lock = None;
                }
                if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
                    lock.clear();
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

    // 1. WASAPI Real-Time Microphone Session Inspection:
    // Core Audio session enumeration directly queries all physical recording sessions on Windows
    // with sub-millisecond accuracy, avoiding registry delays, buffering, and focus dropouts.
    let wasapi_mic_active = inspect_wasapi_microphones(&mut mic_apps);
    let mut mic_ok = wasapi_mic_active;

    // 2. Windows CapabilityAccessManager ConsentStore Registry scan:
    // Checks standard packaged and non-packaged app capability permissions in registry.
    let cam_path: Vec<u16> =
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\webcam\0"
            .encode_utf16()
            .collect();
    let mic_path: Vec<u16> =
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\0"
            .encode_utf16()
            .collect();

    let mut cam_ok = false;

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

    // 3. Media Foundation Sensor Activity Monitor for camera hardware streaming:
    // When an app (Chrome, Brave, Zoom, Teams, Discord, OBS, Camera, etc.) opens a video capture stream,
    // MF sensor activities monitor dispatches OnActivitiesReport with streaming = true.
    // We maintain active status as long as the stream is active and the capturing process is alive.
    if MF_CAMERA_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        let mut pids_still_running = true;
        if let Ok(lock) = MF_CAMERA_PIDS.lock() {
            if !lock.is_empty() {
                pids_still_running = lock.iter().any(|&pid| is_pid_alive(pid));
            }
        }

        if pids_still_running {
            if let Ok(lock) = MF_CAMERA_APP.lock() {
                let app_name = lock.clone().unwrap_or_else(|| "Camera".to_string());
                if !cam_apps.contains(&app_name) {
                    cam_apps.push(app_name);
                }
            } else if cam_apps.is_empty() {
                cam_apps.push("Camera".to_string());
            }
            cam_ok = true;
        } else {
            // Process terminated without sensor release callback
            MF_CAMERA_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
            if let Ok(mut lock) = MF_CAMERA_APP.lock() {
                *lock = None;
            }
            if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
                lock.clear();
            }
        }
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
    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_inspect_privacy() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_camera_monitor();
        let state = inspect_privacy_indicators();
        eprintln!(
            "PRIVACY_STATE_TEST_RESULT: camera_active={}, mic_active={}, cam_app={:?}, mic_app={:?}",
            state.camera_active, state.microphone_active, state.camera_app, state.microphone_app
        );
        assert!(state.is_reliable);
    }

    #[test]
    fn test_camera_stream_retention_with_live_pid() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let current_pid = std::process::id();
        MF_CAMERA_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut lock) = MF_CAMERA_APP.lock() {
            *lock = Some("Test Camera App".to_string());
        }
        if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
            *lock = vec![current_pid];
        }

        // Even after time passes, camera must remain active because the PID is alive
        let state = inspect_privacy_indicators();
        assert!(state.camera_active, "Camera indicator must persist while streaming process is alive");
        assert!(
            state.camera_app.as_deref().unwrap_or("").contains("Test Camera App"),
            "Camera app name must be preserved"
        );

        // Cleanup
        MF_CAMERA_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut lock) = MF_CAMERA_APP.lock() {
            *lock = None;
        }
        if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
            lock.clear();
        }
    }

    #[test]
    fn test_camera_stream_self_heals_when_process_dies() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // PID 99999999 is virtually guaranteed to not exist
        MF_CAMERA_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut lock) = MF_CAMERA_APP.lock() {
            *lock = Some("Crashed App".to_string());
        }
        if let Ok(mut lock) = MF_CAMERA_PIDS.lock() {
            *lock = vec![99_999_999];
        }

        let state = inspect_privacy_indicators();
        // Since PID is dead and not in registry, it must clear
        assert!(
            !MF_CAMERA_ACTIVE.load(std::sync::atomic::Ordering::SeqCst),
            "Camera state must self-heal to false when process dies"
        );
        assert!(!state.camera_active || state.camera_app != Some("Crashed App".to_string()));
    }

    #[test]
    fn test_clean_app_name_edge_cases() {
        assert_eq!(clean_app_name("chrome.exe"), "Google Chrome");
        assert_eq!(clean_app_name("msedge.exe"), "Microsoft Edge");
        assert_eq!(clean_app_name("brave.exe"), "Brave Browser");
        assert_eq!(clean_app_name("zoom.exe"), "Zoom");
        assert_eq!(clean_app_name("discord.exe"), "Discord");
        assert_eq!(clean_app_name("obs64.exe"), "OBS Studio");
        assert_eq!(clean_app_name("C:#Program Files#WhatsApp#whatsapp.exe"), "WhatsApp");
        assert_eq!(clean_app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe!App"), "Windows Camera");
        assert_eq!(clean_app_name(""), "");
        assert_eq!(clean_app_name("customapp.exe"), "Customapp");
    }

    #[test]
    fn test_rapid_consecutive_privacy_inspections() {
        let _guard = TEST_MUTEX.lock().unwrap();
        for _ in 0..50 {
            let state = inspect_privacy_indicators();
            let _ = state.is_reliable;
        }
    }
}
