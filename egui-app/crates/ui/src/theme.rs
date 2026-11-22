// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/theme.rs.

//! Design tokens and font installation (no direct file I/O in this crate).

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use orbit_core::config::{ColorTheme, GlassMode, ThemeMode};

pub const FALLBACK_ACCENT: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);

pub const SIZE_LABEL: f32 = 12.0;
pub const SIZE_BODY: f32 = 13.0;
pub const SIZE_PILL_TIME: f32 = 19.5;

pub const FAMILY_SEMIBOLD: &str = "orbit-semibold";
pub const FAMILY_BOLD: &str = "orbit-bold";
pub const FAMILY_ICON_FILL: &str = "phosphor-fill";
pub const FAMILY_ICON_REGULAR: &str = "phosphor";

const DARK_TEXT: Color32 = Color32::WHITE;
const LIGHT_TEXT: Color32 = Color32::from_rgb(10, 15, 28);

/// Font bytes passed in from `orbit_platform::fonts`.
pub struct FontAssets {
    pub regular: Option<Vec<u8>>,
    pub semibold: Option<Vec<u8>>,
    pub bold: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub is_light: bool,
    pub accent: Color32,
    pub accent_blue: Color32,
    pub accent_pink: Color32,
    pub accent_purple: Color32,
    pub accent_amber: Color32,
    pub accent_cyan: Color32,
    pub accent_green: Color32,
    pub pill_top: Color32,
    pub pill_bottom: Color32,
    pub pill_hover_top: Color32,
    pub pill_hover_bottom: Color32,
    pub pill_rim: Color32,
    pub pill_hover_rim: Color32,
    pub pill_inner_rim: Color32,
    pub pill_highlight: Color32,
    pub pill_hover_highlight: Color32,
    pub pill_shadow: Color32,
    pub pill_text: Color32,
    pub pill_text_dim: Color32,
    pub opaque_surface: Color32,
    pub opaque_border: Color32,
    pub glass_fallback_surface: Color32,
    pub card_bg: Color32,
    pub card_top: Color32,
    pub card_bottom: Color32,
    pub card_border: Color32,
    pub card_highlight: Color32,
    pub core_glow: Color32,
    pub track_bg: Color32,
    pub pill_btn_bg: Color32,
    pub pill_btn_border: Color32,
    pub noise_alpha: u8,
    pub grain_px: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(FALLBACK_ACCENT)
    }
}

impl Theme {
    pub fn new(accent: Color32) -> Self {
        Self {
            is_light: false,
            accent,
            accent_blue: Color32::from_rgb(38, 132, 255),
            accent_pink: Color32::from_rgb(236, 72, 153),
            accent_purple: Color32::from_rgb(168, 85, 247),
            accent_amber: Color32::from_rgb(245, 158, 11),
            accent_cyan: Color32::from_rgb(56, 189, 248),
            accent_green: Color32::from_rgb(52, 211, 153),
            pill_top: Color32::from_rgba_unmultiplied(30, 32, 38, 248),
            pill_bottom: Color32::from_rgba_unmultiplied(6, 7, 10, 252),
            pill_hover_top: Color32::from_rgba_unmultiplied(42, 45, 54, 250),
            pill_hover_bottom: Color32::from_rgba_unmultiplied(12, 13, 18, 254),
            pill_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 34),
            pill_hover_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 64),
            pill_inner_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 16),
            pill_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 58),
            pill_hover_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 92),
            pill_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 120),
            pill_text: DARK_TEXT,
            pill_text_dim: Color32::from_rgb(212, 222, 238),
            opaque_surface: Color32::from_rgb(20, 22, 27),
            opaque_border: Color32::from_rgb(38, 41, 48),
            glass_fallback_surface: Color32::from_rgba_unmultiplied(24, 29, 38, 238),
            card_bg: Color32::from_rgba_unmultiplied(22, 24, 30, 215),
            card_top: Color32::from_rgba_unmultiplied(34, 37, 46, 226),
            card_bottom: Color32::from_rgba_unmultiplied(16, 18, 24, 234),
            card_border: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            card_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 32),
            core_glow: with_alpha(accent, 0.35),
            track_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 22),
            pill_btn_bg: Color32::from_rgba_unmultiplied(20, 28, 44, 210),
            pill_btn_border: Color32::from_rgba_unmultiplied(59, 130, 246, 72),
            noise_alpha: 6,
            grain_px: 1.0,
        }
    }

    pub fn light(accent: Color32) -> Self {
        Self {
            is_light: true,
            accent,
            accent_blue: Color32::from_rgb(24, 114, 238),
            accent_pink: Color32::from_rgb(219, 39, 119),
            accent_purple: Color32::from_rgb(126, 34, 206),
            accent_amber: Color32::from_rgb(217, 119, 6),
            accent_cyan: Color32::from_rgb(2, 132, 199),
            accent_green: Color32::from_rgb(5, 150, 105),
            pill_top: Color32::from_rgba_unmultiplied(250, 252, 255, 246),
            pill_bottom: Color32::from_rgba_unmultiplied(220, 228, 240, 248),
            pill_hover_top: Color32::from_rgba_unmultiplied(255, 255, 255, 250),
            pill_hover_bottom: Color32::from_rgba_unmultiplied(232, 238, 248, 252),
            pill_rim: Color32::from_rgba_unmultiplied(15, 23, 42, 42),
            pill_hover_rim: Color32::from_rgba_unmultiplied(15, 23, 42, 68),
            pill_inner_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 175),
            pill_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 210),
            pill_hover_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 240),
            pill_shadow: Color32::from_rgba_unmultiplied(15, 23, 42, 54),
            pill_text: LIGHT_TEXT,
            pill_text_dim: Color32::from_rgb(51, 65, 85),
            opaque_surface: Color32::from_rgb(245, 247, 250),
            opaque_border: Color32::from_rgb(203, 211, 222),
            glass_fallback_surface: Color32::from_rgba_unmultiplied(239, 245, 252, 238),
            card_bg: Color32::from_rgba_unmultiplied(236, 241, 248, 228),
            card_top: Color32::from_rgba_unmultiplied(249, 251, 255, 238),
            card_bottom: Color32::from_rgba_unmultiplied(226, 234, 245, 240),
            card_border: Color32::from_rgba_unmultiplied(15, 23, 42, 28),
            card_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 210),
            core_glow: with_alpha(accent, 0.28),
            track_bg: Color32::from_rgba_unmultiplied(15, 23, 42, 26),
            pill_btn_bg: Color32::from_rgba_unmultiplied(224, 235, 250, 225),
            pill_btn_border: Color32::from_rgba_unmultiplied(37, 99, 235, 68),
            noise_alpha: 4,
            grain_px: 1.0,
        }
    }

    pub fn is_light_effective(mode: ThemeMode, system_light: bool) -> bool {
        match mode {
            ThemeMode::Light => true,
            ThemeMode::Dark => false,
            ThemeMode::Auto => system_light,
        }
    }

    pub fn for_mode(mode: ThemeMode, system_light: bool, accent: Color32) -> Self {
        if Self::is_light_effective(mode, system_light) {
            Self::light(accent)
        } else {
            Self::new(accent)
        }
    }

    pub fn for_config(mode: ThemeMode, preset: ColorTheme, system_light: bool) -> Self {
        if preset == ColorTheme::CeramicPearl {
            return Self::ceramic_pearl();
        }

        let dark = Self::dark_preset(preset);
        let use_light_surface = mode == ThemeMode::Light
            || (mode == ThemeMode::Auto && system_light && preset == ColorTheme::Obsidian);

        if use_light_surface {
            Self::frosted_porcelain_from(&dark)
        } else {
            dark
        }
    }

    fn ceramic_pearl() -> Self {
        let accent = Color32::from_rgb(0x00, 0x66, 0xCC);
        let mut theme = Self::light(accent);
        theme.accent_blue = accent;
        theme.accent_cyan = Color32::from_rgb(14, 145, 226);
        theme.pill_top = Color32::from_rgba_unmultiplied(0xF1, 0xF5, 0xF9, 248);
        theme.pill_bottom = Color32::from_rgba_unmultiplied(0xDD, 0xE6, 0xF2, 250);
        theme.pill_hover_top = Color32::from_rgba_unmultiplied(252, 254, 255, 252);
        theme.pill_hover_bottom = Color32::from_rgba_unmultiplied(226, 234, 246, 252);
        theme.pill_rim = Color32::from_rgba_unmultiplied(15, 23, 42, 38);
        theme.pill_hover_rim = Color32::from_rgba_unmultiplied(0, 102, 204, 72);
        theme.pill_text = LIGHT_TEXT;
        theme.pill_text_dim = Color32::from_rgb(51, 65, 85);
        theme.opaque_surface = Color32::from_rgb(0xEB, 0xF0, 0xF7);
        theme.opaque_border = Color32::from_rgb(198, 209, 224);
        theme.glass_fallback_surface = Color32::from_rgba_unmultiplied(238, 244, 252, 240);
        theme.card_top = Color32::from_rgba_unmultiplied(249, 251, 255, 242);
        theme.card_bottom = Color32::from_rgba_unmultiplied(226, 234, 245, 242);
        theme.card_border = Color32::from_rgba_unmultiplied(15, 23, 42, 28);
        theme.card_highlight = Color32::from_rgba_unmultiplied(255, 255, 255, 220);
        theme.core_glow = with_alpha(accent, 0.28);
        theme.pill_btn_bg = Color32::from_rgba_unmultiplied(224, 236, 252, 232);
        theme.pill_btn_border = Color32::from_rgba_unmultiplied(0, 102, 204, 72);
        theme
    }

    fn frosted_porcelain_from(dark: &Self) -> Self {
        let mut light = Self::light(dark.accent);
        light.accent_blue = dark.accent_blue;
        light.accent_pink = dark.accent_pink;
        light.accent_purple = dark.accent_purple;
        light.accent_amber = dark.accent_amber;
        light.accent_cyan = dark.accent_cyan;
        light.accent_green = dark.accent_green;
        light.core_glow = with_alpha(dark.accent, 0.28);
        light.pill_top = tint_rgb(light.pill_top, dark.accent, 0.04);
        light.pill_bottom = tint_rgb(light.pill_bottom, dark.accent_purple, 0.05);
        light.pill_hover_top = tint_rgb(light.pill_hover_top, dark.accent, 0.05);
        light.pill_hover_bottom = tint_rgb(light.pill_hover_bottom, dark.accent_blue, 0.06);
        light.card_top = tint_rgb(light.card_top, dark.accent, 0.03);
        light.card_bottom = tint_rgb(light.card_bottom, dark.accent_blue, 0.05);
        light.pill_btn_bg = tint_rgb(light.pill_btn_bg, dark.accent_blue, 0.14);
        light.pill_btn_border = with_alpha(dark.accent_blue, 0.30);
        light.pill_text = LIGHT_TEXT;
        light
    }

    fn dark_preset(preset: ColorTheme) -> Self {
        match preset {
            ColorTheme::Obsidian => Self::new(FALLBACK_ACCENT),
            ColorTheme::OledBlack => {
                let accent = Color32::from_rgb(56, 189, 248);
                let mut t = Self::new(accent);
                t.pill_top = Color32::from_rgba_unmultiplied(15, 17, 22, 252);
                t.pill_bottom = Color32::from_rgba_unmultiplied(2, 3, 5, 254);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(24, 27, 34, 252);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(6, 7, 11, 255);
                t.pill_rim = Color32::from_rgba_unmultiplied(255, 255, 255, 38);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(56, 189, 248, 86);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
                t.opaque_surface = Color32::from_rgb(6, 7, 10);
                t.opaque_border = Color32::from_rgb(30, 34, 42);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(10, 12, 17, 246);
                t.card_bg = Color32::from_rgba_unmultiplied(12, 13, 18, 228);
                t.card_top = Color32::from_rgba_unmultiplied(20, 22, 29, 236);
                t.card_bottom = Color32::from_rgba_unmultiplied(6, 7, 10, 244);
                t.card_highlight = Color32::from_rgba_unmultiplied(255, 255, 255, 28);
                t.core_glow = with_alpha(accent, 0.36);
                t
            }
            ColorTheme::LinearIndigo => {
                let accent = Color32::from_rgb(99, 102, 241);
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(79, 124, 255);
                t.accent_purple = Color32::from_rgb(139, 92, 246);
                t.accent_cyan = Color32::from_rgb(56, 189, 248);
                t.pill_top = Color32::from_rgba_unmultiplied(26, 29, 46, 248);
                t.pill_bottom = Color32::from_rgba_unmultiplied(10, 12, 22, 252);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(36, 40, 62, 250);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(15, 18, 32, 254);
                t.pill_rim = Color32::from_rgba_unmultiplied(165, 180, 252, 42);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(165, 180, 252, 82);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(199, 210, 254, 22);
                t.opaque_surface = Color32::from_rgb(18, 20, 34);
                t.opaque_border = Color32::from_rgb(49, 54, 82);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(22, 25, 42, 240);
                t.card_bg = Color32::from_rgba_unmultiplied(22, 25, 42, 220);
                t.card_top = Color32::from_rgba_unmultiplied(34, 38, 62, 228);
                t.card_bottom = Color32::from_rgba_unmultiplied(16, 18, 32, 236);
                t.card_border = Color32::from_rgba_unmultiplied(165, 180, 252, 34);
                t.card_highlight = Color32::from_rgba_unmultiplied(199, 210, 254, 38);
                t.core_glow = with_alpha(accent, 0.42);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(32, 38, 72, 215);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(99, 102, 241, 88);
                t
            }
            ColorTheme::VisionSpatial => {
                let accent = Color32::from_rgb(96, 165, 250);
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(96, 165, 250);
                t.accent_cyan = Color32::from_rgb(103, 232, 249);
                t.accent_purple = Color32::from_rgb(192, 132, 252);
                t.pill_top = Color32::from_rgba_unmultiplied(44, 50, 64, 238);
                t.pill_bottom = Color32::from_rgba_unmultiplied(22, 26, 36, 244);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(58, 65, 82, 242);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(28, 33, 45, 248);
                t.pill_rim = Color32::from_rgba_unmultiplied(255, 255, 255, 58);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(255, 255, 255, 96);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(255, 255, 255, 34);
                t.pill_highlight = Color32::from_rgba_unmultiplied(255, 255, 255, 88);
                t.pill_hover_highlight = Color32::from_rgba_unmultiplied(255, 255, 255, 128);
                t.opaque_surface = Color32::from_rgb(33, 38, 50);
                t.opaque_border = Color32::from_rgb(72, 80, 98);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(36, 42, 56, 232);
                t.card_bg = Color32::from_rgba_unmultiplied(40, 46, 60, 210);
                t.card_top = Color32::from_rgba_unmultiplied(54, 62, 79, 218);
                t.card_bottom = Color32::from_rgba_unmultiplied(30, 34, 46, 228);
                t.card_border = Color32::from_rgba_unmultiplied(255, 255, 255, 44);
                t.card_highlight = Color32::from_rgba_unmultiplied(255, 255, 255, 60);
                t.core_glow = with_alpha(accent, 0.38);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(46, 58, 84, 205);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(147, 197, 253, 86);
                t
            }
            ColorTheme::CatppuccinMocha => {
                let accent = Color32::from_rgb(203, 166, 247); // Mauve
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(137, 180, 250);
                t.accent_pink = Color32::from_rgb(245, 194, 231);
                t.accent_purple = Color32::from_rgb(203, 166, 247);
                t.accent_amber = Color32::from_rgb(249, 226, 175);
                t.accent_cyan = Color32::from_rgb(148, 226, 213);
                t.accent_green = Color32::from_rgb(166, 227, 161);
                t.pill_top = Color32::from_rgba_unmultiplied(36, 39, 58, 248);
                t.pill_bottom = Color32::from_rgba_unmultiplied(17, 17, 27, 252);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(49, 50, 68, 250);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(24, 24, 37, 254);
                t.pill_rim = Color32::from_rgba_unmultiplied(203, 166, 247, 42);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(203, 166, 247, 82);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(205, 214, 244, 20);
                t.pill_text_dim = Color32::from_rgb(212, 220, 245);
                t.opaque_surface = Color32::from_rgb(30, 30, 46);
                t.opaque_border = Color32::from_rgb(69, 71, 90);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(30, 30, 46, 240);
                t.card_bg = Color32::from_rgba_unmultiplied(34, 36, 54, 222);
                t.card_top = Color32::from_rgba_unmultiplied(46, 48, 70, 230);
                t.card_bottom = Color32::from_rgba_unmultiplied(24, 24, 37, 236);
                t.card_border = Color32::from_rgba_unmultiplied(205, 214, 244, 30);
                t.card_highlight = Color32::from_rgba_unmultiplied(245, 224, 220, 36);
                t.core_glow = with_alpha(accent, 0.38);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(49, 50, 74, 218);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(203, 166, 247, 84);
                t
            }
            ColorTheme::Cyberpunk => {
                let accent = Color32::from_rgb(0, 229, 255);
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(0, 212, 255);
                t.accent_pink = Color32::from_rgb(255, 46, 147);
                t.accent_purple = Color32::from_rgb(184, 77, 255);
                t.accent_amber = Color32::from_rgb(250, 204, 21);
                t.accent_cyan = Color32::from_rgb(0, 240, 255);
                t.accent_green = Color32::from_rgb(34, 211, 238);
                t.pill_top = Color32::from_rgba_unmultiplied(28, 20, 44, 248);
                t.pill_bottom = Color32::from_rgba_unmultiplied(10, 8, 18, 253);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(40, 26, 62, 250);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(16, 11, 28, 254);
                t.pill_rim = Color32::from_rgba_unmultiplied(0, 229, 255, 58);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(255, 46, 147, 108);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(0, 229, 255, 26);
                t.pill_highlight = Color32::from_rgba_unmultiplied(180, 248, 255, 76);
                t.pill_hover_highlight = Color32::from_rgba_unmultiplied(255, 160, 214, 108);
                t.opaque_surface = Color32::from_rgb(18, 13, 30);
                t.opaque_border = Color32::from_rgb(66, 40, 96);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(22, 16, 36, 242);
                t.card_bg = Color32::from_rgba_unmultiplied(24, 17, 38, 224);
                t.card_top = Color32::from_rgba_unmultiplied(36, 25, 58, 232);
                t.card_bottom = Color32::from_rgba_unmultiplied(14, 10, 24, 238);
                t.card_border = Color32::from_rgba_unmultiplied(0, 229, 255, 44);
                t.card_highlight = Color32::from_rgba_unmultiplied(0, 240, 255, 48);
                t.core_glow = with_alpha(Color32::from_rgb(255, 46, 147), 0.38);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(42, 22, 64, 218);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(0, 229, 255, 96);
                t
            }
            ColorTheme::SunsetVelvet => {
                let accent = Color32::from_rgb(249, 115, 22);
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(96, 165, 250);
                t.accent_pink = Color32::from_rgb(251, 113, 133);
                t.accent_purple = Color32::from_rgb(192, 132, 252);
                t.accent_amber = Color32::from_rgb(251, 191, 36);
                t.accent_cyan = Color32::from_rgb(56, 189, 248);
                t.accent_green = Color32::from_rgb(52, 211, 153);
                t.pill_top = Color32::from_rgba_unmultiplied(40, 24, 38, 248);
                t.pill_bottom = Color32::from_rgba_unmultiplied(16, 9, 18, 252);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(54, 32, 50, 250);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(24, 13, 26, 254);
                t.pill_rim = Color32::from_rgba_unmultiplied(251, 146, 60, 48);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(251, 113, 133, 88);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(254, 215, 170, 22);
                t.pill_text_dim = Color32::from_rgb(228, 212, 222);
                t.opaque_surface = Color32::from_rgb(28, 16, 28);
                t.opaque_border = Color32::from_rgb(72, 42, 66);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(32, 19, 31, 240);
                t.card_bg = Color32::from_rgba_unmultiplied(34, 20, 33, 222);
                t.card_top = Color32::from_rgba_unmultiplied(49, 29, 46, 230);
                t.card_bottom = Color32::from_rgba_unmultiplied(21, 12, 23, 236);
                t.card_border = Color32::from_rgba_unmultiplied(251, 146, 60, 34);
                t.card_highlight = Color32::from_rgba_unmultiplied(254, 215, 170, 38);
                t.core_glow = with_alpha(accent, 0.40);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(58, 30, 48, 215);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(249, 115, 22, 86);
                t
            }
            ColorTheme::EmeraldAurora => {
                let accent = Color32::from_rgb(16, 185, 129);
                let mut t = Self::new(accent);
                t.accent_blue = Color32::from_rgb(56, 189, 248);
                t.accent_pink = Color32::from_rgb(244, 114, 182);
                t.accent_purple = Color32::from_rgb(167, 139, 250);
                t.accent_amber = Color32::from_rgb(251, 191, 36);
                t.accent_cyan = Color32::from_rgb(45, 212, 191);
                t.accent_green = Color32::from_rgb(52, 211, 153);
                t.pill_top = Color32::from_rgba_unmultiplied(17, 36, 32, 248);
                t.pill_bottom = Color32::from_rgba_unmultiplied(7, 16, 14, 252);
                t.pill_hover_top = Color32::from_rgba_unmultiplied(24, 50, 44, 250);
                t.pill_hover_bottom = Color32::from_rgba_unmultiplied(10, 22, 19, 254);
                t.pill_rim = Color32::from_rgba_unmultiplied(52, 211, 153, 46);
                t.pill_hover_rim = Color32::from_rgba_unmultiplied(45, 212, 191, 88);
                t.pill_inner_rim = Color32::from_rgba_unmultiplied(167, 243, 208, 22);
                t.pill_text_dim = Color32::from_rgb(208, 232, 224);
                t.opaque_surface = Color32::from_rgb(12, 26, 23);
                t.opaque_border = Color32::from_rgb(33, 68, 59);
                t.glass_fallback_surface = Color32::from_rgba_unmultiplied(14, 30, 26, 240);
                t.card_bg = Color32::from_rgba_unmultiplied(15, 32, 28, 222);
                t.card_top = Color32::from_rgba_unmultiplied(22, 46, 40, 230);
                t.card_bottom = Color32::from_rgba_unmultiplied(9, 20, 17, 236);
                t.card_border = Color32::from_rgba_unmultiplied(52, 211, 153, 34);
                t.card_highlight = Color32::from_rgba_unmultiplied(167, 243, 208, 36);
                t.core_glow = with_alpha(accent, 0.38);
                t.pill_btn_bg = Color32::from_rgba_unmultiplied(20, 52, 44, 215);
                t.pill_btn_border = Color32::from_rgba_unmultiplied(16, 185, 129, 86);
                t
            }
            ColorTheme::CeramicPearl => Self::ceramic_pearl(),
        }
    }

    pub fn color_theme_meta(preset: ColorTheme) -> (&'static str, &'static str, [Color32; 3]) {
        match preset {
            ColorTheme::Obsidian => (
                "Obsidian",
                "Signature smoked glass",
                [
                    Color32::from_rgb(18, 20, 26),
                    Color32::from_rgb(56, 189, 248),
                    Color32::from_rgb(168, 85, 247),
                ],
            ),
            ColorTheme::OledBlack => (
                "OLED Black",
                "Pure midnight contrast",
                [
                    Color32::from_rgb(5, 6, 9),
                    Color32::from_rgb(56, 189, 248),
                    Color32::from_rgb(38, 132, 255),
                ],
            ),
            ColorTheme::LinearIndigo => (
                "Linear Indigo",
                "Crisp indigo depth",
                [
                    Color32::from_rgb(18, 20, 34),
                    Color32::from_rgb(99, 102, 241),
                    Color32::from_rgb(139, 92, 246),
                ],
            ),
            ColorTheme::VisionSpatial => (
                "Vision Spatial",
                "Luminous frosted optics",
                [
                    Color32::from_rgb(33, 38, 50),
                    Color32::from_rgb(96, 165, 250),
                    Color32::from_rgb(226, 232, 240),
                ],
            ),
            ColorTheme::CatppuccinMocha => (
                "Catppuccin Mocha",
                "Pastel cozy dark",
                [
                    Color32::from_rgb(30, 30, 46),
                    Color32::from_rgb(203, 166, 247),
                    Color32::from_rgb(137, 180, 250),
                ],
            ),
            ColorTheme::Cyberpunk => (
                "Cyberpunk",
                "Electric neon chrome",
                [
                    Color32::from_rgb(18, 13, 30),
                    Color32::from_rgb(0, 229, 255),
                    Color32::from_rgb(255, 46, 147),
                ],
            ),
            ColorTheme::SunsetVelvet => (
                "Sunset Velvet",
                "Warm twilight plum",
                [
                    Color32::from_rgb(28, 16, 28),
                    Color32::from_rgb(249, 115, 22),
                    Color32::from_rgb(251, 113, 133),
                ],
            ),
            ColorTheme::EmeraldAurora => (
                "Emerald Aurora",
                "Deep boreal jade",
                [
                    Color32::from_rgb(12, 26, 23),
                    Color32::from_rgb(16, 185, 129),
                    Color32::from_rgb(45, 212, 191),
                ],
            ),
            ColorTheme::CeramicPearl => (
                "Ceramic Pearl",
                "Porcelain cobalt light",
                [
                    Color32::from_rgb(0xF1, 0xF5, 0xF9),
                    Color32::from_rgb(0x00, 0x66, 0xCC),
                    Color32::from_rgb(56, 189, 248),
                ],
            ),
        }
    }

    /// Returns `(top, bottom, rim, highlight)` RGBA bytes tuned for `Windows.UI.Composition`
    /// layering above `CreateHostBackdropBrush` (or solid tint fallback when blur is off).
    pub fn backdrop_palette(
        &self,
        mode: GlassMode,
        os_transparency: bool,
        hover_t: f32,
    ) -> ([u8; 4], [u8; 4], [u8; 4], [u8; 4]) {
        let base_top = mix(self.pill_top, self.pill_hover_top, hover_t);
        let base_bottom = mix(self.pill_bottom, self.pill_hover_bottom, hover_t);
        let base_rim = mix(self.pill_rim, self.pill_hover_rim, hover_t);
        let base_hl = mix(self.pill_highlight, self.pill_hover_highlight, hover_t);

        let is_light = self.is_light || self.pill_top.r() > 128;
        let alpha_scale = if !os_transparency || mode == GlassMode::Opaque {
            1.0
        } else {
            match (mode, is_light) {
                (GlassMode::Normal, false) => 0.62,
                (GlassMode::Normal, true) => 0.72,
                (GlassMode::Realistic, false) => 0.74,
                (GlassMode::Realistic, true) => 0.80,
                (GlassMode::Blurred, false) => 0.84,
                (GlassMode::Blurred, true) => 0.88,
                (GlassMode::Opaque, _) => 1.0,
            }
        };

        let mut top = base_top.to_srgba_unmultiplied();
        let mut bottom = base_bottom.to_srgba_unmultiplied();
        top[3] = (top[3] as f32 * alpha_scale).round().clamp(0.0, 255.0) as u8;
        bottom[3] = (bottom[3] as f32 * alpha_scale).round().clamp(0.0, 255.0) as u8;

        (
            top,
            bottom,
            base_rim.to_srgba_unmultiplied(),
            base_hl.to_srgba_unmultiplied(),
        )
    }

    pub fn font(size: f32) -> FontId {
        FontId::proportional(size)
    }

    pub fn font_semibold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FAMILY_SEMIBOLD.into()))
    }

    pub fn font_bold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FAMILY_BOLD.into()))
    }
}

/// Returns the secondary accent color for each `ColorTheme` preset, used for
/// dual-tone UI elements and Settings previews.
pub fn theme_accent_secondary(preset: ColorTheme) -> Color32 {
    match preset {
        ColorTheme::OledBlack => Color32::from_rgb(0x30, 0xD1, 0x58),
        ColorTheme::Obsidian => Color32::from_rgb(0x25, 0x63, 0xEB),
        ColorTheme::LinearIndigo => Color32::from_rgb(0xA8, 0x55, 0xF7),
        ColorTheme::VisionSpatial => Color32::from_rgb(0x38, 0xBD, 0xF8),
        ColorTheme::CatppuccinMocha => Color32::from_rgb(0xFA, 0xB3, 0x87),
        ColorTheme::Cyberpunk => Color32::from_rgb(0xFF, 0x00, 0x7F),
        ColorTheme::SunsetVelvet => Color32::from_rgb(0xF5, 0x9E, 0x0B),
        ColorTheme::EmeraldAurora => Color32::from_rgb(0x34, 0xD3, 0x99),
        ColorTheme::CeramicPearl => Color32::from_rgb(0x4F, 0x46, 0xE5),
    }
}

fn tint_rgb(base: Color32, tint: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [br, bg, bb, ba] = base.to_srgba_unmultiplied();
    let [tr, tg, tb, _] = tint.to_srgba_unmultiplied();
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(br, tr), l(bg, tg), l(bb, tb), ba)
}

pub fn with_alpha(c: Color32, alpha: f32) -> Color32 {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let a = (a as f32 * alpha.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.to_srgba_unmultiplied();
    let [br, bg, bb, ba] = b.to_srgba_unmultiplied();
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(ar, br), l(ag, bg), l(ab, bb), l(aa, ba))
}

/// Installs Segoe UI (regular + semibold + bold, tabular digits by default) and
/// Phosphor Regular + Fill icon fonts.
pub fn install_fonts(ctx: &egui::Context, assets: FontAssets) {
    let mut fonts = FontDefinitions::default();

    if let Some(bytes) = assets.regular {
        fonts
            .font_data
            .insert("segoe".into(), FontData::from_owned(bytes).into());
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, "segoe".into());
        }
    }
    let mut semibold_family = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(bytes) = assets.semibold {
        fonts
            .font_data
            .insert("segoe-semibold".into(), FontData::from_owned(bytes).into());
        semibold_family.insert(0, "segoe-semibold".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(FAMILY_SEMIBOLD.into()), semibold_family);

    let mut bold_family = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(bytes) = assets.bold {
        fonts
            .font_data
            .insert("segoe-bold".into(), FontData::from_owned(bytes).into());
        bold_family.insert(0, "segoe-bold".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(FAMILY_BOLD.into()), bold_family);

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    egui_phosphor::add_font_bytes_as_family(
        &mut fonts,
        FAMILY_ICON_FILL,
        egui_phosphor::Variant::Fill.font_bytes(),
    );
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_interpolates_endpoints() {
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 0.0), Color32::BLACK);
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
    }

    #[test]
    fn alpha_scales_unmultiplied_channel() {
        assert_eq!(with_alpha(Color32::WHITE, 0.5).a(), 128);
    }

    #[test]
    fn backdrop_palette_falls_back_to_high_alpha_when_os_transparency_off() {
        let dark = Theme::new(FALLBACK_ACCENT);
        let (glass_top, _, _, _) = dark.backdrop_palette(GlassMode::Normal, true, 0.0);
        let (fallback_top, _, _, _) = dark.backdrop_palette(GlassMode::Normal, false, 0.0);
        let (opaque_top, _, _, _) = dark.backdrop_palette(GlassMode::Opaque, true, 0.0);
        assert!(glass_top[3] < fallback_top[3]);
        assert_eq!(fallback_top, opaque_top);
    }

    #[test]
    fn light_and_dark_themes_define_3d_depth_tokens() {
        let dark = Theme::new(FALLBACK_ACCENT);
        let light = Theme::light(FALLBACK_ACCENT);
        assert_ne!(dark.pill_top, dark.pill_bottom);
        assert_ne!(light.pill_top, light.pill_bottom);
        assert!(dark.pill_shadow.a() > 0 && light.pill_shadow.a() > 0);
        assert!(dark.pill_highlight.a() > 0 && light.pill_highlight.a() > 0);
        assert_ne!(dark.pill_text, light.pill_text);
        assert!(!dark.is_light && light.is_light);
        assert_ne!(dark.card_top, dark.card_bottom);
        assert!(dark.core_glow.a() > 0 && light.core_glow.a() > 0);
    }

    #[test]
    fn for_config_maps_all_presets_and_modes() {
        let presets = [
            ColorTheme::Obsidian,
            ColorTheme::OledBlack,
            ColorTheme::LinearIndigo,
            ColorTheme::VisionSpatial,
            ColorTheme::CatppuccinMocha,
            ColorTheme::Cyberpunk,
            ColorTheme::SunsetVelvet,
            ColorTheme::EmeraldAurora,
            ColorTheme::CeramicPearl,
        ];
        for preset in presets {
            let (name, tagline, swatches) = Theme::color_theme_meta(preset);
            assert!(!name.is_empty() && !tagline.is_empty());
            assert_eq!(swatches.len(), 3);

            let t_dark = Theme::for_config(ThemeMode::Dark, preset, false);
            let t_light = Theme::for_config(ThemeMode::Light, preset, false);
            assert!(t_light.is_light);
            assert_eq!(t_light.pill_text, Color32::from_rgb(10, 15, 28));
            assert_eq!(t_light.pill_text_dim, Color32::from_rgb(51, 65, 85));

            if preset == ColorTheme::CeramicPearl {
                assert!(t_dark.is_light);
                assert_eq!(t_dark.accent, Color32::from_rgb(0x00, 0x66, 0xCC));
            } else {
                assert!(!t_dark.is_light);
                assert_eq!(t_dark.pill_text, Color32::WHITE);
                let dim_luma = 0.2126 * t_dark.pill_text_dim.r() as f32
                    + 0.7152 * t_dark.pill_text_dim.g() as f32
                    + 0.0722 * t_dark.pill_text_dim.b() as f32;
                assert!(
                    dim_luma >= 205.0,
                    "dark preset {:?} pill_text_dim luma too low: {}",
                    preset,
                    dim_luma
                );
                assert_eq!(t_light.accent, t_dark.accent);
            }

            let secondary = theme_accent_secondary(preset);
            assert_eq!(secondary.a(), 255);
        }

        // Auto + system_light: Obsidian adapts to light, stylized presets remain dark
        let obsidian_auto_light = Theme::for_config(ThemeMode::Auto, ColorTheme::Obsidian, true);
        assert!(obsidian_auto_light.is_light);
        let cyberpunk_auto_light = Theme::for_config(ThemeMode::Auto, ColorTheme::Cyberpunk, true);
        assert!(!cyberpunk_auto_light.is_light);
    }

    #[test]
    fn theme_accent_secondary_matches_all_presets() {
        assert_eq!(
            theme_accent_secondary(ColorTheme::OledBlack),
            Color32::from_rgb(0x30, 0xD1, 0x58)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::Obsidian),
            Color32::from_rgb(0x25, 0x63, 0xEB)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::LinearIndigo),
            Color32::from_rgb(0xA8, 0x55, 0xF7)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::VisionSpatial),
            Color32::from_rgb(0x38, 0xBD, 0xF8)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::CatppuccinMocha),
            Color32::from_rgb(0xFA, 0xB3, 0x87)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::Cyberpunk),
            Color32::from_rgb(0xFF, 0x00, 0x7F)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::SunsetVelvet),
            Color32::from_rgb(0xF5, 0x9E, 0x0B)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::EmeraldAurora),
            Color32::from_rgb(0x34, 0xD3, 0x99)
        );
        assert_eq!(
            theme_accent_secondary(ColorTheme::CeramicPearl),
            Color32::from_rgb(0x4F, 0x46, 0xE5)
        );
    }
}
