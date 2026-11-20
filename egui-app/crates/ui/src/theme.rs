// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/theme.rs.

//! Design tokens and font installation (no direct file I/O in this crate).

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};

pub const FALLBACK_ACCENT: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);

pub const SIZE_LABEL: f32 = 11.5;
pub const SIZE_BODY: f32 = 13.0;

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
    pub pill_top: Color32,
    pub pill_bottom: Color32,
    pub pill_hover_top: Color32,
    pub pill_hover_bottom: Color32,
    pub pill_rim: Color32,
    pub pill_hover_rim: Color32,
    pub pill_highlight: Color32,
    pub pill_hover_highlight: Color32,
    pub pill_text: Color32,
    pub pill_text_dim: Color32,
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
            pill_top: Color32::from_rgba_unmultiplied(24, 24, 28, 246),
            pill_bottom: Color32::from_rgba_unmultiplied(7, 7, 10, 250),
            pill_hover_top: Color32::from_rgba_unmultiplied(38, 40, 48, 250),
            pill_hover_bottom: Color32::from_rgba_unmultiplied(15, 16, 22, 252),
            pill_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 26),
            pill_hover_rim: Color32::from_rgba_unmultiplied(255, 255, 255, 56),
            pill_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 38),
            pill_hover_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 76),
            pill_text: Color32::from_rgb(245, 245, 247),
            pill_text_dim: Color32::from_rgb(168, 172, 184),
            noise_alpha: 6,
            grain_px: 1.0,
        }
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
}
