// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/app.rs (draw_pill / ink_text) and shell.rs.

//! Gate 1 compact pill: top-center capsule with Windows locale clock and
//! battery % (charging bolt when plugged in, or short date when no battery).

use crate::anim::{self, SpringParams};
use crate::geometry::Squircle;
use crate::paint;
use crate::theme::{self, FAMILY_ICON_FILL, FAMILY_ICON_REGULAR, SIZE_BODY, SIZE_LABEL, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, FontFamily, FontId, Id, Rect, Ui, Vec2, pos2, vec2};
use orbit_core::events::{BatteryStatus, ClockStatus};

pub const PILL_BASE_SIZE: Vec2 = vec2(160.0, 34.0);
pub const PILL_HOVER_SWELL: Vec2 = vec2(6.0, 2.0);
pub const PILL_TOP_MARGIN: f32 = 6.0;
pub const WINDOW_HEIGHT: f32 = 84.0;
const PILL_PAD_X: f32 = 15.0;

pub struct PillOutput {
    pub shape: Squircle,
    pub hovered: bool,
}

/// Computes the centered pill rect inside `window` for a given `size`.
pub fn centered_pill_rect(window: Rect, size: Vec2) -> Rect {
    let min_x = (window.center().x - size.x * 0.5).round();
    let min_y = window.min.y + PILL_TOP_MARGIN;
    Rect::from_min_size(pos2(min_x, min_y), size)
}

pub fn show(
    ui: &mut Ui,
    theme: &Theme,
    clock: &ClockStatus,
    battery: Option<BatteryStatus>,
) -> PillOutput {
    let ctx = ui.ctx().clone();
    let window = ctx.content_rect();
    let id = Id::new("orbit-pill");

    // Check pointer against last frame's shape (or base rect on frame 0).
    let prev_shape = ctx
        .data(|d| d.get_temp::<Squircle>(id.with("shape")))
        .unwrap_or_else(|| {
            let r = centered_pill_rect(window, PILL_BASE_SIZE);
            Squircle::new(r, r.height() * 0.5).with_exponent(2.0)
        });
    let hovered = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| prev_shape.contains(p));

    // Smooth hover brighten + subtle swell. Stops repainting once settled.
    let hover_t = anim::tween(&ctx, id.with("hover-t"), hovered, 0.14);
    let target_w = PILL_BASE_SIZE.x + if hovered { PILL_HOVER_SWELL.x } else { 0.0 };
    let target_h = PILL_BASE_SIZE.y + if hovered { PILL_HOVER_SWELL.y } else { 0.0 };
    let w = anim::spring(&ctx, id.with("w"), target_w, SpringParams::critical(420.0));
    let h = anim::spring(&ctx, id.with("h"), target_h, SpringParams::critical(420.0));

    let rect = centered_pill_rect(window, vec2(w, h));
    let shape = Squircle::new(rect, rect.height() * 0.5).with_exponent(2.0);
    ctx.data_mut(|d| d.insert_temp(id.with("shape"), shape));

    let outline = shape.outline();
    let top = theme::mix(theme.pill_top, theme.pill_hover_top, hover_t);
    let bottom = theme::mix(theme.pill_bottom, theme.pill_hover_bottom, hover_t);
    let rim = theme::mix(theme.pill_rim, theme.pill_hover_rim, hover_t);
    let highlight = theme::mix(theme.pill_highlight, theme.pill_hover_highlight, hover_t);

    let painter = ui.painter();
    paint::pill_surface(
        painter,
        &shape,
        &outline,
        top,
        bottom,
        rim,
        highlight,
        theme.noise_alpha,
        theme.grain_px,
    );

    let cy = rect.center().y;
    let left_x = rect.min.x + PILL_PAD_X;
    let right_x = rect.max.x - PILL_PAD_X;

    // Left: Windows locale clock (updates once per minute).
    ink_text(
        painter,
        &clock.time,
        Theme::font_semibold(SIZE_BODY),
        theme.pill_text,
        left_x,
        cy,
        false,
    );

    // Right: battery % + charging bolt, or short date when no battery.
    match battery {
        Some(b) => {
            let pct_rect = ink_text(
                painter,
                &format!("{}%", b.percent),
                Theme::font(SIZE_LABEL),
                theme.pill_text_dim,
                right_x,
                cy,
                true,
            );
            let (glyph, color, filled) = battery_icon(b, theme);
            ink_text(
                painter,
                glyph,
                icon_font(14.5, filled),
                color,
                pct_rect.min.x - 4.5,
                cy,
                true,
            );
        }
        None => {
            ink_text(
                painter,
                &clock.date,
                Theme::font(SIZE_LABEL),
                theme.pill_text_dim,
                right_x,
                cy,
                true,
            );
        }
    }

    PillOutput { shape, hovered }
}

/// Paints `text` aligned by its ink bounding box (`mesh_bounds`) rather than line box.
fn ink_text(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    x: f32,
    cy: f32,
    from_right: bool,
) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let ink = galley.mesh_bounds;
    let dx = if from_right {
        x - ink.max.x
    } else {
        x - ink.min.x
    };
    let offset = vec2(dx, cy - ink.center().y);
    painter.galley(pos2(offset.x, offset.y), galley, color);
    ink.translate(offset)
}

fn icon_font(size: f32, fill: bool) -> FontId {
    let family = if fill {
        FAMILY_ICON_FILL
    } else {
        FAMILY_ICON_REGULAR
    };
    FontId::new(size, FontFamily::Name(family.into()))
}

fn battery_icon(b: BatteryStatus, theme: &Theme) -> (&'static str, Color32, bool) {
    if b.charging {
        return (
            icons_fill::BATTERY_CHARGING,
            Color32::from_rgb(98, 214, 132),
            true,
        );
    }
    let glyph = match b.percent {
        90.. => icons::BATTERY_FULL,
        60.. => icons::BATTERY_HIGH,
        30.. => icons::BATTERY_MEDIUM,
        12.. => icons::BATTERY_LOW,
        _ => icons::BATTERY_WARNING,
    };
    let color = if b.percent < 12 {
        Color32::from_rgb(255, 99, 88)
    } else {
        theme.pill_text_dim
    };
    (glyph, color, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pill_is_horizontally_centered_at_top() {
        let window = Rect::from_min_size(egui::Pos2::ZERO, vec2(1920.0, WINDOW_HEIGHT));
        let r = centered_pill_rect(window, PILL_BASE_SIZE);
        assert_eq!(r.min.y, PILL_TOP_MARGIN);
        assert!((r.center().x - window.center().x).abs() <= 0.5);
    }
}
