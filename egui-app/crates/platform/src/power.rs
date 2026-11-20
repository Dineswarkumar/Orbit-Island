// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/platform/src/system.rs (battery queries).

//! Power and battery status.

use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// `None` on machines without a battery.
pub fn battery() -> Option<Battery> {
    let mut st = SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out pointer.
    unsafe { GetSystemPowerStatus(&mut st) }.ok()?;
    // 128 = no system battery, 255 = unknown status.
    if st.BatteryFlag & 128 != 0 || st.BatteryFlag == 255 || st.BatteryLifePercent > 100 {
        return None;
    }
    Some(Battery {
        percent: st.BatteryLifePercent,
        charging: st.ACLineStatus == 1,
    })
}

/// Battery saver / Energy saver is active.
pub fn battery_saver() -> bool {
    let mut st = SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out pointer.
    unsafe { GetSystemPowerStatus(&mut st) }.is_ok() && st.SystemStatusFlag == 1
}
