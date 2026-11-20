// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/platform/src/lib.rs.

//! Thin wrappers over the Win32 and DWM APIs ORBIT uses.
//! No egui types appear in this crate.

pub mod crash;
pub mod fonts;
pub mod hittest;
pub mod material;
pub mod monitor;
pub mod power;
pub mod process;
pub mod registry;
pub mod single_instance;
pub mod system;
pub mod window;

pub use window::Hwnd;

/// A failed Windows API call.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("{call} failed: {source}")]
    Win32 {
        call: &'static str,
        #[source]
        source: windows::core::Error,
    },
    #[error("{call} failed with Win32 error {code}")]
    Code { call: &'static str, code: u32 },
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Attaches the API name to a `windows::core::Result`.
pub(crate) trait Win32Context<T> {
    fn api(self, call: &'static str) -> Result<T>;
}

impl<T> Win32Context<T> for windows::core::Result<T> {
    fn api(self, call: &'static str) -> Result<T> {
        self.map_err(|source| PlatformError::Win32 { call, source })
    }
}

/// Converts a `WIN32_ERROR` return code into a `Result`.
#[allow(dead_code)]
pub(crate) fn check(
    call: &'static str,
    err: windows::Win32::Foundation::WIN32_ERROR,
) -> Result<()> {
    if err.is_ok() {
        Ok(())
    } else {
        Err(PlatformError::Code { call, code: err.0 })
    }
}

/// Null-terminated UTF-16 for `PCWSTR` parameters.
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
