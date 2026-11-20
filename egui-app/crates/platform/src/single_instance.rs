// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied from nook/crates/platform/src/single_instance.rs.

//! Single-instance guard via a named session mutex.

use crate::{Result, Win32Context, wide};
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

/// Holds the named mutex for the life of the process.
pub struct InstanceGuard(HANDLE);

// SAFETY: a mutex handle is a process-wide kernel handle.
unsafe impl Send for InstanceGuard {}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // SAFETY: we own the handle.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// `Ok(None)` if another instance already runs.
/// Passes `Local\<name>` to `CreateMutexW`.
pub fn acquire(name: &str) -> Result<Option<InstanceGuard>> {
    let mutex_name = if name.starts_with("Local\\") {
        name.to_owned()
    } else {
        format!("Local\\{name}")
    };
    let name = wide(&mutex_name);
    // SAFETY: valid name buffer; handle is closed by the guard.
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.api("CreateMutexW")?;
    // SAFETY: reads the thread's last error right after CreateMutexW.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: we own this handle copy.
        let _ = unsafe { CloseHandle(handle) };
        return Ok(None);
    }
    Ok(Some(InstanceGuard(handle)))
}
