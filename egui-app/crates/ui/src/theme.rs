// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/theme.rs.

//! Design tokens and font installation (no direct file I/O in this crate).

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use orbit_core::config::{GlassMode, ThemeMode};

pub const FALLBACK_ACCENT: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);

pub const SIZE_LABEL: f32 = 12.0;
pub const SIZE_BODY: f32 = 13.0;
pub const SIZE_PILL_TIME: f32 = 16.5;

pub const FAMILY_SEMIBOLD: &str = "orbit-semibold";
pub const FAMILY_ICON_FILL: &str = "phosphor-fill";
pub const FAMILY_ICON_REGULAR: &str = "phosphor";

/// Font bytes passed in from `orbit_platform::fonts`.
pub struct FontAssets {
    pub regular: Option<Vec<u8>>,
    pub semibold: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
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
    pub card_bg: Color32,
    pub card_border: Color32,
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
            pill_text: Color32::from_rgb(245, 245, 247),
            pill_text_dim: Color32::from_rgb(168, 172, 184),
            card_bg: Color32::from_rgba_unmultiplied(22, 24, 30, 215),
            card_border: Color32::from_rgba_unmultiplied(255, 255, 255, 26),
            track_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 22),
            pill_btn_bg: Color32::from_rgba_unmultiplied(20, 28, 44, 210),
            pill_btn_border: Color32::from_rgba_unmultiplied(59, 130, 246, 72),
            noise_alpha: 6,
            grain_px: 1.0,
        }
    }

    pub fn light(accent: Color32) -> Self {
        Self {
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
            pill_text: Color32::from_rgb(17, 24, 39),
            pill_text_dim: Color32::from_rgb(82, 92, 110),
            card_bg: Color32::from_rgba_unmultiplied(236, 241, 248, 228),
            card_border: Color32::from_rgba_unmultiplied(15, 23, 42, 28),
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

        let is_light = self.pill_top.r() > 128;
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

/// Installs Segoe UI (regular + semibold, tabular digits by default) and
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
    }
}
