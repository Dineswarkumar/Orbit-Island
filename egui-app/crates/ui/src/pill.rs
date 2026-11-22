// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/app.rs (draw_pill / ink_text) and shell.rs.

//! Compact pill: top-edge capsule with Windows locale clock (split AM/PM accent),
//! optional weather / focus timer / battery / short date, privacy dots, and 4-bar spectrum.

use crate::anim::{self, SpringParams};
use crate::geometry::Squircle;
use crate::paint;
use crate::theme::{
    self, FAMILY_ICON_FILL, FAMILY_ICON_REGULAR, SIZE_LABEL, SIZE_PILL_TIME, Theme,
};
use crate::{icons, icons_fill};
use egui::{Color32, FontFamily, FontId, Id, Rect, Sense, Stroke, TextureId, Ui, Vec2, pos2, vec2};
use orbit_core::config::{Placement, WeatherUnit};
use orbit_core::events::{
    AudioSpectrumState, BatteryStatus, ClockStatus, FocusTimerState, PrivacyState, WeatherGlance,
};
use orbit_core::store::AppSnapshot;

pub const PILL_BASE_SIZE: Vec2 = vec2(146.0, 38.0);
pub const PILL_COMPACT_HEIGHT: f32 = 38.0;
pub const PILL_MIN_WIDTH: f32 = 128.0;
pub const PILL_MAX_WIDTH: f32 = 304.0;
pub const PILL_HOVER_SWELL: Vec2 = vec2(4.0, 2.0);
pub const PILL_TOP_MARGIN: f32 = 6.0;
pub const PILL_EDGE_MARGIN: f32 = 24.0;
/// Fixed window height with headroom above expanded height (176 + 6) so OS never resizes.
pub const WINDOW_HEIGHT: f32 = 220.0;
const PILL_PAD_X: f32 = 12.0;

pub struct PillOutput {
    pub shape: Squircle,
    pub hovered: bool,
    pub clicked: bool,
}

/// Data view needed to lay out and paint the compact pill.
pub struct PillData<'a> {
    pub clock: &'a ClockStatus,
    pub battery: Option<BatteryStatus>,
    pub weather: Option<&'a WeatherGlance>,
    pub weather_enabled: bool,
    pub weather_unit: WeatherUnit,
    pub timer: &'a FocusTimerState,
    pub privacy: &'a PrivacyState,
    pub spectrum: &'a AudioSpectrumState,
    pub media_playing: bool,
    pub media_title: Option<&'a str>,
    pub media_artist: Option<&'a str>,
    pub media_art: Option<TextureId>,
    pub now_unix_ms: u64,
}

impl<'a> PillData<'a> {
    pub fn from_snapshot(snap: &'a AppSnapshot, now_unix_ms: u64) -> Self {
        Self::from_snapshot_with_media(snap, None, now_unix_ms)
    }

    pub fn from_snapshot_with_media(
        snap: &'a AppSnapshot,
        media_art: Option<TextureId>,
        now_unix_ms: u64,
    ) -> Self {
        let media_playing = snap
            .media
            .as_ref()
            .is_some_and(|m| m.playback_status.eq_ignore_ascii_case("playing"));
        let (media_title, media_artist) = match snap.media.as_ref() {
            Some(m) if !m.track.title.trim().is_empty() => (
                Some(m.track.title.as_str()),
                (!m.track.artist.trim().is_empty())
                    .then_some(m.track.artist.as_str())
                    .or(Some(m.source_title.as_str())),
            ),
            _ => (None, None),
        };
        Self {
            clock: &snap.clock,
            battery: snap.system.battery,
            weather: snap.weather.as_ref(),
            weather_enabled: snap.config.weather_enabled,
            weather_unit: snap.config.weather_unit,
            timer: &snap.focus_timer,
            privacy: &snap.privacy,
            spectrum: &snap.spectrum,
            media_playing,
            media_title,
            media_artist,
            media_art,
            now_unix_ms,
        }
    }
}

/// Computes the centered pill rect inside `window` for a given `size`.
pub fn centered_pill_rect(window: Rect, size: Vec2) -> Rect {
    pill_rect_for_placement(window, size, Placement::Center)
}

/// Target X coordinate for `width` under `placement`.
pub fn target_x_for_placement(window: Rect, width: f32, placement: Placement) -> f32 {
    match placement {
        Placement::Left => window.min.x + PILL_EDGE_MARGIN,
        Placement::Right => (window.max.x - width - PILL_EDGE_MARGIN).max(window.min.x),
        Placement::Center => (window.center().x - width * 0.5).round(),
    }
}

/// Computes the pill rect inside `window` according to `placement` (Left, Center, Right).
pub fn pill_rect_for_placement(window: Rect, size: Vec2, placement: Placement) -> Rect {
    let min_x = target_x_for_placement(window, size.x, placement);
    let min_y = window.min.y + PILL_TOP_MARGIN;
    Rect::from_min_size(pos2(min_x, min_y), size)
}

fn privacy_width_delta(priv_state: &PrivacyState) -> f32 {
    match (priv_state.camera_active, priv_state.microphone_active) {
        (true, true) => 56.0,
        (true, false) | (false, true) => 33.0,
        (false, false) => 0.0,
    }
}

/// Computes the responsive compact pill size tuned to only the active slots (no empty gaps).
pub fn compact_target_size(data: &PillData<'_>) -> Vec2 {
    let has_media_card =
        data.media_playing && (data.media_title.is_some() || data.media_art.is_some());
    let mut w: f32 = if has_media_card {
        let base = if !data.clock.ampm.is_empty() {
            258.0
        } else {
            240.0
        };
        base + privacy_width_delta(data.privacy)
    } else {
        // Clock digits (~46) + divider (~12) + padding (24) = 82 baseline
        let mut s = 76.0;
        if !data.clock.ampm.is_empty() {
            s += 22.0;
        }
        if data.timer.is_active() {
            s += 56.0;
        } else if data.weather_enabled && data.weather.is_some() {
            s += 42.0;
        } else if data.battery.is_some() {
            s += 44.0;
        } else {
            s += 46.0;
        }
        s += privacy_width_delta(data.privacy);
        if data.media_playing {
            s += 24.0;
        }
        s
    };
    w = w.clamp(PILL_MIN_WIDTH, PILL_MAX_WIDTH);
    vec2(w, PILL_COMPACT_HEIGHT)
}

pub fn show(
    ui: &mut Ui,
    theme: &Theme,
    clock: &ClockStatus,
    battery: Option<BatteryStatus>,
    placement: Placement,
) -> PillOutput {
    let empty_timer = FocusTimerState::default();
    let empty_privacy = PrivacyState::default();
    let empty_spectrum = AudioSpectrumState::default();
    let data = PillData {
        clock,
        battery,
        weather: None,
        weather_enabled: false,
        weather_unit: WeatherUnit::Celsius,
        timer: &empty_timer,
        privacy: &empty_privacy,
        spectrum: &empty_spectrum,
        media_playing: false,
        media_title: None,
        media_artist: None,
        media_art: None,
        now_unix_ms: 0,
    };
    let ctx = ui.ctx().clone();
    let window = ctx.content_rect();
    let id = Id::new("orbit-pill");

    let base_size = compact_target_size(&data);
    let prev_shape = ctx
        .data(|d| d.get_temp::<Squircle>(id.with("shape")))
        .unwrap_or_else(|| {
            let r = pill_rect_for_placement(window, base_size, placement);
            Squircle::new(r, r.height() * 0.5).with_exponent(2.0)
        });
    let hovered = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| prev_shape.contains(p));

    let hover_t = anim::tween(&ctx, id.with("hover-t"), hovered, 0.14);
    let max_w = (window.width() - 16.0).max(PILL_MIN_WIDTH);
    let target_w = (base_size.x + if hovered { PILL_HOVER_SWELL.x } else { 0.0 }).min(max_w);
    let target_h = base_size.y + if hovered { PILL_HOVER_SWELL.y } else { 0.0 };
    let w = anim::spring(&ctx, id.with("w"), target_w, SpringParams::critical(420.0));
    let h = anim::spring(&ctx, id.with("h"), target_h, SpringParams::critical(420.0));
    let target_x = target_x_for_placement(window, w, placement);
    let x = anim::spring(&ctx, id.with("x"), target_x, SpringParams::placement_x());

    let rect = Rect::from_min_size(pos2(x, window.min.y + PILL_TOP_MARGIN), vec2(w, h));
    let shape = Squircle::new(rect, rect.height() * 0.5).with_exponent(2.0);
    ctx.data_mut(|d| d.insert_temp(id.with("shape"), shape));

    let painter = ui.painter();
    paint_shell(painter, &shape, theme, hover_t);
    paint_compact_contents(painter, rect, theme, &data, 1.0);

    let clicked = ui
        .interact(rect, id.with("click"), Sense::click())
        .clicked();

    PillOutput {
        shape,
        hovered,
        clicked,
    }
}

pub fn paint_shell(painter: &egui::Painter, shape: &Squircle, theme: &Theme, hover_t: f32) {
    let outline = shape.outline();
    let top = theme::mix(theme.pill_top, theme.pill_hover_top, hover_t);
    let bottom = theme::mix(theme.pill_bottom, theme.pill_hover_bottom, hover_t);
    let rim = theme::mix(theme.pill_rim, theme.pill_hover_rim, hover_t);
    let highlight = theme::mix(theme.pill_highlight, theme.pill_hover_highlight, hover_t);

    paint::pill_surface(
        painter,
        shape,
        &outline,
        top,
        bottom,
        rim,
        theme.pill_inner_rim,
        highlight,
        theme.pill_shadow,
        theme.noise_alpha,
        theme.grain_px,
    );
}

pub fn paint_glass_frame(painter: &egui::Painter, shape: &Squircle, theme: &Theme, hover_t: f32) {
    let outline = shape.outline();
    let rim = theme::mix(theme.pill_rim, theme.pill_hover_rim, hover_t);
    let highlight = theme::mix(theme.pill_highlight, theme.pill_hover_highlight, hover_t);
    paint::glass_frame(
        painter,
        shape,
        &outline,
        theme.pill_shadow,
        rim,
        theme.pill_inner_rim,
        highlight,
    );
}

pub fn paint_compact_contents(
    painter: &egui::Painter,
    rect: Rect,
    theme: &Theme,
    data: &PillData<'_>,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let cy = rect.center().y;
    let left_x = rect.min.x + PILL_PAD_X;
    let mut right_x = rect.max.x - PILL_PAD_X;

    // Left: bold clock digits (~16.5px logical) + accent AM/PM suffix.
    let digits = if data.clock.time_digits.is_empty() {
        data.clock.time.as_str()
    } else {
        data.clock.time_digits.as_str()
    };
    let time_rect = ink_text(
        painter,
        digits,
        Theme::font_semibold(SIZE_PILL_TIME),
        theme::with_alpha(theme.pill_text, alpha),
        left_x,
        cy,
        false,
    );
    let mut after_clock = time_rect.max.x;
    if !data.clock.ampm.is_empty() {
        let ampm_rect = ink_text(
            painter,
            &data.clock.ampm,
            Theme::font_semibold(10.5),
            theme::with_alpha(theme.accent_blue, alpha),
            after_clock + 3.5,
            cy + 1.0,
            false,
        );
        after_clock = ampm_rect.max.x;
    }

    // Task 3: Privacy ringed icons on the RIGHT side (camera & mic only when active, no dots).
    if data.privacy.camera_active || data.privacy.microphone_active {
        if data.privacy.microphone_active {
            let center = pos2(right_x - 10.0, cy);
            paint_privacy_badge(
                painter,
                center,
                icons_fill::MICROPHONE,
                theme.accent_amber,
                alpha,
            );
            right_x -= 23.0;
        }
        if data.privacy.camera_active {
            let center = pos2(right_x - 10.0, cy);
            paint_privacy_badge(
                painter,
                center,
                icons_fill::VIDEO_CAMERA,
                theme.accent_green,
                alpha,
            );
            right_x -= 23.0;
        }
        let div_x = right_x - 3.5;
        if div_x > after_clock + 14.0 {
            painter.line_segment(
                [pos2(div_x, rect.min.y + 10.0), pos2(div_x, rect.max.y - 10.0)],
                Stroke::new(1.0, theme::with_alpha(theme.card_border, alpha)),
            );
            right_x = div_x - 6.5;
        }
    }

    let has_media_card =
        data.media_playing && (data.media_title.is_some() || data.media_art.is_some());
    if has_media_card && right_x - after_clock >= 72.0 {
        let div_x = after_clock + 6.5;
        painter.line_segment(
            [pos2(div_x, rect.min.y + 10.0), pos2(div_x, rect.max.y - 10.0)],
            Stroke::new(1.0, theme::with_alpha(theme.card_border, alpha)),
        );

        let eq_right = paint_compact_equalizer(
            painter,
            right_x,
            cy,
            &data.spectrum.compact_bars,
            theme.accent_blue,
            alpha,
        );
        let art_x = div_x + 6.5;
        let art_r = Rect::from_min_size(pos2(art_x, cy - 11.0), vec2(22.0, 22.0));
        if let Some(tex) = data.media_art {
            paint::rounded_texture(
                painter,
                art_r,
                5.0,
                tex,
                alpha,
                theme::with_alpha(theme.card_border, alpha),
            );
        } else {
            painter.rect_filled(
                art_r,
                5.0,
                theme::with_alpha(theme.accent_blue, 0.18 * alpha),
            );
            painter.rect_stroke(
                art_r,
                5.0,
                Stroke::new(1.0, theme::with_alpha(theme.accent_blue, 0.45 * alpha)),
                egui::StrokeKind::Inside,
            );
            let note = painter.layout_no_wrap(
                icons_fill::MUSIC_NOTES.into(),
                icon_font(12.0, true),
                theme::with_alpha(theme.accent_blue, alpha),
            );
            painter.galley(
                art_r.center() - note.size() * 0.5,
                note,
                theme::with_alpha(theme.accent_blue, alpha),
            );
        }

        let text_x = art_r.max.x + 6.5;
        let text_w = (eq_right - text_x - 6.0).max(12.0);
        if text_w > 18.0 {
            if let Some(title) = data.media_title {
                let mut job = egui::text::LayoutJob::single_section(
                    title.to_owned(),
                    egui::TextFormat {
                        font_id: Theme::font_semibold(10.5),
                        color: theme::with_alpha(theme.pill_text, alpha),
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping {
                    max_width: text_w,
                    max_rows: 1,
                    break_anywhere: true,
                    ..Default::default()
                };
                let tg = painter.layout_job(job);
                let ty = if data.media_artist.is_some() {
                    cy - 11.0
                } else {
                    cy - tg.size().y * 0.5
                };
                painter.galley(pos2(text_x, ty), tg, theme::with_alpha(theme.pill_text, alpha));
            }
            if let Some(artist) = data.media_artist {
                let mut job = egui::text::LayoutJob::single_section(
                    artist.to_owned(),
                    egui::TextFormat {
                        font_id: Theme::font_semibold(9.0),
                        color: theme::with_alpha(theme.accent_blue, alpha),
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping {
                    max_width: text_w,
                    max_rows: 1,
                    break_anywhere: true,
                    ..Default::default()
                };
                let ag = painter.layout_job(job);
                painter.galley(
                    pos2(text_x, cy + 0.5),
                    ag,
                    theme::with_alpha(theme.accent_blue, alpha),
                );
            }
        }
        return;
    }

    // Optional mini equalizer at right when audio plays without metadata.
    if data.media_playing {
        right_x = paint_compact_equalizer(
            painter,
            right_x,
            cy,
            &data.spectrum.compact_bars,
            theme.accent_blue,
            alpha,
        ) - 6.5;
    }

    // Right status slot: Focus timer -> Weather -> Battery -> Short date.
    let right_slot_min_x = if data.timer.is_active() {
        let rem = data.timer.remaining_secs(data.now_unix_ms);
        let label = format!("{:02}:{:02}", rem / 60, rem % 60);
        let color = if data.timer.is_running() {
            theme.accent_blue
        } else {
            theme.pill_text_dim
        };
        let txt_rect = ink_text(
            painter,
            &label,
            Theme::font_semibold(SIZE_LABEL),
            theme::with_alpha(color, alpha),
            right_x,
            cy,
            true,
        );
        ink_text(
            painter,
            icons_fill::TIMER,
            icon_font(12.5, true),
            theme::with_alpha(color, alpha),
            txt_rect.min.x - 4.0,
            cy,
            true,
        )
        .min
        .x
    } else if data.weather_enabled
        && let Some(w) = data.weather
    {
        let temp = match data.weather_unit {
            WeatherUnit::Celsius => w.temp_c,
            WeatherUnit::Fahrenheit => w.temp_f,
        };
        let temp_rect = ink_text(
            painter,
            &format!("{temp}°"),
            Theme::font_semibold(SIZE_LABEL + 0.5),
            theme::with_alpha(theme.pill_text, alpha),
            right_x,
            cy,
            true,
        );
        let (w_icon, w_col) = weather_icon(&w.condition_code, theme);
        ink_text(
            painter,
            w_icon,
            icon_font(14.5, true),
            theme::with_alpha(w_col, alpha),
            temp_rect.min.x - 4.0,
            cy,
            true,
        )
        .min
        .x
    } else if let Some(b) = data.battery {
        let pct_rect = ink_text(
            painter,
            &format!("{}%", b.percent),
            Theme::font_semibold(SIZE_LABEL),
            theme::with_alpha(theme.pill_text, alpha),
            right_x,
            cy,
            true,
        );
        let (glyph, color, filled) = battery_icon(b, theme);
        ink_text(
            painter,
            glyph,
            icon_font(14.5, filled),
            theme::with_alpha(color, alpha),
            pct_rect.min.x - 4.0,
            cy,
            true,
        )
        .min
        .x
    } else {
        ink_text(
            painter,
            &data.clock.date,
            Theme::font(SIZE_LABEL),
            theme::with_alpha(theme.pill_text_dim, alpha),
            right_x,
            cy,
            true,
        )
        .min
        .x
    };

    // Draw subtle vertical divider centered cleanly between clock and right status.
    if right_slot_min_x - after_clock >= 7.0 {
        let div_x = (after_clock + right_slot_min_x) * 0.5;
        painter.line_segment(
            [pos2(div_x, rect.min.y + 10.0), pos2(div_x, rect.max.y - 10.0)],
            Stroke::new(1.0, theme::with_alpha(theme.card_border, alpha)),
        );
    }
}

fn paint_privacy_badge(
    painter: &egui::Painter,
    center: egui::Pos2,
    glyph: &str,
    accent: Color32,
    alpha: f32,
) {
    let radius = 9.5;
    painter.circle_filled(center, radius, theme::with_alpha(accent, 0.14 * alpha));
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.1, theme::with_alpha(accent, 0.85 * alpha)),
    );
    let g = painter.layout_no_wrap(
        glyph.into(),
        icon_font(10.5, true),
        theme::with_alpha(accent, alpha),
    );
    painter.galley(center - g.size() * 0.5, g, theme::with_alpha(accent, alpha));
}

fn paint_compact_equalizer(
    painter: &egui::Painter,
    right_x: f32,
    cy: f32,
    compact_bars: &[f32; 4],
    accent: Color32,
    alpha: f32,
) -> f32 {
    let bar_w = 2.2;
    let gap = 1.8;
    let total_w = 5.0 * bar_w + 4.0 * gap;
    let start_x = right_x - total_w;
    for idx in 0..5 {
        let level = match idx {
            0..=3 => compact_bars[idx],
            _ => (compact_bars[1] + compact_bars[2]) * 0.5,
        };
        let h = (4.0 + level.clamp(0.0, 1.0) * 11.0).min(15.0);
        let bx = start_x + idx as f32 * (bar_w + gap);
        let r = Rect::from_center_size(pos2(bx + bar_w * 0.5, cy), vec2(bar_w, h));
        painter.rect_filled(r, 1.1, theme::with_alpha(accent, alpha));
    }
    start_x
}

fn weather_icon(code: &str, theme: &Theme) -> (&'static str, Color32) {
    match code {
        "storm" => (icons_fill::CLOUD_LIGHTNING, theme.accent_amber),
        "rain" => (icons_fill::CLOUD_RAIN, theme.accent_cyan),
        "snow" => (icons_fill::SNOWFLAKE, Color32::from_rgb(200, 232, 255)),
        "partly_cloudy" => (icons_fill::CLOUD_SUN, theme.pill_text),
        "cloudy" => (icons_fill::CLOUD, theme.pill_text_dim),
        _ => (icons_fill::SUN, theme.accent_amber),
    }
}

/// Paints `text` aligned by its ink bounding box (`mesh_bounds`) rather than line box.
pub(crate) fn ink_text(
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

pub(crate) fn icon_font(size: f32, fill: bool) -> FontId {
    let family = if fill {
        FAMILY_ICON_FILL
    } else {
        FAMILY_ICON_REGULAR
    };
    FontId::new(size, FontFamily::Name(family.into()))
}

fn battery_icon(b: BatteryStatus, theme: &Theme) -> (&'static str, Color32, bool) {
    if b.charging {
        return (icons_fill::BATTERY_CHARGING, theme.accent_green, true);
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

    #[test]
    fn compact_pill_is_shorter_has_larger_time_and_adapts_for_privacy_and_media() {
        assert_eq!(SIZE_PILL_TIME, 16.5);
        assert_eq!(PILL_COMPACT_HEIGHT, 38.0);

        let clock = ClockStatus {
            time: "11:11 AM".into(),
            time_digits: "11:11".into(),
            ampm: "AM".into(),
            date: "Nov 22".into(),
            ..Default::default()
        };
        let weather = WeatherGlance {
            temp_c: 22,
            temp_f: 72,
            label: "Rain".into(),
            condition_code: "rain".into(),
        };
        let empty_spec = AudioSpectrumState::default();
        let empty_timer = FocusTimerState::default();
        let none_priv = PrivacyState::default();
        let cam_only = PrivacyState {
            camera_active: true,
            ..Default::default()
        };
        let both_priv = PrivacyState {
            camera_active: true,
            microphone_active: true,
            ..Default::default()
        };

        let make = |priv_s, playing, title| PillData {
            clock: &clock,
            battery: None,
            weather: Some(&weather),
            weather_enabled: true,
            weather_unit: WeatherUnit::Celsius,
            timer: &empty_timer,
            privacy: priv_s,
            spectrum: &empty_spec,
            media_playing: playing,
            media_title: title,
            media_artist: None,
            media_art: None,
            now_unix_ms: 0,
        };

        let base = compact_target_size(&make(&none_priv, false, None));
        let one_priv = compact_target_size(&make(&cam_only, false, None));
        let two_priv = compact_target_size(&make(&both_priv, false, None));
        let media = compact_target_size(&make(&none_priv, true, Some("Track Name")));

        assert!(base.x < 150.0, "base pill must be compact (<150): {}", base.x);
        assert!(one_priv.x > base.x && two_priv.x > one_priv.x);
        assert!(media.x > two_priv.x);
    }
}
