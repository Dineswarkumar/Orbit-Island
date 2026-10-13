use serde::Serialize;
use std::mem::size_of;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReport {
    pub main_process: ProcessItem,
    pub child_processes: Vec<ProcessItem>,
    pub total_working_set_mb: f64,
    pub total_private_mb: f64,
    pub meets_normal_goal: bool, // <= 50 MB
    pub meets_maximum_limit: bool, // <= 100 MB
    pub measurement_method: String,
}

fn get_process_mem(handle: HANDLE) -> Option<(usize, usize)> {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        pmc.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

        if K32GetProcessMemoryInfo(
            handle,
            &mut pmc as *mut _ as *mut _,
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ) != 0
        {
            Some((pmc.WorkingSetSize, pmc.PrivateUsage))
        } else {
            None
        }
    }
}

pub fn measure_memory() -> Result<MemoryReport, String> {
    unsafe {
        let current_pid = GetCurrentProcessId();
        let current_handle = GetCurrentProcess();

        let (main_ws, main_private) = get_process_mem(current_handle)
            .ok_or_else(|| "Failed to query memory info for main process".to_string())?;

        let main_process = ProcessItem {
            pid: current_pid,
            name: "orbit.exe".to_string(),
            working_set_bytes: main_ws,
            working_set_mb: (main_ws as f64) / (1024.0 * 1024.0),
            private_bytes: main_private,
            private_mb: (main_private as f64) / (1024.0 * 1024.0),
        };

        let mut child_processes = Vec::new();

        // Enumerate child processes via Toolhelp snapshot
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot != INVALID_HANDLE_VALUE {
            let mut entry: PROCESSENTRY32 = std::mem::zeroed();
            entry.dwSize = size_of::<PROCESSENTRY32>() as u32;

            if Process32First(snapshot, &mut entry) != FALSE {
                loop {
                    if entry.th32ParentProcessID == current_pid {
                        let child_pid = entry.th32ProcessID;
                        let child_handle =
                            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, child_pid);

                        if !child_handle.is_null() {
                            if let Some((child_ws, child_private)) = get_process_mem(child_handle) {
                                let name = String::from_utf8_lossy(
                                    &entry
                                        .szExeFile
                                        .iter()
                                        .take_while(|&&c| c != 0)
                                        .map(|&c| c as u8)
                                        .collect::<Vec<u8>>(),
                                )
                                .to_string();

                                child_processes.push(ProcessItem {
                                    pid: child_pid,
                                    name,
                                    working_set_bytes: child_ws,
                                    working_set_mb: (child_ws as f64) / (1024.0 * 1024.0),
                                    private_bytes: child_private,
                                    private_mb: (child_private as f64) / (1024.0 * 1024.0),
                                });
                            }
                            CloseHandle(child_handle);
                        }
                    }

                    if Process32Next(snapshot, &mut entry) == FALSE {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
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

        Ok(MemoryReport {
            main_process,
            child_processes,
            total_working_set_mb,
            total_private_mb,
            meets_normal_goal: total_private_mb <= 50.0,
            meets_maximum_limit: total_private_mb <= 100.0,
            measurement_method: "Win32 K32GetProcessMemoryInfo PrivateUsage across main orbit.exe process + child WebView2 processes".to_string(),
        })
    }
}
