// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/theme.rs (font loading separated into platform).

//! Loads Segoe UI fonts from `%WINDIR%\Fonts` so the UI crate performs no I/O.

pub struct FontBytes {
    pub regular: Option<Vec<u8>>,
    pub semibold: Option<Vec<u8>>,
}

/// Loads `segoeui.ttf` and `seguisb.ttf` from `%WINDIR%\Fonts`.
/// Returns `None` entries if missing so the UI falls back cleanly.
pub fn load_segoe_ui() -> FontBytes {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let load = |file: &str| std::fs::read(format!("{windir}\\Fonts\\{file}")).ok();
    FontBytes {
        regular: load("segoeui.ttf"),
        semibold: load("seguisb.ttf"),
    }
}
