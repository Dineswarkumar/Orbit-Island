// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied from nook/crates/platform/src/registry.rs.

//! Minimal registry access (values only, no key enumeration).

use crate::wide;
use std::path::Path;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW,
    RegOpenKeyExW, RegSetValueExW,
};
use windows::core::PCWSTR;

pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUN_VALUE_NAME: &str = "ORBIT";

#[derive(Clone, Copy, Debug)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

impl Hive {
    fn key(self) -> HKEY {
        match self {
            Hive::CurrentUser => HKEY_CURRENT_USER,
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }
}

/// Reads a `REG_DWORD`; `None` if missing or of another type.
pub fn read_dword(hive: Hive, subkey: &str, value: &str) -> Option<u32> {
    let (k, v) = (wide(subkey), wide(value));
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: data/size point at a u32 we own.
    let err = unsafe {
        RegGetValueW(
            hive.key(),
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    err.is_ok().then_some(data)
}

/// Reads a `REG_SZ`; `None` if missing.
pub fn read_string(hive: Hive, subkey: &str, value: &str) -> Option<String> {
    let (k, v) = (wide(subkey), wide(value));
    let mut size = 0u32;
    // SAFETY: first call only queries the size.
    let err = unsafe {
        RegGetValueW(
            hive.key(),
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    if err.is_err() || size == 0 {
        return None;
    }
    let mut buf = vec![0u16; (size as usize).div_ceil(2)];
    // SAFETY: buf holds `size` bytes.
    let err = unsafe {
        RegGetValueW(
            hive.key(),
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if err.is_err() {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

pub fn format_startup_command(exe_path: &Path) -> String {
    format!("\"{}\"", exe_path.display())
}

/// Registers or removes ORBIT under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
pub fn sync_startup_registry(enabled: bool, exe_path: &Path) -> Result<(), String> {
    let subkey = wide(RUN_SUBKEY);
    let value_name = wide(RUN_VALUE_NAME);
    let mut hkey = HKEY::default();

    if enabled {
        // SAFETY: Valid HKEY_CURRENT_USER + NUL-terminated UTF-16 subkey; handle closed below.
        let err = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut hkey,
                None,
            )
        };
        if err != ERROR_SUCCESS {
            return Err(format!("Failed to open startup registry key ({})", err.0));
        }
        let cmd = format_startup_command(exe_path);
        let cmd_bytes: Vec<u8> = wide(&cmd).into_iter().flat_map(u16::to_le_bytes).collect();
        // SAFETY: Open key handle with KEY_SET_VALUE and NUL-terminated REG_SZ UTF-16LE bytes.
        let set_err = unsafe {
            RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                None,
                REG_SZ,
                Some(&cmd_bytes),
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        if set_err != ERROR_SUCCESS {
            return Err(format!("Failed to set startup registry entry ({})", set_err.0));
        }
        Ok(())
    } else {
        // SAFETY: Valid HKEY_CURRENT_USER + NUL-terminated UTF-16 subkey; handle closed below.
        let err = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                KEY_SET_VALUE,
                &mut hkey,
            )
        };
        if err == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        if err != ERROR_SUCCESS {
            return Err(format!("Failed to open startup registry key ({})", err.0));
        }
        // SAFETY: Open key handle with KEY_SET_VALUE and NUL-terminated value name.
        let del_err = unsafe { RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr())) };
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        if del_err != ERROR_SUCCESS && del_err != ERROR_FILE_NOT_FOUND {
            return Err(format!("Failed to remove startup registry entry ({})", del_err.0));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_startup_command_quotes_path_with_spaces() {
        let p = Path::new(r"C:\Program Files\ORBIT\orbit-egui.exe");
        assert_eq!(
            format_startup_command(p),
            "\"C:\\Program Files\\ORBIT\\orbit-egui.exe\""
        );
    }
}
