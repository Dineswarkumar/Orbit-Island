// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied from nook/crates/platform/src/material.rs.

//! DWM window attributes for the overlay.

use crate::window::Hwnd;
use windows::Win32::Graphics::Dwm::{
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DwmSetWindowAttribute,
};
use windows::core::BOOL;

/// Square DWM corners (the shape outline does the rounding) and dark-mode hint.
pub fn prepare(hwnd: Hwnd, dark: bool) {
    let pref = DWMWCP_DONOTROUND;
    // SAFETY: attribute and size match.
    if let Err(e) = unsafe {
        DwmSetWindowAttribute(
            hwnd.get(),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const _,
            std::mem::size_of_val(&pref) as u32,
        )
    } {
        tracing::debug!("DWMWA_WINDOW_CORNER_PREFERENCE: {e}");
    }
    let dark = BOOL::from(dark);
    // SAFETY: attribute and size match.
    if let Err(e) = unsafe {
        DwmSetWindowAttribute(
            hwnd.get(),
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        )
    } {
        tracing::debug!("DWMWA_USE_IMMERSIVE_DARK_MODE: {e}");
    }
}
