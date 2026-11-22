//! Camera and microphone privacy indicator monitor (1 Hz watcher).

use crate::media::ComGuard;
use crossbeam_channel::Sender;
use orbit_core::events::{CoreEvent, PrivacyState, WakeFn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(target_os = "windows")]
static MF_CAMERA_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static MF_CAMERA_APP: Mutex<Option<String>> = Mutex::new(None);
#[cfg(target_os = "windows")]
static MF_CAMERA_PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

#[cfg(target_os = "windows")]
struct MonitorWrapper(Option<windows::Win32::Media::MediaFoundation::IMFSensorActivityMonitor>);
#[cfg(target_os = "windows")]
unsafe impl Send for MonitorWrapper {}
#[cfg(target_os = "windows")]
unsafe impl Sync for MonitorWrapper {}

#[cfg(target_os = "windows")]
static MONITOR_HANDLE: Mutex<MonitorWrapper> = Mutex::new(MonitorWrapper(None));
#[cfg(target_os = "windows")]
static MF_STARTED: AtomicBool = AtomicBool::new(false);

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
    let clean_path = raw.replace('#', "\\");
    let path = std::path::Path::new(&clean_path);
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str())
        && !file_stem.is_empty()
    {
        let mut chars = file_stem.chars();
        return match chars.next() {
            None => raw.to_string(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        };
    }
    raw.to_string()
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
    };
    use windows::Win32::Media::MediaFoundation::{
        IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
        IMFSensorActivitiesReportCallback_Impl, MF_VERSION, MFCreateSensorActivityMonitor,
        MFShutdown, MFStartup,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
    use windows_core::{Interface, implement};
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW,
        RegOpenKeyExW, RegQueryValueExW,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };

    const WIDE_NON_PACKAGED: &[u16] = &[
        b'N' as u16, b'o' as u16, b'n' as u16, b'P' as u16, b'a' as u16, b'c' as u16, b'k' as u16,
        b'a' as u16, b'g' as u16, b'e' as u16, b'd' as u16, 0,
    ];
    const WIDE_LAST_START: &[u16] = &[
        b'L' as u16, b'a' as u16, b's' as u16, b't' as u16, b'U' as u16, b's' as u16, b'e' as u16,
        b'd' as u16, b'T' as u16, b'i' as u16, b'm' as u16, b'e' as u16, b'S' as u16, b't' as u16,
        b'a' as u16, b'r' as u16, b't' as u16, 0,
    ];
    const WIDE_LAST_STOP: &[u16] = &[
        b'L' as u16, b'a' as u16, b's' as u16, b't' as u16, b'U' as u16, b's' as u16, b'e' as u16,
        b'd' as u16, b'T' as u16, b'i' as u16, b'm' as u16, b'e' as u16, b'S' as u16, b't' as u16,
        b'o' as u16, b'p' as u16, 0,
    ];

    fn wide_eq_ignore_ascii_case(a: &[u16], b_ascii: &[u8]) -> bool {
        a.len() == b_ascii.len()
            && a.iter()
                .zip(b_ascii)
                .all(|(&w, &b)| w <= 0x7F && (w as u8).eq_ignore_ascii_case(&b))
    }

    pub type AudioEnumeratorCache = Option<IMMDeviceEnumerator>;

    fn get_app_name_from_pid(pid: u32) -> String {
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

    fn is_pid_alive(pid: u32) -> bool {
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

    fn inspect_wasapi_microphones(
        enumerator_cache: &mut AudioEnumeratorCache,
        out_apps: &mut Vec<String>,
    ) -> bool {
        let mut any_mic_active = false;
        unsafe {
            if enumerator_cache.is_none() {
                *enumerator_cache =
                    CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
                        .ok();
            }
            let Some(enumerator) = enumerator_cache.as_ref() else {
                return false;
            };
            let Ok(devices) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
                *enumerator_cache = None;
                return false;
            };
            let count = devices.GetCount().unwrap_or(0);
            for i in 0..count {
                if let Ok(device) = devices.Item(i) {
                    if let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                        && let Ok(session_enum) = manager.GetSessionEnumerator()
                    {
                        let session_count = session_enum.GetCount().unwrap_or(0);
                        for s in 0..session_count {
                            if let Ok(control) = session_enum.GetSession(s)
                                && let Ok(state) = control.GetState()
                                && state == AudioSessionStateActive
                            {
                                any_mic_active = true;
                                if let Ok(control2) = control.cast::<IAudioSessionControl2>()
                                    && let Ok(pid) = control2.GetProcessId()
                                    && pid > 0
                                {
                                    let app_name = get_app_name_from_pid(pid);
                                    if !app_name.is_empty() && !out_apps.contains(&app_name) {
                                        out_apps.push(app_name);
                                    }
                                }
                            }
                        }
                    }
                    if let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
                        && let Ok(peak) = meter.GetPeakValue()
                        && peak > 0.005
                    {
                        any_mic_active = true;
                        if out_apps.is_empty() {
                            out_apps.push("Microphone".to_string());
                        }
                    }
                }
            }
        }
        any_mic_active
    }

    #[implement(IMFSensorActivitiesReportCallback)]
    struct OrbitSensorActivityCallback;

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
                        .is_ok()
                            && written > 0
                        {
                            let len = (written as usize).min(name_buf.len());
                            String::from_utf16_lossy(&name_buf[..len])
                                .trim_end_matches('\0')
                                .to_string()
                        } else {
                            String::new()
                        };

                        let proc_count = unsafe { act.GetProcessCount().unwrap_or(0) };
                        for j in 0..proc_count {
                            if let Ok(proc_rep) = unsafe { act.GetProcessActivity(j) } {
                                let streaming =
                                    unsafe { proc_rep.GetStreamingState().unwrap_or_default() };
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

                MF_CAMERA_ACTIVE.store(any_streaming, Ordering::SeqCst);
                if any_streaming {
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

    pub fn init_camera_monitor() {
        unsafe {
            if MFStartup(MF_VERSION, 0).is_ok() {
                MF_STARTED.store(true, Ordering::Release);
                let cb: IMFSensorActivitiesReportCallback = OrbitSensorActivityCallback.into();
                if let Ok(monitor) = MFCreateSensorActivityMonitor(&cb)
                    && monitor.Start().is_ok()
                    && let Ok(mut lock) = MONITOR_HANDLE.lock()
                {
                    lock.0 = Some(monitor);
                }
            }
        }
    }

    pub fn shutdown_camera_monitor() {
        if let Ok(mut lock) = MONITOR_HANDLE.lock()
            && let Some(monitor) = lock.0.take()
        {
            unsafe {
                let _ = monitor.Stop();
            }
        }
        if MF_STARTED.swap(false, Ordering::AcqRel) {
            unsafe {
                let _ = MFShutdown();
            }
        }
    }

    unsafe fn check_subkeys(root: HKEY, out_apps: &mut Vec<String>) -> bool {
        unsafe {
            let mut index = 0u32;
            let mut key_name_buf = [0u16; 512];
            let mut had_success = false;
            loop {
                let mut name_len = (key_name_buf.len() - 1) as u32;
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
                let n = (name_len as usize).min(key_name_buf.len() - 1);
                key_name_buf[n] = 0;
                if wide_eq_ignore_ascii_case(&key_name_buf[..n], b"NonPackaged") {
                    let mut np_key = std::ptr::null_mut();
                    if RegOpenKeyExW(root, WIDE_NON_PACKAGED.as_ptr(), 0, KEY_READ, &mut np_key)
                        == ERROR_SUCCESS
                    {
                        check_subkeys(np_key, out_apps);
                        RegCloseKey(np_key);
                    }
                    continue;
                }
                let mut app_key = std::ptr::null_mut();
                if RegOpenKeyExW(root, key_name_buf.as_ptr(), 0, KEY_READ, &mut app_key)
                    == ERROR_SUCCESS
                {
                    let mut start: u64 = 0;
                    let mut stop: u64 = 0;
                    let mut size = std::mem::size_of::<u64>() as u32;
                    let res_start = RegQueryValueExW(
                        app_key,
                        WIDE_LAST_START.as_ptr(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut start as *mut u64 as *mut u8,
                        &mut size,
                    );
                    size = std::mem::size_of::<u64>() as u32;
                    let res_stop = RegQueryValueExW(
                        app_key,
                        WIDE_LAST_STOP.as_ptr(),
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
                            let subkey_name = OsString::from_wide(&key_name_buf[..n])
                                .to_string_lossy()
                                .to_string();
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

    pub fn inspect_privacy_indicators() -> PrivacyState {
        let mut cache = None;
        inspect_privacy_indicators_cached(&mut cache)
    }

    pub fn inspect_privacy_indicators_cached(
        enumerator_cache: &mut AudioEnumeratorCache,
    ) -> PrivacyState {
        static CAM_PATH: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();
        static MIC_PATH: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();

        let mut cam_apps = Vec::new();
        let mut mic_apps = Vec::new();
        let wasapi_mic_active = inspect_wasapi_microphones(enumerator_cache, &mut mic_apps);
        let mut mic_ok = wasapi_mic_active;

        let cam_path = CAM_PATH.get_or_init(|| {
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\webcam\0"
                .encode_utf16()
                .collect()
        });
        let mic_path = MIC_PATH.get_or_init(|| {
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\0"
                .encode_utf16()
                .collect()
        });

        let mut cam_ok = false;
        for &hive in &[HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let mut cam_key = std::ptr::null_mut();
            unsafe {
                if RegOpenKeyExW(hive, cam_path.as_ptr(), 0, KEY_READ, &mut cam_key)
                    == ERROR_SUCCESS
                {
                    check_subkeys(cam_key, &mut cam_apps);
                    RegCloseKey(cam_key);
                    cam_ok = true;
                }
            }
            let mut mic_key = std::ptr::null_mut();
            unsafe {
                if RegOpenKeyExW(hive, mic_path.as_ptr(), 0, KEY_READ, &mut mic_key)
                    == ERROR_SUCCESS
                {
                    check_subkeys(mic_key, &mut mic_apps);
                    RegCloseKey(mic_key);
                    mic_ok = true;
                }
            }
        }

        if MF_CAMERA_ACTIVE.load(Ordering::SeqCst) {
            let mut pids_still_running = true;
            if let Ok(lock) = MF_CAMERA_PIDS.lock()
                && !lock.is_empty()
            {
                pids_still_running = lock.iter().any(|&pid| is_pid_alive(pid));
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
                MF_CAMERA_ACTIVE.store(false, Ordering::SeqCst);
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
            camera_app: (!cam_apps.is_empty()).then(|| cam_apps.join(", ")),
            microphone_active: !mic_apps.is_empty(),
            microphone_app: (!mic_apps.is_empty()).then(|| mic_apps.join(", ")),
            is_reliable: cam_ok || mic_ok,
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win {
    use super::*;
    pub type AudioEnumeratorCache = Option<()>;
    pub fn init_camera_monitor() {}
    pub fn shutdown_camera_monitor() {}
    pub fn inspect_privacy_indicators() -> PrivacyState {
        PrivacyState::default()
    }
    pub fn inspect_privacy_indicators_cached(_cache: &mut AudioEnumeratorCache) -> PrivacyState {
        PrivacyState::default()
    }
}

pub use win::{init_camera_monitor, inspect_privacy_indicators};

pub struct PrivacyService {
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    last_state: Arc<Mutex<PrivacyState>>,
    thread: Option<JoinHandle<()>>,
}

impl PrivacyService {
    pub fn spawn(
        tx: Sender<CoreEvent>,
        wake: WakeFn,
        enabled: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));
        let last_state = Arc::new(Mutex::new(PrivacyState::default()));

        let en = enabled.clone();
        let st = stop.clone();
        let pk = park.clone();
        let ls = last_state.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-privacy".into())
            .spawn(move || {
                let _com = ComGuard::new();
                init_camera_monitor();
                let mut enumerator_cache: win::AudioEnumeratorCache = None;
                let mut last = PrivacyState::default();

                while !st.load(Ordering::Relaxed) {
                    let state = if en.load(Ordering::Acquire) {
                        win::inspect_privacy_indicators_cached(&mut enumerator_cache)
                    } else {
                        PrivacyState::default()
                    };

                    if state != last {
                        last = state.clone();
                        {
                            let mut g = ls.lock().unwrap_or_else(|e| e.into_inner());
                            *g = state.clone();
                        }
                        if tx.send(CoreEvent::PrivacyUpdated(state)).is_err() {
                            break;
                        }
                        wake();
                    }

                    let guard = pk.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !st.load(Ordering::Relaxed) {
                        let _ = pk.1.wait_timeout(guard, Duration::from_millis(1000));
                    }
                }
            })?;

        Ok(Self {
            enabled,
            stop,
            park,
            last_state,
            thread: Some(thread),
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        self.park.1.notify_all();
    }

    pub fn state(&self) -> PrivacyState {
        self.last_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Drop for PrivacyService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.park.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        win::shutdown_camera_monitor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_inspection_succeeds_and_is_reliable() {
        let _com = ComGuard::new();
        init_camera_monitor();
        let state = inspect_privacy_indicators();
        assert!(state.is_reliable);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn clean_app_name_normalizes_known_apps() {
        assert_eq!(clean_app_name("chrome.exe"), "Google Chrome");
        assert_eq!(clean_app_name("discord.exe"), "Discord");
        assert_eq!(
            clean_app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe!App"),
            "Windows Camera"
        );
    }
}
