use serde::Serialize;
use std::collections::HashSet;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub working_set_bytes: usize,
    pub working_set_mb: f64,
    pub private_bytes: usize,
    pub private_mb: f64,
    pub private_working_set_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReport {
    pub main_process: ProcessItem,
    pub webview_processes: Vec<ProcessItem>,
    pub unmeasured_webview_count: usize,
    pub combined_private_working_set_mb: Option<f64>,
    pub measured_at_unix_ms: u128,
    pub measurement_method: String,
}

#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
    parent_pid: u32,
    name: String,
    created_at: u64,
}

fn descendants(processes: &[ProcessInfo], root_pid: u32, root_created_at: u64) -> Vec<ProcessInfo> {
    let mut result = Vec::new();
    let mut pending = vec![(root_pid, root_created_at)];
    let mut seen = HashSet::from([root_pid]);

    while let Some((parent_pid, parent_created_at)) = pending.pop() {
        for process in processes.iter().filter(|process| process.parent_pid == parent_pid && process.created_at >= parent_created_at) {
            if seen.insert(process.pid) {
                result.push(process.clone());
                pending.push((process.pid, process.created_at));
            }
        }
    }
    result
}

fn get_process_mem(handle: HANDLE) -> Option<(usize, usize, Option<usize>)> {
    unsafe {
        let mut extended: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
        extended.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
        if K32GetProcessMemoryInfo(
            handle,
            &mut extended as *mut _ as *mut _,
            extended.cb,
        ) != 0
        {
            return Some((
                extended.WorkingSetSize,
                extended.PrivateUsage,
                Some(extended.PrivateWorkingSetSize),
            ));
        }

        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        pmc.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

        if K32GetProcessMemoryInfo(
            handle,
            &mut pmc as *mut _ as *mut _,
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ) != 0
        {
            Some((pmc.WorkingSetSize, pmc.PrivateUsage, None))
        } else {
            None
        }
    }
}

pub fn measure_memory() -> Result<MemoryReport, String> {
    unsafe {
        let current_pid = GetCurrentProcessId();
        let current_handle = GetCurrentProcess();
        let mut root_created: FILETIME = std::mem::zeroed();
        let mut root_exited: FILETIME = std::mem::zeroed();
        let mut root_kernel: FILETIME = std::mem::zeroed();
        let mut root_user: FILETIME = std::mem::zeroed();
        if GetProcessTimes(current_handle, &mut root_created, &mut root_exited, &mut root_kernel, &mut root_user) == 0 {
            return Err("Failed to identify ORBIT process start time".to_string());
        }
        let root_created_at = ((root_created.dwHighDateTime as u64) << 32) | root_created.dwLowDateTime as u64;

        let (main_ws, main_private, main_private_ws) = get_process_mem(current_handle)
            .ok_or_else(|| "Failed to query memory info for main process".to_string())?;

        let main_process = ProcessItem {
            pid: current_pid,
            name: "orbit.exe".to_string(),
            working_set_bytes: main_ws,
            working_set_mb: (main_ws as f64) / (1024.0 * 1024.0),
            private_bytes: main_private,
            private_mb: (main_private as f64) / (1024.0 * 1024.0),
            private_working_set_mb: main_private_ws.map(|value| value as f64 / (1024.0 * 1024.0)),
        };

        let mut processes = Vec::new();
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("Failed to enumerate Windows processes".to_string());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) == FALSE {
            CloseHandle(snapshot);
            return Err("Failed to read Windows process list".to_string());
        }
        loop {
            let name_end = entry
                .szExeFile
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(entry.szExeFile.len());
            let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, entry.th32ProcessID);
            let mut created: FILETIME = std::mem::zeroed();
            let mut exited: FILETIME = std::mem::zeroed();
            let mut kernel: FILETIME = std::mem::zeroed();
            let mut user: FILETIME = std::mem::zeroed();
            let creation_time = if !process_handle.is_null() {
                let ok = GetProcessTimes(process_handle, &mut created, &mut exited, &mut kernel, &mut user);
                CloseHandle(process_handle);
                if ok != 0 { ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64 } else { 0 }
            } else { 0 };
            processes.push(ProcessInfo {
                pid: entry.th32ProcessID,
                parent_pid: entry.th32ParentProcessID,
                name: String::from_utf16_lossy(&entry.szExeFile[..name_end]),
                created_at: creation_time,
            });

            if Process32NextW(snapshot, &mut entry) == FALSE {
                break;
            }
        }
        CloseHandle(snapshot);

        let mut webview_processes = Vec::new();
        let mut unmeasured_webview_count = 0;
        let mut all_private_working_sets_available = main_private_ws.is_some();
        for process in descendants(&processes, current_pid, root_created_at) {
            // ORBIT's memory panel describes only its own WebView2 subprocesses. Other
            // descendants are excluded because summing unrelated helpers is misleading.
            if !process.name.eq_ignore_ascii_case("msedgewebview2.exe") {
                continue;
            }
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process.pid);
            if handle.is_null() {
                unmeasured_webview_count += 1;
                all_private_working_sets_available = false;
                continue;
            }
            if let Some((working_set_bytes, private_bytes, private_working_set_bytes)) =
                get_process_mem(handle)
            {
                if private_working_set_bytes.is_none() {
                    all_private_working_sets_available = false;
                }
                webview_processes.push(ProcessItem {
                    pid: process.pid,
                    name: process.name,
                    working_set_bytes,
                    working_set_mb: working_set_bytes as f64 / (1024.0 * 1024.0),
                    private_bytes,
                    private_mb: private_bytes as f64 / (1024.0 * 1024.0),
                    private_working_set_mb: private_working_set_bytes
                        .map(|value| value as f64 / (1024.0 * 1024.0)),
                });
            } else {
                unmeasured_webview_count += 1;
                all_private_working_sets_available = false;
            }
            CloseHandle(handle);
        }

        let combined_private_working_set_mb = all_private_working_sets_available.then(|| {
            main_private_ws.unwrap_or(0) as f64 / (1024.0 * 1024.0)
                + webview_processes
                    .iter()
                    .filter_map(|process| process.private_working_set_mb)
                    .sum::<f64>()
        });

        Ok(MemoryReport {
            main_process,
            webview_processes,
            unmeasured_webview_count,
            combined_private_working_set_mb,
            measured_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            measurement_method: "Windows Private Working Set. ORBIT and verified descendant msedgewebview2.exe processes are listed separately; unrelated child processes are excluded. The combined value is hidden if a WebView2 process could not be measured.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{descendants, ProcessInfo};

    #[test]
    fn includes_nested_children_and_excludes_unrelated_processes() {
        let processes = vec![
            ProcessInfo { pid: 2, parent_pid: 1, name: "webview".into(), created_at: 20 },
            ProcessInfo { pid: 3, parent_pid: 2, name: "renderer".into(), created_at: 30 },
            ProcessInfo { pid: 4, parent_pid: 99, name: "unrelated".into(), created_at: 30 },
        ];
        let mut ids = descendants(&processes, 1, 10)
            .into_iter()
            .map(|process| process.pid)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec![2, 3]);
    }

    #[test]
    fn rejects_reused_parent_pid_from_an_older_process() {
        let processes = vec![
            ProcessInfo { pid: 2, parent_pid: 1, name: "stale".into(), created_at: 4 },
        ];
        assert!(descendants(&processes, 1, 10).is_empty());
    }
}
