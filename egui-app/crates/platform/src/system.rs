// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied and adapted from nook/crates/platform/src/system.rs.

//! System preferences, locale clock/date, metrics, and OS version info.

use crate::registry::{self, Hive};
use windows::Win32::Globalization::{
    ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx, GetTimeFormatEx, TIME_NOSECONDS,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::{
    SM_CXSMICON, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};
use windows::core::{BOOL, PCWSTR, w};

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// Windows "app mode" is light.
pub fn apps_use_light_theme() -> bool {
    registry::read_dword(Hive::CurrentUser, PERSONALIZE, "AppsUseLightTheme").unwrap_or(0) != 0
}

/// "Transparency effects" toggle in Settings > Personalization > Colors.
pub fn transparency_enabled() -> bool {
    registry::read_dword(Hive::CurrentUser, PERSONALIZE, "EnableTransparency").unwrap_or(1) != 0
}

/// Pixel size of a small tray icon at the system DPI.
pub fn small_icon_size() -> u32 {
    // SAFETY: plain metric queries with no pointers.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    if px > 0 { px as u32 } else { 16 }
}

/// "Animation effects" (Settings > Accessibility > Visual effects).
pub fn animations_enabled() -> bool {
    let mut on = BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes a BOOL.
    let res = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut on as *mut BOOL as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    res.is_err() || on.as_bool()
}

/// Windows build number, e.g. 26100. 0 if unknown.
pub fn windows_build() -> u32 {
    registry::read_string(
        Hive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0)
}

/// Human-readable Windows version string for the startup diagnostics log.
pub fn windows_version() -> String {
    const KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let read = |name| registry::read_string(Hive::LocalMachine, KEY, name).unwrap_or_default();
    let build = windows_build();
    let ubr = registry::read_dword(Hive::LocalMachine, KEY, "UBR").unwrap_or(0);
    let product = read("ProductName");
    let product = if build >= 22000 {
        product.replacen("Windows 10", "Windows 11", 1)
    } else {
        product
    };
    format!("{product} {} ({build}.{ubr})", read("DisplayVersion"))
}

/// Local date and time for crash log entries.
pub fn file_timestamp() -> String {
    // SAFETY: plain query returning a value.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}-{:02}-{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

/// Local time and date formatted with the user's Windows locale settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    pub time: String,
    pub date: String,
    /// Milliseconds until just after the minute rolls over (+50 ms cushion so
    /// the UI never wakes 1 ms early and has to schedule a second tick).
    pub millis_to_next_minute: u64,
}

pub fn clock() -> Clock {
    let read = |buf: &[u16], n: i32| {
        let n = (n.max(1) as usize - 1).min(buf.len());
        String::from_utf16_lossy(&buf[..n])
    };
    // SAFETY: buffers are sized and SYSTEMTIME is a local value; null locale
    // means the user's Windows default locale.
    unsafe {
        let st = GetLocalTime();
        let mut tbuf = [0u16; 32];
        let n = GetTimeFormatEx(
            PCWSTR::null(),
            TIME_NOSECONDS,
            Some(&st),
            PCWSTR::null(),
            Some(&mut tbuf),
        );
        let time = if n > 0 {
            read(&tbuf, n)
        } else {
            format!("{:02}:{:02}", st.wHour, st.wMinute)
        };
        let mut dbuf = [0u16; 32];
        let n = GetDateFormatEx(
            PCWSTR::null(),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&st),
            w!("ddd d"),
            Some(&mut dbuf),
            PCWSTR::null(),
        );
        let date = if n > 0 {
            read(&dbuf, n)
        } else {
            st.wDay.to_string()
        };
        let ms_into_min =
            u64::from(st.wSecond.min(59)) * 1000 + u64::from(st.wMilliseconds.min(999));
        let millis_to_next_minute = (60_000 - ms_into_min) + 50;
        Clock {
            time,
            date,
            millis_to_next_minute,
        }
    }
}
