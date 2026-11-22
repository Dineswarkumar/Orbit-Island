// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied from nook/crates/platform/src/process.rs.

//! Process-level housekeeping.

use crate::{PlatformError, Result};
use windows::Win32::System::ProcessStatus::{
    K32EmptyWorkingSet, K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::Threading::GetCurrentProcess;

/// Releases this process's working set back to the OS standby list.
///
/// Called only on `GetCurrentProcess()` after 3 s of idle, never while the
/// pointer is over the island, and never on any other process.
pub fn trim_working_set() -> Result<()> {
    // SAFETY: pseudo-handle of the current process only.
    if unsafe { K32EmptyWorkingSet(GetCurrentProcess()) }.as_bool() {
        Ok(())
    } else {
        Err(PlatformError::Other("K32EmptyWorkingSet failed".into()))
    }
}

/// Returns this process's current working set in bytes, or `None` if the query fails.
pub fn current_working_set_bytes() -> Option<u64> {
    let mut pmc = PROCESS_MEMORY_COUNTERS::default();
    pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: querying memory counters for the current process pseudo-handle.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) }.as_bool();
    ok.then_some(pmc.WorkingSetSize as u64)
}

#[cfg(feature = "bughunt-probe")]
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessCounters {
    pub tid: u32,
    pub private_usage_bytes: u64,
    pub working_set_bytes: u64,
    pub page_fault_count: u32,
    pub handle_count: u32,
    pub gdi_objects: u32,
    pub user_objects: u32,
    pub thread_count: u32,
}

#[cfg(feature = "bughunt-probe")]
pub fn capture_counters() -> ProcessCounters {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::{
        GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess as SysCurrentProcess,
        GetCurrentProcessId, GetCurrentThreadId, GetGuiResources, GetProcessHandleCount,
    };

    // SAFETY: queries current process/thread diagnostic counters via documented Win32 APIs.
    unsafe {
        let proc_h = SysCurrentProcess();
        let pid = GetCurrentProcessId();
        let tid = GetCurrentThreadId();

        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let mem_ok = K32GetProcessMemoryInfo(
            proc_h,
            (&mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            pmc.cb,
        ) != 0;

        let mut handle_count = 0u32;
        let _ = GetProcessHandleCount(proc_h, &mut handle_count);
        let gdi_objects = GetGuiResources(proc_h, GR_GDIOBJECTS);
        let user_objects = GetGuiResources(proc_h, GR_USEROBJECTS);

        let mut thread_count = 0u32;
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if !snap.is_null() && snap != (-1isize as *mut core::ffi::c_void) {
            let mut pe: PROCESSENTRY32W = std::mem::zeroed();
            pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(snap, &mut pe) != 0 {
                loop {
                    if pe.th32ProcessID == pid {
                        thread_count = pe.cntThreads;
                        break;
                    }
                    if Process32NextW(snap, &mut pe) == 0 {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }

        ProcessCounters {
            tid,
            private_usage_bytes: if mem_ok { pmc.PrivateUsage as u64 } else { 0 },
            working_set_bytes: if mem_ok { pmc.WorkingSetSize as u64 } else { 0 },
            page_fault_count: if mem_ok { pmc.PageFaultCount } else { 0 },
            handle_count,
            gdi_objects,
            user_objects,
            thread_count,
        }
    }
}
