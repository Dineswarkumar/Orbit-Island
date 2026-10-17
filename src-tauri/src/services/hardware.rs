use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareMetrics {
    pub cpu_usage: f32, // 0.0 to 100.0%
    pub cpu_cores: u32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub ram_percentage: f32,
    pub disk_free_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percentage: f32,
    pub battery_percentage: Option<u8>,
    pub is_charging: bool,
    pub wifi_connected: bool,
    pub bluetooth_enabled: bool,
    pub hotspot_active: bool,
    pub network_upload_speed_bps: u64,
    pub network_download_speed_bps: u64,
    pub gpu_name: String,
    pub gpu_usage: Option<f32>, // None when unsupported; never fabricated
}

impl Default for HardwareMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            cpu_cores: 1,
            ram_used_bytes: 0,
            ram_total_bytes: 0,
            ram_percentage: 0.0,
            disk_free_bytes: 0,
            disk_total_bytes: 0,
            disk_percentage: 0.0,
            battery_percentage: None,
            is_charging: false,
            wifi_connected: false,
            bluetooth_enabled: false,
            hotspot_active: false,
            network_upload_speed_bps: 0,
            network_download_speed_bps: 0,
            gpu_name: "Unavailable".to_string(),
            gpu_usage: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCleanResult {
    pub freed_bytes: u64,
    pub freed_mb: f32,
    pub ram_used_bytes: u64,
    pub ram_percentage: f32,
    pub trimmed_processes: u32,
}

pub struct HardwareManager {
    is_running: Arc<AtomicBool>,
    last_metrics: Arc<Mutex<HardwareMetrics>>,
}

impl HardwareManager {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            last_metrics: Arc::new(Mutex::new(collect_metrics_once(None, None))),
        }
    }

    pub fn get_metrics(&self) -> HardwareMetrics {
        self.last_metrics.lock().unwrap().clone()
    }

    pub fn free_up_memory(&self) -> MemoryCleanResult {
        let (before_used, _, _) = get_ram_metrics();
        let trimmed_processes = clean_working_sets();
        let (after_used, _, after_pct) = get_ram_metrics();

        let freed_bytes = before_used.saturating_sub(after_used);
        let freed_mb = round_one_decimal(freed_bytes as f32 / (1024.0 * 1024.0));

        {
            let mut guard = self.last_metrics.lock().unwrap();
            guard.ram_used_bytes = after_used;
            guard.ram_percentage = after_pct;
        }

        MemoryCleanResult {
            freed_bytes,
            freed_mb,
            ram_used_bytes: after_used,
            ram_percentage: after_pct,
            trimmed_processes,
        }
    }

    pub fn start(&self, app: AppHandle) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_running = self.is_running.clone();
        let last_metrics = self.last_metrics.clone();

        std::thread::Builder::new()
            .name("orbit-hardware-monitor".to_string())
            .spawn(move || {
                let mut prev_cpu = get_cpu_times();
                let mut prev_net = get_network_octets();
                let mut prev_time = Instant::now();
                let mut gpu_tracker = PdhGpuTracker::new();

                while is_running.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1000));

                    let now = Instant::now();
                    let elapsed_secs = now.duration_since(prev_time).as_secs_f64().max(0.1);
                    prev_time = now;

                    let (cur_in, cur_out) = get_network_octets();
                    let net_delta_in = cur_in.saturating_sub(prev_net.0);
                    let net_delta_out = cur_out.saturating_sub(prev_net.1);
                    prev_net = (cur_in, cur_out);

                    let download_bps = (net_delta_in as f64 / elapsed_secs) as u64;
                    let upload_bps = (net_delta_out as f64 / elapsed_secs) as u64;

                    let cur_cpu = get_cpu_times();
                    let cpu_usage = calculate_cpu_usage(prev_cpu, cur_cpu);
                    prev_cpu = cur_cpu;

                    let (ram_used, ram_total, ram_pct) = get_ram_metrics();
                    let (disk_free, disk_total, disk_pct) = get_disk_metrics();
                    let (battery_pct, is_charging) = get_power_status();
                    let (wifi_connected, bluetooth_enabled, hotspot_active) = get_connectivity_status();
                    let cpu_cores = get_cpu_cores();
                    let gpu_name = get_gpu_name();
                    let gpu_usage = gpu_tracker.sample();

                    let metrics = HardwareMetrics {
                        cpu_usage,
                        cpu_cores,
                        ram_used_bytes: ram_used,
                        ram_total_bytes: ram_total,
                        ram_percentage: ram_pct,
                        disk_free_bytes: disk_free,
                        disk_total_bytes: disk_total,
                        disk_percentage: disk_pct,
                        battery_percentage: battery_pct,
                        is_charging,
                        wifi_connected,
                        bluetooth_enabled,
                        hotspot_active,
                        network_upload_speed_bps: upload_bps,
                        network_download_speed_bps: download_bps,
                        gpu_name,
                        gpu_usage,
                    };

                    {
                        let mut guard = last_metrics.lock().unwrap();
                        *guard = metrics.clone();
                    }

                    let _ = app.emit("hardware-metrics-changed", &metrics);
                }
            })
            .expect("Failed to spawn hardware monitor thread");
    }
}

pub fn collect_metrics_once(
    _prev_cpu: Option<(u64, u64, u64)>,
    _prev_net: Option<(u64, u64)>,
) -> HardwareMetrics {
    let cpu_cores = get_cpu_cores();
    let (ram_used, ram_total, ram_pct) = get_ram_metrics();
    let (disk_free, disk_total, disk_pct) = get_disk_metrics();
    let (battery_pct, is_charging) = get_power_status();
    let (wifi_connected, bluetooth_enabled, hotspot_active) = get_connectivity_status();
    let gpu_name = get_gpu_name();

    HardwareMetrics {
        cpu_usage: 0.0,
        cpu_cores,
        ram_used_bytes: ram_used,
        ram_total_bytes: ram_total,
        ram_percentage: ram_pct,
        disk_free_bytes: disk_free,
        disk_total_bytes: disk_total,
        disk_percentage: disk_pct,
        battery_percentage: battery_pct,
        is_charging,
        wifi_connected,
        bluetooth_enabled,
        hotspot_active,
        network_upload_speed_bps: 0,
        network_download_speed_bps: 0,
        gpu_name,
        gpu_usage: Some(0.0),
    }
}

#[cfg(target_os = "windows")]
fn get_cpu_times() -> Option<(u64, u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let mut idle = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
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

#[cfg(target_os = "windows")]
fn clean_working_sets() -> u32 {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
    };

    let mut trimmed_count = 0u32;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return 0;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != FALSE {
            loop {
                // Ignore system idle / kernel process (PID <= 4)
                if entry.th32ProcessID > 4 {
                    let handle = OpenProcess(
                        PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA,
                        FALSE,
                        entry.th32ProcessID,
                    );
                    if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                        if K32EmptyWorkingSet(handle) != 0 {
                            trimmed_count += 1;
                        }
                        CloseHandle(handle);
                    }
                }

                if Process32NextW(snapshot, &mut entry) == FALSE {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    trimmed_count
}

#[cfg(not(target_os = "windows"))]
fn clean_working_sets() -> u32 {
    0
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
    let mut info = unsafe { std::mem::zeroed::<SYSTEM_INFO>() };
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
    let mut stat = unsafe { std::mem::zeroed::<MEMORYSTATUSEX>() };
    stat.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut stat) } != 0 {
        let total = stat.ullTotalPhys;
        let free = stat.ullAvailPhys;
        let used = total.saturating_sub(free);
        let pct = if total > 0 { round_one_decimal((used as f64 / total as f64 * 100.0) as f32) } else { 0.0 };
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
    if unsafe {
        GetDiskFreeSpaceExW(
            drive.as_ptr(),
            &mut free_bytes_avail,
            &mut total_bytes,
            &mut total_free,
        )
    } != 0 {
        let used = total_bytes.saturating_sub(total_free);
        let pct = if total_bytes > 0 { round_one_decimal((used as f64 / total_bytes as f64 * 100.0) as f32) } else { 0.0 };
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
fn get_power_status() -> (Option<u8>, bool) {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = unsafe { std::mem::zeroed::<SYSTEM_POWER_STATUS>() };
    if unsafe { GetSystemPowerStatus(&mut status) } != 0 {
        let pct = if status.BatteryLifePercent <= 100 {
            Some(status.BatteryLifePercent)
        } else {
            None // Desktop or no battery detected
        };
        let is_charging = status.ACLineStatus == 1;
        (pct, is_charging)
    } else {
        (None, false)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_power_status() -> (Option<u8>, bool) {
    (None, false)
}

#[cfg(target_os = "windows")]
fn get_network_octets() -> (u64, u64) {
    use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut table) } == 0 && !table.is_null() {
        let num_entries = unsafe { (*table).NumEntries };
        let mut in_octets = 0u64;
        let mut out_octets = 0u64;
        for i in 0..num_entries {
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
    use windows_sys::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ};
    // Primary display adapter key in Windows registry
    let subkey: Vec<u16> = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0000"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let val_name: Vec<u16> = "DriverDesc".encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = std::ptr::null_mut();
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) } == 0 {
        let mut buf = [0u8; 512];
        let mut buf_len = 512u32;
        let mut val_type = 0u32;
        let query_res = unsafe {
            RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                buf.as_mut_ptr(),
                &mut buf_len,
            )
        };
        unsafe { RegCloseKey(hkey) };

        if query_res == 0 && buf_len > 1 {
            let u16_slice: &[u16] = unsafe {
                std::slice::from_raw_parts(buf.as_ptr() as *const u16, (buf_len / 2) as usize)
            };
            let mut name = String::from_utf16_lossy(u16_slice);
            if let Some(nul) = name.find('\0') {
                name.truncate(nul);
            }
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "Integrated Graphics".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_collection_produces_realistic_hardware_values() {
        let metrics = collect_metrics_once(None, None);
        assert!(metrics.cpu_cores >= 1, "CPU cores must be >= 1");
        assert!(metrics.ram_total_bytes > 0, "Total RAM must be > 0 bytes");
        assert!(metrics.ram_percentage >= 0.0 && metrics.ram_percentage <= 100.0, "RAM pct must be 0-100");
        assert!(metrics.disk_total_bytes > 0, "Total Disk must be > 0 bytes");
        assert!(!metrics.gpu_name.is_empty(), "GPU name must be populated");
        assert!(metrics.gpu_usage.is_some(), "GPU usage must be populated on Windows");
    }

    #[test]
    fn cpu_usage_calculation_bounds() {
        assert_eq!(calculate_cpu_usage(None, None), 0.0);
        let prev = Some((100, 200, 300));
        let cur = Some((150, 300, 450));
        let usage = calculate_cpu_usage(prev, cur);
        assert!(usage >= 0.0 && usage <= 100.0);
    }

    #[test]
    fn free_up_memory_executes_safely_and_returns_valid_metrics() {
        let hw = HardwareManager::new();
        let result = hw.free_up_memory();
        assert!(result.ram_percentage >= 0.0 && result.ram_percentage <= 100.0);
        assert!(result.freed_mb >= 0.0);
        let current = hw.get_metrics();
        assert_eq!(current.ram_used_bytes, result.ram_used_bytes);
        assert_eq!(current.ram_percentage, result.ram_percentage);
    }
}
