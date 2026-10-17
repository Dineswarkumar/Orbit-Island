use serde::Serialize;
use std::collections::HashSet;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
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
    pub child_processes: Vec<ProcessItem>,
    pub unmeasured_descendant_count: usize,
    pub total_working_set_mb: f64,
    pub total_private_mb: f64,
    pub total_private_working_set_mb: Option<f64>,
    pub aspiration_mb: f64, // < 30 MB
    pub target_mb: f64,     // < 50 MB
    pub maximum_mb: f64,    // < 100 MB
    pub meets_aspiration: bool,
    pub meets_target: bool,
    pub meets_maximum: bool,
    pub measurement_method: String,
}

#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
    parent_pid: u32,
    name: String,
}

fn descendants(processes: &[ProcessInfo], root_pid: u32) -> Vec<ProcessInfo> {
    let mut result = Vec::new();
    let mut pending = vec![root_pid];
    let mut seen = HashSet::from([root_pid]);

    while let Some(parent_pid) = pending.pop() {
        for process in processes.iter().filter(|process| process.parent_pid == parent_pid) {
            if seen.insert(process.pid) {
                result.push(process.clone());
                pending.push(process.pid);
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
            processes.push(ProcessInfo {
                pid: entry.th32ProcessID,
                parent_pid: entry.th32ParentProcessID,
                name: String::from_utf16_lossy(&entry.szExeFile[..name_end]),
            });

            if Process32NextW(snapshot, &mut entry) == FALSE {
                break;
            }
        }
        CloseHandle(snapshot);

        let mut child_processes = Vec::new();
        let mut unmeasured_descendant_count = 0;
        let mut all_private_working_sets_available = main_private_ws.is_some();
        for process in descendants(&processes, current_pid) {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process.pid);
            if handle.is_null() {
                unmeasured_descendant_count += 1;
                continue;
            }
            if let Some((working_set_bytes, private_bytes, private_working_set_bytes)) =
                get_process_mem(handle)
            {
                if private_working_set_bytes.is_none() {
                    all_private_working_sets_available = false;
                }
                child_processes.push(ProcessItem {
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
                unmeasured_descendant_count += 1;
                all_private_working_sets_available = false;
            }
            CloseHandle(handle);
        }

        let total_ws_bytes = main_process.working_set_bytes
            + child_processes
                .iter()
                .map(|p| p.working_set_bytes)
                .sum::<usize>();

        let total_private_bytes = main_process.private_bytes
            + child_processes
                .iter()
                .map(|p| p.private_bytes)
                .sum::<usize>();

        let total_working_set_mb = (total_ws_bytes as f64) / (1024.0 * 1024.0);
        let total_private_mb = (total_private_bytes as f64) / (1024.0 * 1024.0);
        let total_private_working_set_mb = all_private_working_sets_available.then(|| {
            main_private_ws.unwrap_or(0) as f64 / (1024.0 * 1024.0)
                + child_processes
                    .iter()
                    .filter_map(|process| process.private_working_set_mb)
                    .sum::<f64>()
        });

        Ok(MemoryReport {
            main_process,
            child_processes,
            unmeasured_descendant_count,
            total_working_set_mb,
            total_private_mb,
            total_private_working_set_mb,
            aspiration_mb: 30.0,
            target_mb: 50.0,
            maximum_mb: 100.0,
            meets_aspiration: total_private_working_set_mb.is_some_and(|mb| mb < 30.0),
            meets_target: total_private_working_set_mb.is_some_and(|mb| mb < 50.0),
            meets_maximum: total_private_working_set_mb.is_some_and(|mb| mb < 100.0),
            measurement_method: "Win32 K32GetProcessMemoryInfo: PROCESS_MEMORY_COUNTERS_EX2.PrivateWorkingSetSize is the target metric; PrivateUsage (Private Bytes/commit charge) and WorkingSetSize (total resident working set) are reported separately. All accessible ORBIT descendants are included; unavailable measurements are marked incomplete.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{descendants, ProcessInfo};

    #[test]
    fn includes_nested_children_and_excludes_unrelated_processes() {
        let processes = vec![
            ProcessInfo { pid: 2, parent_pid: 1, name: "webview".into() },
            ProcessInfo { pid: 3, parent_pid: 2, name: "renderer".into() },
            ProcessInfo { pid: 4, parent_pid: 99, name: "unrelated".into() },
        ];
        let mut ids = descendants(&processes, 1)
            .into_iter()
            .map(|process| process.pid)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec![2, 3]);
    }
}
