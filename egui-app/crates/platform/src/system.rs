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
    pub time_digits: String,
    pub ampm: String,
    pub date: String,
    pub full_date: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub weekday: u32,
    /// Milliseconds until just after the minute rolls over (+50 ms cushion so
    /// the UI never wakes 1 ms early and has to schedule a second tick).
    pub millis_to_next_minute: u64,
}

fn split_time_ampm(time: &str) -> (String, String) {
    let trimmed = time.trim();
    if let Some((digits, suffix)) = trimmed.rsplit_once(|c: char| c.is_whitespace()) {
        let up = suffix.trim().to_ascii_uppercase();
        if up == "AM" || up == "PM" {
            return (digits.trim_end().to_string(), up);
        }
    }
    (trimmed.to_string(), String::new())
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
        let (time_digits, ampm) = split_time_ampm(&time);

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

        let mut fbuf = [0u16; 64];
        let fn_len = GetDateFormatEx(
            PCWSTR::null(),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&st),
            w!("dddd, d MMMM"),
            Some(&mut fbuf),
            PCWSTR::null(),
        );
        let full_date = if fn_len > 0 {
            read(&fbuf, fn_len)
        } else {
            date.clone()
        };

        let ms_into_min =
            u64::from(st.wSecond.min(59)) * 1000 + u64::from(st.wMilliseconds.min(999));
        let millis_to_next_minute = (60_000 - ms_into_min) + 50;
        Clock {
            time,
            time_digits,
            ampm,
            date,
            full_date,
            year: i32::from(st.wYear),
            month: u32::from(st.wMonth),
            day: u32::from(st.wDay),
            weekday: u32::from(st.wDayOfWeek),
            millis_to_next_minute,
        }
    }
}

/// Fail-closed lookup for trusted Windows binaries strictly under `%SystemRoot%`.
/// Never falls back to a bare PATH name.
pub fn system_root_binary(subpath: &str) -> std::result::Result<std::path::PathBuf, String> {
    let trimmed = subpath.trim();
    if trimmed.is_empty() || trimmed.contains("..") {
        return Err(format!("Invalid system binary subpath: '{subpath}'"));
    }
    let system_root = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("WINDIR"))
        .unwrap_or_else(|_| r"C:\Windows".to_string());
    let candidate = std::path::Path::new(&system_root).join(trimmed);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "Required system binary not found under SystemRoot: {}",
            candidate.display()
        ))
    }
}

/// Strictly opens `http://` or `https://` URLs in the system default browser via `ShellExecuteW`.
pub fn open_external_url(url: &str) -> std::result::Result<(), String> {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err("Only http:// and https:// URLs are allowed.".to_string());
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("URL contains invalid control characters.".to_string());
    }
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = trimmed.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: Validated null-terminated HTTP(S) URL passed to ShellExecuteW.
    let res = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if (res.0 as isize) <= 32 {
        Err(format!("Failed to open URL (code: {:?}).", res.0 as isize))
    } else {
        Ok(())
    }
}

/// Opens a native Win32 multi-select file picker via COM `IFileOpenDialog` (no child process).
pub fn pick_files_native() -> std::result::Result<Vec<String>, String> {
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::UI::Shell::{
        FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog,
        IFileOpenDialog, SIGDN_FILESYSPATH,
    };

    const HRESULT_CANCELLED: i32 = 0x8007_04C7u32 as i32;

    // SAFETY: Balanced COM apartment initialization for modal IFileOpenDialog.
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let should_uninit = init.is_ok();

        let result = (|| -> std::result::Result<Vec<String>, String> {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("Failed to create FileOpenDialog: {e}"))?;
            let opts = dialog
                .GetOptions()
                .map_err(|e| format!("GetOptions failed: {e}"))?;
            dialog
                .SetOptions(opts | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM)
                .map_err(|e| format!("SetOptions failed: {e}"))?;

            if let Err(err) = dialog.Show(None) {
                if err.code().0 == HRESULT_CANCELLED {
                    return Ok(Vec::new());
                }
                return Err(format!("File dialog failed: {err}"));
            }

            let array = dialog
                .GetResults()
                .map_err(|e| format!("GetResults failed: {e}"))?;
            let count = array
                .GetCount()
                .map_err(|e| format!("GetCount failed: {e}"))?;
            let mut paths = Vec::with_capacity(count as usize);
            for i in 0..count {
                if let Ok(item) = array.GetItemAt(i)
                    && let Ok(pw) = item.GetDisplayName(SIGDN_FILESYSPATH)
                {
                    if let Ok(s) = pw.to_string()
                        && !s.is_empty()
                    {
                        paths.push(s);
                    }
                    CoTaskMemFree(Some(pw.0 as *const core::ffi::c_void));
                }
            }
            Ok(paths)
        })();

        if should_uninit {
            CoUninitialize();
        }
        result
    }
}

/// Opens a folder (or reveals a file) in Windows Explorer using fail-closed `SystemRoot\explorer.exe`.
pub fn reveal_dir_in_explorer(dir: &std::path::Path) -> std::result::Result<(), String> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create folder {}: {e}", dir.display()))?;
    }
    let canon = dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve folder {}: {e}", dir.display()))?;
    let path_str = canon.to_string_lossy();
    let trimmed = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
    let explorer = system_root_binary("explorer.exe")?;
    std::process::Command::new(explorer)
        .arg(trimmed)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch File Explorer: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_root_binary_fails_closed_for_missing_file() {
        let err = system_root_binary(r"System32\definitely_nonexistent_orbit_binary_12345.exe")
            .unwrap_err();
        assert!(err.contains("not found under SystemRoot"), "got: {err}");
    }

    #[test]
    fn system_root_binary_rejects_traversal_and_empty() {
        assert!(system_root_binary("").is_err());
        assert!(system_root_binary(r"..\cmd.exe").is_err());
    }

    #[test]
    fn system_root_binary_resolves_explorer() {
        let p = system_root_binary("explorer.exe").expect("explorer.exe should exist");
        assert!(p.is_absolute());
        assert!(p.is_file());
    }

    #[test]
    fn clock_returns_valid_date_parts() {
        let c = clock();
        assert!(c.year >= 2024, "unexpected year {}", c.year);
        assert!((1..=12).contains(&c.month), "unexpected month {}", c.month);
        assert!((1..=31).contains(&c.day), "unexpected day {}", c.day);
        assert!((0..=6).contains(&c.weekday), "unexpected weekday {}", c.weekday);
        assert!(!c.time_digits.is_empty());
        assert!(!c.full_date.is_empty());
    }

    #[test]
    fn split_time_ampm_handles_12h_and_24h() {
        assert_eq!(split_time_ampm("9:06 PM"), ("9:06".into(), "PM".into()));
        assert_eq!(
            split_time_ampm("12:00\u{202f}am"),
            ("12:00".into(), "AM".into())
        );
        assert_eq!(split_time_ampm("21:06"), ("21:06".into(), "".into()));
    }

    #[test]
    fn open_external_url_rejects_unsafe_schemes() {
        assert!(open_external_url("javascript:alert(1)").is_err());
        assert!(open_external_url("file:///C:/Windows/win.ini").is_err());
        assert!(open_external_url("cmd://calc").is_err());
    }
}
