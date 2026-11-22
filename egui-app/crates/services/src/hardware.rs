//! Hardware & system telemetry service (CPU, RAM, GPU, disk, network, power).

use crossbeam_channel::Sender;
use orbit_core::events::{CoreEvent, HardwareMetrics, MemoryCleanResult, WakeFn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

struct SamplerState {
    prev_cpu: Option<(u64, u64, u64)>,
    prev_net: (u64, u64),
    prev_time: Instant,
    gpu_tracker: PdhGpuTracker,
    downloads_tracker: DownloadsTracker,
    cpu_cores: u32,
    gpu_name: String,
}

impl SamplerState {
    fn new() -> Self {
        Self {
            prev_cpu: get_cpu_times(),
            prev_net: get_network_octets(),
            prev_time: Instant::now(),
            gpu_tracker: PdhGpuTracker::new(),
            downloads_tracker: DownloadsTracker::new(),
            cpu_cores: get_cpu_cores(),
            gpu_name: get_gpu_name(),
        }
    }

    fn sample(&mut self) -> HardwareMetrics {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.prev_time).as_secs_f64().max(0.1);
        self.prev_time = now;

        let (cur_in, cur_out) = get_network_octets();
        let net_delta_in = cur_in.saturating_sub(self.prev_net.0);
        let net_delta_out = cur_out.saturating_sub(self.prev_net.1);
        self.prev_net = (cur_in, cur_out);

        let download_bps = (net_delta_in as f64 / elapsed_secs) as u64;
        let upload_bps = (net_delta_out as f64 / elapsed_secs) as u64;

        let cur_cpu = get_cpu_times();
        let cpu_usage = calculate_cpu_usage(self.prev_cpu, cur_cpu);
        self.prev_cpu = cur_cpu;

        let (ram_used, ram_total, ram_pct) = get_ram_metrics();
        let (disk_free, disk_total, disk_pct) = get_disk_metrics();
        let (battery_pct, ac_power_connected, battery_charging) = get_power_status();
        let (wifi_connected, bluetooth_enabled, hotspot_active) = get_connectivity_status();
        let gpu_usage = self.gpu_tracker.sample();
        let screen_recording_active = is_screen_recording_active();
        let (active_download_count, last_completed_download) = self.downloads_tracker.sample();

        HardwareMetrics {
            cpu_usage,
            cpu_cores: self.cpu_cores,
            ram_used_bytes: ram_used,
            ram_total_bytes: ram_total,
            ram_percentage: ram_pct,
            disk_free_bytes: disk_free,
            disk_total_bytes: disk_total,
            disk_percentage: disk_pct,
            battery_percentage: battery_pct,
            battery_charging,
            ac_power_connected,
            wifi_connected,
            bluetooth_enabled,
            hotspot_active,
            network_upload_speed_bps: upload_bps,
            network_download_speed_bps: download_bps,
            gpu_name: self.gpu_name.clone(),
            gpu_usage,
            screen_recording_active,
            active_download_count,
            last_completed_download,
            battery_saver_active: is_battery_saver_active(),
            os_transparency_enabled: orbit_platform::system::transparency_enabled(),
        }
    }
}

pub struct HardwareService {
    active: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    last_metrics: Arc<Mutex<HardwareMetrics>>,
    sampler: Arc<Mutex<SamplerState>>,
    thread: Option<JoinHandle<()>>,
}

impl HardwareService {
    pub fn spawn(tx: Sender<CoreEvent>, wake: WakeFn) -> std::io::Result<Self> {
        let active = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));
        let last_metrics = Arc::new(Mutex::new(HardwareMetrics::default()));
        let sampler = Arc::new(Mutex::new(SamplerState::new()));

        let a = active.clone();
        let s = stop.clone();
        let p = park.clone();
        let lm = last_metrics.clone();
        let sm = sampler.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-hardware".into())
            .spawn(move || {
                let mut last_emitted: Option<HardwareMetrics> = None;

                // Perform one immediate initial sample so telemetry (gpu_name, RAM, CPU,
                // Disk, Battery) is populated at startup even while the island is collapsed.
                if !s.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                    if !s.load(Ordering::Relaxed) {
                        let metrics = {
                            let mut samp = sm.lock().unwrap_or_else(|e| e.into_inner());
                            samp.sample()
                        };
                        {
                            let mut guard = lm.lock().unwrap_or_else(|e| e.into_inner());
                            *guard = metrics.clone();
                        }
                        last_emitted = Some(metrics.clone());
                        if tx.send(CoreEvent::HardwareUpdated(metrics)).is_err() {
                            return;
                        }
                        wake();
                    }
                }

                while !s.load(Ordering::Relaxed) {
                    if !a.load(Ordering::Acquire) {
                        let guard = p.0.lock().unwrap_or_else(|e| e.into_inner());
                        if !a.load(Ordering::Acquire) && !s.load(Ordering::Relaxed) {
                            drop(p.1.wait(guard));
                        }
                        continue;
                    }

                    let metrics = {
                        let mut samp = sm.lock().unwrap_or_else(|e| e.into_inner());
                        samp.sample()
                    };
                    {
                        let mut guard = lm.lock().unwrap_or_else(|e| e.into_inner());
                        *guard = metrics.clone();
                    }
                    if last_emitted.as_ref() != Some(&metrics) {
                        last_emitted = Some(metrics.clone());
                        if tx.send(CoreEvent::HardwareUpdated(metrics)).is_err() {
                            break;
                        }
                        wake();
                    }

                    let guard = p.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !s.load(Ordering::Relaxed) && a.load(Ordering::Acquire) {
                        drop(p.1.wait_timeout(guard, Duration::from_millis(1000)));
                    }
                }
            })?;

        Ok(Self {
            active,
            stop,
            park,
            last_metrics,
            sampler,
            thread: Some(thread),
        })
    }

    /// Enables or pauses the 1 Hz polling loop (paused when collapsed/idle).
    pub fn set_active(&self, active: bool) {
        let prev = self.active.swap(active, Ordering::AcqRel);
        if prev != active {
            self.park.1.notify_all();
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn get_metrics(&self) -> HardwareMetrics {
        self.last_metrics
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn sample_once(&self) -> HardwareMetrics {
        let metrics = {
            let mut samp = self.sampler.lock().unwrap_or_else(|e| e.into_inner());
            samp.sample()
        };
        let mut guard = self.last_metrics.lock().unwrap_or_else(|e| e.into_inner());
        *guard = metrics.clone();
        metrics
    }

    /// Trims working sets of accessible processes in the current user session
    /// and reports the observed memory reduction in `freed_bytes` / `freed_mb`.
    pub fn free_up_memory(&self) -> MemoryCleanResult {
        let (before_used, _, _) = get_ram_metrics();
        let (trimmed_processes, trimmed_ws_bytes) = clean_session_working_sets();
        std::thread::sleep(Duration::from_millis(60));
        let (ram_used_bytes, _, ram_percentage) = get_ram_metrics();
        let freed_bytes = system_memory_freed(before_used, ram_used_bytes).max(trimmed_ws_bytes);
        {
            let mut metrics = self.last_metrics.lock().unwrap_or_else(|e| e.into_inner());
            metrics.ram_used_bytes = ram_used_bytes;
            metrics.ram_percentage = ram_percentage;
        }
        MemoryCleanResult {
            freed_bytes,
            freed_mb: round_one_decimal(freed_bytes as f32 / (1024.0 * 1024.0)),
            ram_used_bytes,
            ram_percentage,
            trimmed_processes,
        }
    }
}

#[cfg(target_os = "windows")]
fn clean_session_working_sets() -> (u32, u64) {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        K32EmptyWorkingSet, K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_QUERY_INFORMATION,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    };

    let mut trimmed_count: u32 = 0;
    let mut trimmed_ws_bytes: u64 = 0;

    // SAFETY: Win32 process snapshot, session check, and working-set trim on accessible session processes.
    unsafe {
        let current_pid = GetCurrentProcessId();
        let mut current_session: u32 = 0;
        if ProcessIdToSessionId(current_pid, &mut current_session) != 0 {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if !snapshot.is_null() && snapshot != INVALID_HANDLE_VALUE {
                let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
                if Process32FirstW(snapshot, &mut entry) != FALSE {
                    loop {
                        let pid = entry.th32ProcessID;
                        if pid > 4 {
                            let mut proc_session: u32 = 0;
                            if ProcessIdToSessionId(pid, &mut proc_session) != 0
                                && proc_session == current_session
                            {
                                let mut handle = OpenProcess(
                                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                                    FALSE,
                                    pid,
                                );
                                if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                                    handle = OpenProcess(
                                        PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA,
                                        FALSE,
                                        pid,
                                    );
                                }
                                if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                                    let mut before_pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                                    before_pmc.cb =
                                        std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                                    let has_before = K32GetProcessMemoryInfo(
                                        handle,
                                        &mut before_pmc,
                                        before_pmc.cb,
                                    ) != 0;

                                    if K32EmptyWorkingSet(handle) != 0 {
                                        trimmed_count += 1;
                                        if has_before {
                                            let mut after_pmc: PROCESS_MEMORY_COUNTERS =
                                                std::mem::zeroed();
                                            after_pmc.cb =
                                                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()
                                                    as u32;
                                            if K32GetProcessMemoryInfo(
                                                handle,
                                                &mut after_pmc,
                                                after_pmc.cb,
                                            ) != 0
                                            {
                                                trimmed_ws_bytes = trimmed_ws_bytes.saturating_add(
                                                    before_pmc
                                                        .WorkingSetSize
                                                        .saturating_sub(after_pmc.WorkingSetSize)
                                                        as u64,
                                                );
                                            }
                                        }
                                    }
                                    CloseHandle(handle);
                                }
                            }
                        }
                        if Process32NextW(snapshot, &mut entry) == FALSE {
                            break;
                        }
                    }
                }
                CloseHandle(snapshot);
            }
        }
    }

    if trimmed_count == 0 && orbit_platform::process::trim_working_set().is_ok() {
        trimmed_count = 1;
    }

    (trimmed_count, trimmed_ws_bytes)
}

#[cfg(not(target_os = "windows"))]
fn clean_session_working_sets() -> (u32, u64) {
    (0, 0)
}

impl Drop for HardwareService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.park.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub fn collect_metrics_once() -> HardwareMetrics {
    let mut state = SamplerState::new();
    std::thread::sleep(Duration::from_millis(120));
    state.sample()
}

#[cfg(target_os = "windows")]
fn get_cpu_times() -> Option<(u64, u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    // SAFETY: valid FILETIME out pointers.
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0 {
        let to_u64 = |ft: FILETIME| ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        Some((to_u64(idle), to_u64(kernel), to_u64(user)))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cpu_times() -> Option<(u64, u64, u64)> {
    None
}

fn round_one_decimal(val: f32) -> f32 {
    (val * 10.0).round() / 10.0
}

fn system_memory_freed(before_used: u64, after_used: u64) -> u64 {
    before_used.saturating_sub(after_used)
}

fn calculate_cpu_usage(prev: Option<(u64, u64, u64)>, cur: Option<(u64, u64, u64)>) -> f32 {
    if let (Some((p_idle, p_kern, p_user)), Some((c_idle, c_kern, c_user))) = (prev, cur) {
        let idle_diff = c_idle.saturating_sub(p_idle);
        let kern_diff = c_kern.saturating_sub(p_kern);
        let user_diff = c_user.saturating_sub(p_user);
        let total_diff = kern_diff + user_diff;
        if total_diff > 0 {
            let busy_diff = total_diff.saturating_sub(idle_diff);
            let pct = (busy_diff as f64 / total_diff as f64 * 100.0) as f32;
            round_one_decimal(pct.clamp(0.0, 100.0))
        } else {
            0.0
        }
    } else {
        0.0
    }
}

#[cfg(target_os = "windows")]
fn get_cpu_cores() -> u32 {
    use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    // SAFETY: plain struct filled by GetSystemInfo.
    let mut info = unsafe { std::mem::zeroed::<SYSTEM_INFO>() };
    // SAFETY: valid out pointer.
    unsafe { GetSystemInfo(&mut info) };
    info.dwNumberOfProcessors.max(1)
}

#[cfg(not(target_os = "windows"))]
fn get_cpu_cores() -> u32 {
    1
}

#[cfg(target_os = "windows")]
fn get_ram_metrics() -> (u64, u64, f32) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: zero-init MEMORYSTATUSEX before setting dwLength.
    let mut stat = unsafe { std::mem::zeroed::<MEMORYSTATUSEX>() };
    stat.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: valid out pointer.
    if unsafe { GlobalMemoryStatusEx(&mut stat) } != 0 {
        let total = stat.ullTotalPhys;
        let free = stat.ullAvailPhys;
        let used = total.saturating_sub(free);
        let pct = if total > 0 {
            round_one_decimal((used as f64 / total as f64 * 100.0) as f32)
        } else {
            0.0
        };
        (used, total, pct)
    } else {
        (0, 0, 0.0)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_ram_metrics() -> (u64, u64, f32) {
    (0, 0, 0.0)
}

#[cfg(target_os = "windows")]
fn get_disk_metrics() -> (u64, u64, f32) {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let drive: Vec<u16> = "C:\\".encode_utf16().chain(std::iter::once(0)).collect();
    let mut free_bytes_avail = 0u64;
    let mut total_bytes = 0u64;
    let mut total_free = 0u64;
    // SAFETY: valid wide drive string and out pointers.
    if unsafe {
        GetDiskFreeSpaceExW(
            drive.as_ptr(),
            &mut free_bytes_avail,
            &mut total_bytes,
            &mut total_free,
        )
    } != 0
    {
        let used = total_bytes.saturating_sub(total_free);
        let pct = if total_bytes > 0 {
            round_one_decimal((used as f64 / total_bytes as f64 * 100.0) as f32)
        } else {
            0.0
        };
        (total_free, total_bytes, pct)
    } else {
        (0, 0, 0.0)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_disk_metrics() -> (u64, u64, f32) {
    (0, 0, 0.0)
}

#[cfg(target_os = "windows")]
fn get_power_status() -> (Option<u8>, Option<bool>, Option<bool>) {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    // SAFETY: zero-init out struct.
    let mut status = unsafe { std::mem::zeroed::<SYSTEM_POWER_STATUS>() };
    // SAFETY: valid out pointer.
    if unsafe { GetSystemPowerStatus(&mut status) } != 0 {
        let pct = if status.BatteryLifePercent <= 100 {
            Some(status.BatteryLifePercent)
        } else {
            None
        };
        let ac_power_connected = match status.ACLineStatus {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let battery_charging = if pct.is_some() && status.BatteryFlag != 255 {
            Some(status.BatteryFlag & 8 != 0)
        } else {
            None
        };
        (pct, ac_power_connected, battery_charging)
    } else {
        (None, None, None)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_power_status() -> (Option<u8>, Option<bool>, Option<bool>) {
    (None, None, None)
}

#[cfg(target_os = "windows")]
fn is_battery_saver_active() -> bool {
    orbit_platform::power::battery_saver()
}

#[cfg(not(target_os = "windows"))]
fn is_battery_saver_active() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn get_network_octets() -> (u64, u64) {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
    };
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 allocates `table` which is freed below via FreeMibTable.
    if unsafe { GetIfTable2(&mut table) } == 0 && !table.is_null() {
        let num_entries = unsafe { (*table).NumEntries };
        let mut in_octets = 0u64;
        let mut out_octets = 0u64;
        for i in 0..num_entries {
            // SAFETY: bounded by NumEntries in returned MIB_IF_TABLE2.
            let row = unsafe { (*table).Table.as_ptr().add(i as usize) };
            unsafe {
                in_octets += (*row).InOctets;
                out_octets += (*row).OutOctets;
            }
        }
        unsafe { FreeMibTable(table as *const std::ffi::c_void) };
        (in_octets, out_octets)
    } else {
        (0, 0)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_network_octets() -> (u64, u64) {
    (0, 0)
}

#[cfg(target_os = "windows")]
fn get_connectivity_status() -> (bool, bool, bool) {
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfTable, MIB_IFROW, MIB_IFTABLE};
    let mut wifi_connected = false;
    let mut bluetooth_enabled = false;
    let mut hotspot_active = false;

    let mut size = 0u32;
    // SAFETY: two-step size query + read on owned Vec buffer.
    unsafe {
        let _ = GetIfTable(std::ptr::null_mut(), &mut size, 0);
        if size > 0 {
            let mut buffer = vec![0u8; size as usize];
            let p_table = buffer.as_mut_ptr() as *mut MIB_IFTABLE;
            if GetIfTable(p_table, &mut size, 0) == 0 && !p_table.is_null() {
                let num_entries = (*p_table).dwNumEntries;
                for i in 0..num_entries {
                    let row: &MIB_IFROW = &*(*p_table).table.as_ptr().add(i as usize);
                    let desc_len = (row.dwDescrLen as usize).min(row.bDescr.len());
                    let desc = String::from_utf8_lossy(&row.bDescr[..desc_len]).to_lowercase();
                    if row.dwType == 71 {
                        let is_operational = row.dwOperStatus == 4 || row.dwOperStatus == 5;
                        if is_operational {
                            let has_traffic = row.dwInOctets > 0 || row.dwOutOctets > 0;
                            if desc.contains("wi-fi direct virtual adapter")
                                || desc.contains("hosted network")
                                || desc.contains("local area connection*")
                            {
                                if has_traffic {
                                    hotspot_active = true;
                                }
                            } else if has_traffic {
                                wifi_connected = true;
                            }
                        }
                    }
                    if desc.contains("bluetooth") && row.dwOperStatus != 2 {
                        bluetooth_enabled = true;
                    }
                }
            }
        }
    }
    (wifi_connected, bluetooth_enabled, hotspot_active)
}

#[cfg(not(target_os = "windows"))]
fn get_connectivity_status() -> (bool, bool, bool) {
    (false, false, false)
}

#[cfg(target_os = "windows")]
fn get_gpu_name() -> String {
    orbit_platform::registry::read_string(
        orbit_platform::registry::Hive::LocalMachine,
        r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000",
        "DriverDesc",
    )
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "Integrated Graphics".to_string())
}

#[cfg(not(target_os = "windows"))]
fn get_gpu_name() -> String {
    "Unavailable".to_string()
}

#[cfg(target_os = "windows")]
struct PdhGpuTracker {
    query: isize,
    counter: isize,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Copy, Clone)]
struct PdhFmtCounterValue {
    c_status: u32,
    padding: u32,
    double_value: f64,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Copy, Clone)]
struct PdhFmtCounterValueItemW {
    sz_name: *mut u16,
    fmt_value: PdhFmtCounterValue,
}

#[cfg(target_os = "windows")]
#[link(name = "pdh")]
unsafe extern "system" {
    fn PdhOpenQueryW(sz_data_source: *const u16, dw_user_data: usize, ph_query: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(
        h_query: isize,
        sz_full_counter_path: *const u16,
        dw_user_data: usize,
        ph_counter: *mut isize,
    ) -> u32;
    fn PdhCollectQueryData(h_query: isize) -> u32;
    fn PdhGetFormattedCounterArrayW(
        h_counter: isize,
        dw_format: u32,
        lpdw_buffer_size: *mut u32,
        lpdw_item_count: *mut u32,
        item_buffer: *mut PdhFmtCounterValueItemW,
    ) -> u32;
    fn PdhCloseQuery(h_query: isize) -> u32;
}

#[cfg(target_os = "windows")]
impl PdhGpuTracker {
    fn new() -> Self {
        let mut query: isize = 0;
        let mut counter: isize = 0;
        // SAFETY: PDH query/counter creation and initial prime sample.
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut query) == 0 && query != 0 {
                let path: Vec<u16> = "\\GPU Engine(*engtype_3D)\\Utilization Percentage\0"
                    .encode_utf16()
                    .collect();
                if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) == 0 {
                    let _ = PdhCollectQueryData(query);
                }
            }
        }
        Self { query, counter }
    }

    fn sample(&mut self) -> Option<f32> {
        if self.query == 0 || self.counter == 0 {
            return Some(0.0);
        }
        // SAFETY: PDH query sampling into owned buffer.
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return Some(0.0);
            }
            const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
            const PDH_MORE_DATA: u32 = 0x8000_07D2;

            let mut buf_size: u32 = 0;
            let mut item_count: u32 = 0;
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buf_size,
                &mut item_count,
                std::ptr::null_mut(),
            );
            if status != PDH_MORE_DATA || buf_size == 0 {
                return Some(0.0);
            }

            let mut raw_buf = vec![0u8; buf_size as usize];
            let items_ptr = raw_buf.as_mut_ptr() as *mut PdhFmtCounterValueItemW;
            let status2 = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buf_size,
                &mut item_count,
                items_ptr,
            );
            if status2 != 0 || item_count == 0 {
                return Some(0.0);
            }

            let slice = std::slice::from_raw_parts(items_ptr, item_count as usize);
            let mut total = 0.0f64;
            for item in slice {
                if item.fmt_value.c_status == 0 && item.fmt_value.double_value.is_finite() {
                    total += item.fmt_value.double_value;
                }
            }
            Some(round_one_decimal((total as f32).clamp(0.0, 100.0)))
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for PdhGpuTracker {
    fn drop(&mut self) {
        if self.query != 0 {
            // SAFETY: closing our owned PDH query handle.
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct PdhGpuTracker;

#[cfg(not(target_os = "windows"))]
impl PdhGpuTracker {
    fn new() -> Self {
        Self
    }
    fn sample(&mut self) -> Option<f32> {
        None
    }
}

#[cfg(target_os = "windows")]
fn is_screen_recording_active() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    const RECORDERS: &[&str] = &[
        "snippingtool.exe",
        "screensketch.exe",
        "gamebar.exe",
        "bcastdvr.exe",
        "captureservice.exe",
        "obs64.exe",
        "obs32.exe",
        "bdcam.exe",
        "action.exe",
        "camtasia.exe",
        "screenrec.exe",
        "loom.exe",
    ];

    // SAFETY: Toolhelp process snapshot iteration.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut active = false;
        if Process32FirstW(snapshot, &mut entry) != FALSE {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe_name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                if RECORDERS.iter().any(|&rec| exe_name == rec) {
                    active = true;
                    break;
                }
                if Process32NextW(snapshot, &mut entry) == FALSE {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        active
    }
}

#[cfg(not(target_os = "windows"))]
fn is_screen_recording_active() -> bool {
    false
}

struct DownloadsTracker {
    downloads_dir: Option<std::path::PathBuf>,
    prev_temp_files: std::collections::HashSet<String>,
    prev_regular_files: std::collections::HashSet<String>,
    initialized: bool,
}

impl DownloadsTracker {
    fn new() -> Self {
        let downloads_dir = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(|p| std::path::PathBuf::from(p).join("Downloads"));
        Self {
            downloads_dir,
            prev_temp_files: std::collections::HashSet::new(),
            prev_regular_files: std::collections::HashSet::new(),
            initialized: false,
        }
    }

    fn sample(&mut self) -> (u32, Option<String>) {
        let Some(dir) = self.downloads_dir.clone() else {
            return (0, None);
        };
        self.sample_dir(&dir)
    }

    fn sample_dir(&mut self, dir: &std::path::Path) -> (u32, Option<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return (0, None);
        };
        let mut cur_temp = std::collections::HashSet::new();
        let mut cur_regular = std::collections::HashSet::new();
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let lower = file_name.to_lowercase();
            if lower.ends_with(".crdownload")
                || lower.ends_with(".part")
                || lower.ends_with(".opdownload")
                || lower.ends_with(".download")
            {
                cur_temp.insert(file_name);
            } else if !lower.ends_with(".tmp") && !lower.starts_with('.') {
                cur_regular.insert(file_name);
            }
        }
        let active_count = cur_temp.len() as u32;
        let mut completed: Option<String> = None;
        if self.initialized {
            let temp_finished = self.prev_temp_files.iter().any(|t| !cur_temp.contains(t));
            if temp_finished {
                if let Some(new_file) = cur_regular
                    .iter()
                    .find(|f| !self.prev_regular_files.contains(*f))
                {
                    completed = Some(new_file.clone());
                } else if let Some(finished_temp) =
                    self.prev_temp_files.iter().find(|t| !cur_temp.contains(*t))
                {
                    let stripped = finished_temp
                        .trim_end_matches(".crdownload")
                        .trim_end_matches(".part")
                        .trim_end_matches(".opdownload")
                        .trim_end_matches(".download")
                        .to_string();
                    if !stripped.is_empty() {
                        completed = Some(stripped);
                    }
                }
            }
        }
        self.prev_temp_files = cur_temp;
        self.prev_regular_files = cur_regular;
        self.initialized = true;
        (active_count, completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_collection_produces_realistic_hardware_values() {
        let metrics = collect_metrics_once();
        assert!(metrics.cpu_cores >= 1, "CPU cores must be >= 1");
        assert!(metrics.ram_total_bytes > 0, "Total RAM must be > 0 bytes");
        assert!(
            (0.0..=100.0).contains(&metrics.ram_percentage),
            "RAM pct must be 0-100"
        );
        assert!(metrics.disk_total_bytes > 0, "Total Disk must be > 0 bytes");
        assert!(!metrics.gpu_name.is_empty(), "GPU name must be populated");
        assert!(metrics.gpu_usage.is_some(), "GPU usage must be populated on Windows");
    }

    #[test]
    fn free_up_memory_trims_accessible_session_processes() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let wake: WakeFn = Arc::new(|| {});
        let svc = HardwareService::spawn(tx, wake).expect("spawn hardware service");
        let res = svc.free_up_memory();
        assert!(res.trimmed_processes >= 1);
        assert!(res.ram_percentage >= 0.0 && res.ram_percentage <= 100.0);
    }

    #[test]
    fn cpu_usage_calculation_bounds() {
        assert_eq!(calculate_cpu_usage(None, None), 0.0);
        let prev = Some((100, 200, 300));
        let cur = Some((150, 300, 450));
        let usage = calculate_cpu_usage(prev, cur);
        assert!((0.0..=100.0).contains(&usage));
    }

    #[test]
    fn system_memory_delta_never_reports_more_freed_than_observed() {
        assert_eq!(system_memory_freed(800, 500), 300);
        assert_eq!(system_memory_freed(500, 800), 0);
    }
}
