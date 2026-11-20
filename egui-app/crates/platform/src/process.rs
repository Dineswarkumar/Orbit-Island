// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied from nook/crates/platform/src/process.rs.

//! Process-level housekeeping.

use crate::{PlatformError, Result};
use windows::Win32::System::ProcessStatus::K32EmptyWorkingSet;
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
