// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/app.rs (draw_pill / ink_text) and shell.rs.

//! Compact pill: top-edge capsule with Windows locale clock (split AM/PM accent),
//! optional weather / focus timer / battery / short date, privacy dots, and 4-bar spectrum.

use crate::anim;
use crate::geometry::Squircle;
use crate::paint;
use crate::theme::{
    self, FAMILY_ICON_REGULAR, SIZE_LABEL, SIZE_PILL_TIME, Theme,
};
use crate::{icons, icons_fill};
use egui::epaint::{Mesh, PathShape, PathStroke};
use egui::{Color32, FontFamily, FontId, Id, Pos2, Rect, Sense, Stroke, TextureId, Ui, Vec2, pos2, vec2};
use orbit_core::config::{CompactRightSlot, Placement, WeatherUnit};
use orbit_core::events::{
    AudioSpectrumState, BatteryStatus, ClockStatus, FocusTimerState, PrivacyState, WeatherGlance,
};
use orbit_core::store::AppSnapshot;

pub const PILL_BASE_SIZE: Vec2 = vec2(146.0, 38.0);
pub const PILL_COMPACT_HEIGHT: f32 = 38.0;
pub const PILL_MIN_WIDTH: f32 = 128.0;
pub const PILL_MAX_WIDTH: f32 = 330.0;
pub const PILL_HOVER_SWELL: Vec2 = vec2(8.0, 2.5);
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
    pub compact_right_slot: CompactRightSlot,
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
            compact_right_slot: snap.config.compact_right_slot,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedRightSlot {
    Timer,
    Weather,
    Battery,
    OrbitLogo,
    WindowsLogo,
    AppleLogo,
    LinuxLogo,
    DateFallback,
}

pub fn resolve_right_slot(data: &PillData<'_>) -> ResolvedRightSlot {
    if data.timer.is_active() {
        return ResolvedRightSlot::Timer;
    }
    let has_weather = data.weather_enabled && data.weather.is_some();
    match data.compact_right_slot {
        CompactRightSlot::Weather => {
            if has_weather {
                ResolvedRightSlot::Weather
            } else if data.battery.is_some() {
                ResolvedRightSlot::Battery
            } else {
                ResolvedRightSlot::DateFallback
            }
        }
        CompactRightSlot::Battery => {
            if data.battery.is_some() {
                ResolvedRightSlot::Battery
            } else {
                ResolvedRightSlot::DateFallback
            }
        }
        CompactRightSlot::OrbitLogo => ResolvedRightSlot::OrbitLogo,
        CompactRightSlot::WindowsLogo => ResolvedRightSlot::WindowsLogo,
        CompactRightSlot::AppleLogo => ResolvedRightSlot::AppleLogo,
        CompactRightSlot::LinuxLogo => ResolvedRightSlot::LinuxLogo,
        CompactRightSlot::SmartAuto => {
            if has_weather {
                ResolvedRightSlot::Weather
            } else if data.battery.is_some() {
                ResolvedRightSlot::Battery
            } else {
                ResolvedRightSlot::OrbitLogo
            }
        }
    }
}

fn privacy_width_delta(priv_state: &PrivacyState) -> f32 {
    let count = [
        priv_state.camera_active,
        priv_state.microphone_active,
        priv_state.location_active,
    ]
    .into_iter()
    .filter(|&v| v)
    .count();
    if count == 0 {
        0.0
    } else {
        count as f32 * 23.0 + 10.0
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
        s += match resolve_right_slot(data) {
            ResolvedRightSlot::Timer => 56.0,
            ResolvedRightSlot::Weather => 42.0,
            ResolvedRightSlot::Battery => 44.0,
            ResolvedRightSlot::OrbitLogo
            | ResolvedRightSlot::WindowsLogo
            | ResolvedRightSlot::AppleLogo
            | ResolvedRightSlot::LinuxLogo => 28.0,
            ResolvedRightSlot::DateFallback => 46.0,
        };
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
        compact_right_slot: CompactRightSlot::Weather,
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

    let hover_t = anim::transition_tween(&ctx, id.with("hover-t"), hovered);
    let max_w = (window.width() - 16.0).max(PILL_MIN_WIDTH);
    let target_w = (base_size.x + if hovered { PILL_HOVER_SWELL.x } else { 0.0 }).min(max_w);
    let target_h = base_size.y + if hovered { PILL_HOVER_SWELL.y } else { 0.0 };
    let (w, _) = anim::transition_value(&ctx, id.with("w"), target_w, hovered);
    let (h, _) = anim::transition_value(&ctx, id.with("h"), target_h, hovered);
    let target_x = target_x_for_placement(window, target_w, placement);
    let (x, _) = anim::transition_value(&ctx, id.with("x"), target_x, hovered);

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

pub fn paint_opaque_shell(painter: &egui::Painter, shape: &Squircle, theme: &Theme) {
    let outline = shape.outline();
    paint::opaque_3d_surface(
        painter,
        shape,
        &outline,
        theme.pill_top,
        theme.pill_bottom,
        theme.pill_rim,
        theme.pill_inner_rim,
        theme.pill_highlight,
        theme.pill_shadow,
    );
}

pub fn paint_tinted_fallback(
    painter: &egui::Painter,
    shape: &Squircle,
    theme: &Theme,
    hover_t: f32,
) {
    let rim = theme::mix(theme.pill_rim, theme.pill_hover_rim, hover_t);
    let highlight = theme::mix(theme.pill_highlight, theme.pill_hover_highlight, hover_t);
    paint::tinted_fallback_surface(
        painter,
        shape,
        theme.glass_fallback_surface,
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
        Theme::font_bold(SIZE_PILL_TIME),
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

    // Privacy ringed icons on the RIGHT side (camera, mic, location when active).
    if data.privacy.camera_active
        || data.privacy.microphone_active
        || data.privacy.location_active
    {
        if data.privacy.location_active {
            let center = pos2(right_x - 10.0, cy);
            paint_privacy_badge(
                painter,
                center,
                icons_fill::MAP_PIN,
                theme.accent_cyan,
                alpha,
            );
            right_x -= 23.0;
        }
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
                Stroke::new(1.0, theme::with_alpha(theme.card_border, 1.0 * alpha)),
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
            Stroke::new(1.0, theme::with_alpha(theme.card_border, 1.0 * alpha)),
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

    // Right status slot resolved from user config & active context.
    let right_slot_min_x = match resolve_right_slot(data) {
        ResolvedRightSlot::Timer => {
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
                Theme::font(SIZE_LABEL),
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
        }
        ResolvedRightSlot::Weather => {
            if let Some(w) = data.weather {
                let temp = match data.weather_unit {
                    WeatherUnit::Celsius => w.temp_c,
                    WeatherUnit::Fahrenheit => w.temp_f,
                };
                let temp_rect = ink_text(
                    painter,
                    &format!("{temp}°"),
                    Theme::font(SIZE_LABEL + 0.5),
                    theme::with_alpha(theme.pill_text, alpha),
                    right_x,
                    cy,
                    true,
                );
                paint_weather_icon(
                    painter,
                    temp_rect.min.x - 4.0,
                    cy,
                    &w.condition_code,
                    theme,
                    alpha,
                )
            } else {
                right_x
            }
        }
        ResolvedRightSlot::Battery => {
            if let Some(b) = data.battery {
                let pct_rect = ink_text(
                    painter,
                    &format!("{}%", b.percent),
                    Theme::font(SIZE_LABEL),
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
                right_x
            }
        }
        ResolvedRightSlot::OrbitLogo => paint_orbit_logo(painter, right_x, cy, theme, alpha),
        ResolvedRightSlot::WindowsLogo => paint_windows_logo(painter, right_x, cy, alpha),
        ResolvedRightSlot::AppleLogo => paint_apple_logo(painter, right_x, cy, theme, alpha),
        ResolvedRightSlot::LinuxLogo => paint_linux_logo(painter, right_x, cy, theme, alpha),
        ResolvedRightSlot::DateFallback => {
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
        }
    };

    // Draw subtle vertical divider centered cleanly between clock and right status.
    if right_slot_min_x - after_clock >= 7.0 {
        let div_x = (after_clock + right_slot_min_x) * 0.5;
        painter.line_segment(
            [pos2(div_x, rect.min.y + 10.0), pos2(div_x, rect.max.y - 10.0)],
            Stroke::new(1.0, theme::with_alpha(theme.card_border, 1.0 * alpha)),
        );
    }
}

fn paint_orbit_logo(
    painter: &egui::Painter,
    right_x: f32,
    cy: f32,
    theme: &Theme,
    alpha: f32,
) -> f32 {
    let cx = right_x - 8.0;
    let center = pos2(cx, cy);
    let tilt = -0.45_f32;
    let (sin_t, cos_t) = tilt.sin_cos();
    let rx = 8.0_f32;
    let ry = 2.7_f32;
    let ring_pt = |t: f32| -> Pos2 {
        let lx = rx * t.cos();
        let ly = ry * t.sin();
        pos2(cx + lx * cos_t - ly * sin_t, cy + lx * sin_t + ly * cos_t)
    };

    // Soft orbital halo around the planetary core.
    painter.circle_filled(center, 6.2, theme::with_alpha(theme.accent, 0.20 * alpha));

    // Back half of the tilted elliptical orbit ring (behind core).
    let steps = 16;
    let back_pts: Vec<Pos2> = (0..=steps)
        .map(|i| ring_pt(std::f32::consts::PI + std::f32::consts::PI * (i as f32 / steps as f32)))
        .collect();
    painter.add(PathShape::line(
        back_pts,
        PathStroke::new(1.25, theme::with_alpha(theme.accent_cyan, 0.50 * alpha)),
    ));

    // Core sphere with specular highlight.
    painter.circle_filled(center, 3.8, theme::with_alpha(theme.accent, alpha));
    painter.circle_filled(
        pos2(cx - 1.1, cy - 1.1),
        1.4,
        theme::with_alpha(Color32::WHITE, 0.72 * alpha),
    );

    // Front half of the tilted elliptical orbit ring (in front of core) + orbiting satellite.
    let front_pts: Vec<Pos2> = (0..=steps)
        .map(|i| ring_pt(std::f32::consts::PI * (i as f32 / steps as f32)))
        .collect();
    painter.add(PathShape::line(
        front_pts,
        PathStroke::new(1.45, theme::with_alpha(theme.accent_cyan, 0.95 * alpha)),
    ));
    painter.circle_filled(
        ring_pt(0.42),
        1.25,
        theme::with_alpha(Color32::WHITE, 0.95 * alpha),
    );

    right_x - 16.0
}

fn paint_windows_logo(painter: &egui::Painter, right_x: f32, cy: f32, alpha: f32) -> f32 {
    let pane = 5.8_f32;
    let gap = 1.6_f32;
    let total = pane * 2.0 + gap;
    let x0 = right_x - total;
    let y0 = cy - total * 0.5;
    let rounding = 1.2_f32;

    let tiles = [
        (0.0, 0.0, Color32::from_rgb(0x38, 0xBD, 0xF8)), // #38BDF8
        (pane + gap, 0.0, Color32::from_rgb(0x0E, 0xA5, 0xE9)), // #0EA5E9
        (0.0, pane + gap, Color32::from_rgb(0x0E, 0xA5, 0xE9)), // #0EA5E9
        (pane + gap, pane + gap, Color32::from_rgb(0x02, 0x84, 0xC7)), // #0284C7
    ];
    for (dx, dy, color) in tiles {
        let r = Rect::from_min_size(pos2(x0 + dx, y0 + dy), vec2(pane, pane));
        painter.rect_filled(r, rounding, theme::with_alpha(color, alpha));
    }

    x0
}

fn paint_apple_logo(
    painter: &egui::Painter,
    right_x: f32,
    cy: f32,
    theme: &Theme,
    alpha: f32,
) -> f32 {
    let cx = right_x - 6.5;
    let y = cy + 0.5;
    let ink = theme::with_alpha(theme.pill_text, alpha);

    // Star-shaped clockwise boundary around interior center (cx - 0.5, y + 0.8) so a
    // triangle-fan Mesh renders the top/bottom clefts and right bite cleanly.
    let outline: [(f32, f32); 22] = [
        (0.0, -4.4),
        (2.4, -5.2),
        (4.8, -4.2),
        (5.6, -2.5),
        (4.3, -1.3),
        (3.8, 0.2),
        (4.4, 1.7),
        (5.7, 2.7),
        (5.2, 4.4),
        (4.0, 6.0),
        (2.4, 6.8),
        (1.1, 6.5),
        (0.0, 6.0),
        (-1.1, 6.5),
        (-2.5, 6.8),
        (-4.2, 5.8),
        (-5.6, 3.6),
        (-6.3, 0.8),
        (-6.1, -2.0),
        (-5.0, -4.1),
        (-3.2, -5.1),
        (-1.4, -4.9),
    ];
    let mut mesh = Mesh::default();
    mesh.colored_vertex(pos2(cx - 0.5, y + 0.8), ink);
    for (ox, oy) in outline {
        mesh.colored_vertex(pos2(cx + ox, y + oy), ink);
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(egui::Shape::mesh(mesh));

    // Tilted leaf above the cleft.
    let leaf = vec![
        pos2(cx - 0.1, y - 5.1),
        pos2(cx + 0.2, y - 6.7),
        pos2(cx + 1.4, y - 7.8),
        pos2(cx + 2.8, y - 8.1),
        pos2(cx + 2.5, y - 6.5),
        pos2(cx + 1.3, y - 5.4),
    ];
    painter.add(egui::Shape::convex_polygon(leaf, ink, PathStroke::NONE));

    right_x - 13.0
}

fn ellipse_poly(center: Pos2, rx: f32, ry: f32, tilt: f32, steps: usize) -> Vec<Pos2> {
    let (sin_t, cos_t) = tilt.sin_cos();
    (0..steps)
        .map(|i| {
            let a = std::f32::consts::TAU * (i as f32 / steps as f32);
            let lx = rx * a.cos();
            let ly = ry * a.sin();
            pos2(
                center.x + lx * cos_t - ly * sin_t,
                center.y + lx * sin_t + ly * cos_t,
            )
        })
        .collect()
}

fn paint_linux_logo(
    painter: &egui::Painter,
    right_x: f32,
    cy: f32,
    theme: &Theme,
    alpha: f32,
) -> f32 {
    let cx = right_x - 6.8;
    let coat = if theme.is_light {
        Color32::from_rgb(24, 30, 42)
    } else {
        Color32::from_rgb(44, 52, 68)
    };
    let coat_col = theme::with_alpha(coat, alpha);
    let belly_col = theme::with_alpha(Color32::from_rgb(248, 250, 252), alpha);
    let amber_col = theme::with_alpha(Color32::from_rgb(245, 158, 11), alpha);
    let pupil_col = theme::with_alpha(Color32::from_rgb(15, 23, 42), alpha);

    // Flippers (wings) behind body.
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx - 4.7, cy + 1.3), 1.6, 3.6, 0.35, 12),
        coat_col,
        PathStroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx + 4.7, cy + 1.3), 1.6, 3.6, -0.35, 12),
        coat_col,
        PathStroke::NONE,
    ));

    // Torso & head.
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx, cy + 1.5), 4.6, 5.2, 0.0, 16),
        coat_col,
        PathStroke::NONE,
    ));
    painter.circle_filled(pos2(cx, cy - 3.8), 3.6, coat_col);

    // Subtle crisp edge rim on dark themes so Tux separates cleanly from dark glass.
    if !theme.is_light {
        let rim = PathStroke::new(0.8, theme::with_alpha(Color32::from_rgb(203, 213, 225), 0.35 * alpha));
        painter.add(PathShape::closed_line(
            ellipse_poly(pos2(cx, cy + 1.5), 4.6, 5.2, 0.0, 16),
            rim,
        ));
    }

    // Bright white belly.
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx, cy + 2.0), 3.0, 4.1, 0.0, 14),
        belly_col,
        PathStroke::NONE,
    ));

    // Eyes.
    for (ex, px) in [(cx - 1.35, cx - 1.15), (cx + 1.35, cx + 1.15)] {
        painter.circle_filled(pos2(ex, cy - 4.3), 1.15, belly_col);
        painter.circle_filled(pos2(px, cy - 4.2), 0.55, pupil_col);
    }

    // Amber beak.
    let beak = vec![
        pos2(cx, cy - 3.1),
        pos2(cx + 1.6, cy - 2.3),
        pos2(cx, cy - 1.4),
        pos2(cx - 1.6, cy - 2.3),
    ];
    painter.add(egui::Shape::convex_polygon(beak, amber_col, PathStroke::NONE));

    // Amber webbed feet.
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx - 2.2, cy + 6.3), 1.8, 0.95, -0.15, 10),
        amber_col,
        PathStroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        ellipse_poly(pos2(cx + 2.2, cy + 6.3), 1.8, 0.95, 0.15, 10),
        amber_col,
        PathStroke::NONE,
    ));

    right_x - 13.5
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

fn draw_white_cloud(
    painter: &egui::Painter,
    cx: f32,
    cy: f32,
    scale: f32,
    theme: &Theme,
    alpha: f32,
) {
    draw_cloud_layer(
        painter,
        cx,
        cy,
        scale,
        Color32::from_rgb(252, 253, 255),
        theme.is_light,
        alpha,
    );
}

fn draw_cloud_layer(
    painter: &egui::Painter,
    cx: f32,
    cy: f32,
    scale: f32,
    fill: Color32,
    outline: bool,
    alpha: f32,
) {
    let puffs = [
        (pos2(cx - 3.2 * scale, cy + 0.7 * scale), 2.7 * scale),
        (pos2(cx - 0.3 * scale, cy - 1.2 * scale), 3.4 * scale),
        (pos2(cx + 2.8 * scale, cy + 0.5 * scale), 2.6 * scale),
    ];
    let bridge = Rect::from_min_max(
        pos2(cx - 3.2 * scale, cy + 0.2 * scale),
        pos2(cx + 2.8 * scale, cy + 3.4 * scale),
    );
    let bridge_rounding = 1.5 * scale;

    if outline {
        let border_col = theme::with_alpha(Color32::from_rgb(203, 213, 225), alpha);
        for (center, radius) in puffs {
            painter.circle_filled(center, radius + 1.0, border_col);
        }
        painter.rect_filled(bridge.expand(1.0), bridge_rounding + 1.0, border_col);
    }

    let fill_col = theme::with_alpha(fill, alpha);
    for (center, radius) in puffs {
        painter.circle_filled(center, radius, fill_col);
    }
    painter.rect_filled(bridge, bridge_rounding, fill_col);
}

fn paint_weather_icon(
    painter: &egui::Painter,
    right_x: f32,
    cy: f32,
    code: &str,
    theme: &Theme,
    alpha: f32,
) -> f32 {
    let x0 = right_x - 15.5;
    let cx = right_x - 7.75;
    let sun_col = Color32::from_rgb(251, 191, 36);

    match code {
        "partly_cloudy" => {
            let sun_c = pos2(cx + 3.1, cy - 2.3);
            painter.circle_filled(sun_c, 4.1, theme::with_alpha(sun_col, 0.24 * alpha));
            painter.circle_filled(sun_c, 2.9, theme::with_alpha(sun_col, alpha));
            draw_white_cloud(painter, cx - 0.8, cy + 1.4, 0.95, theme, alpha);
        }
        "cloudy" => {
            draw_cloud_layer(
                painter,
                cx + 1.3,
                cy + 1.0,
                0.86,
                Color32::from_rgb(214, 222, 234),
                theme.is_light,
                0.72 * alpha,
            );
            draw_white_cloud(painter, cx - 0.6, cy - 0.4, 0.98, theme, alpha);
        }
        "rain" => {
            draw_white_cloud(painter, cx, cy - 1.8, 0.95, theme, alpha);
            let drop_col = theme::with_alpha(theme.accent_cyan, alpha);
            for ox in [-3.2_f32, 0.0, 3.2] {
                painter.line_segment(
                    [pos2(cx + ox, cy + 2.6), pos2(cx + ox - 1.1, cy + 5.6)],
                    Stroke::new(1.4, drop_col),
                );
            }
        }
        "storm" => {
            draw_white_cloud(painter, cx, cy - 1.8, 0.95, theme, alpha);
            let bolt_col = theme::with_alpha(
                if theme.is_light {
                    theme.accent_amber
                } else {
                    sun_col
                },
                alpha,
            );
            let top = vec![
                pos2(cx + 0.8, cy + 0.8),
                pos2(cx - 1.9, cy + 3.7),
                pos2(cx + 0.4, cy + 3.5),
            ];
            let bottom = vec![
                pos2(cx - 0.3, cy + 3.1),
                pos2(cx + 2.0, cy + 3.1),
                pos2(cx - 1.2, cy + 6.4),
            ];
            painter.add(egui::Shape::convex_polygon(top, bolt_col, Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(bottom, bolt_col, Stroke::NONE));
        }
        "snow" => {
            draw_white_cloud(painter, cx, cy - 1.8, 0.95, theme, alpha);
            let flake_col = theme::with_alpha(
                if theme.is_light {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(186, 230, 253)
                },
                alpha,
            );
            for (ox, oy) in [(-3.3_f32, 3.6_f32), (0.0, 4.9), (3.3, 3.6)] {
                painter.circle_filled(pos2(cx + ox, cy + oy), 1.35, flake_col);
            }
        }
        _ => {
            // "clear"
            let sun_c = pos2(cx, cy);
            painter.circle_filled(sun_c, 6.2, theme::with_alpha(sun_col, 0.26 * alpha));
            painter.circle_filled(sun_c, 4.0, theme::with_alpha(sun_col, alpha));
        }
    }

    x0
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

pub(crate) fn icon_font(size: f32, filled: bool) -> FontId {
    let family = if filled {
        crate::theme::FAMILY_ICON_FILL
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
        assert_eq!(SIZE_PILL_TIME, 19.5);
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
        let loc_only = PrivacyState {
            location_active: true,
            location_app: Some("Maps".into()),
            ..Default::default()
        };
        let both_priv = PrivacyState {
            camera_active: true,
            microphone_active: true,
            ..Default::default()
        };
        let all_priv = PrivacyState {
            camera_active: true,
            microphone_active: true,
            location_active: true,
            location_app: Some("Maps".into()),
            ..Default::default()
        };

        let make = |priv_s, playing, title, compact_right_slot| PillData {
            clock: &clock,
            battery: None,
            weather: Some(&weather),
            weather_enabled: true,
            weather_unit: WeatherUnit::Celsius,
            compact_right_slot,
            timer: &empty_timer,
            privacy: priv_s,
            spectrum: &empty_spec,
            media_playing: playing,
            media_title: title,
            media_artist: None,
            media_art: None,
            now_unix_ms: 0,
        };

        let base = compact_target_size(&make(&none_priv, false, None, CompactRightSlot::Weather));
        let one_priv = compact_target_size(&make(&cam_only, false, None, CompactRightSlot::Weather));
        let loc_priv = compact_target_size(&make(&loc_only, false, None, CompactRightSlot::Weather));
        let two_priv = compact_target_size(&make(&both_priv, false, None, CompactRightSlot::Weather));
        let three_priv = compact_target_size(&make(&all_priv, false, None, CompactRightSlot::Weather));
        let media = compact_target_size(&make(&none_priv, true, Some("Track Name"), CompactRightSlot::Weather));

        assert!(base.x < 150.0, "base pill must be compact (<150): {}", base.x);
        assert!(one_priv.x > base.x && loc_priv.x == one_priv.x);
        assert!(two_priv.x > one_priv.x && three_priv.x > two_priv.x);
        assert!(media.x > two_priv.x);

        for slot in [
            CompactRightSlot::OrbitLogo,
            CompactRightSlot::WindowsLogo,
            CompactRightSlot::AppleLogo,
            CompactRightSlot::LinuxLogo,
        ] {
            let logo_size = compact_target_size(&make(&none_priv, false, None, slot));
            assert!(
                logo_size.x < base.x,
                "logo slot {:?} ({}) should be narrower than weather ({})",
                slot,
                logo_size.x,
                base.x
            );
        }

        // SmartAuto resolves Weather -> Battery -> OrbitLogo, and active Timer wins over all.
        let mut d = make(&none_priv, false, None, CompactRightSlot::SmartAuto);
        assert_eq!(resolve_right_slot(&d), ResolvedRightSlot::Weather);
        d.weather = None;
        assert_eq!(resolve_right_slot(&d), ResolvedRightSlot::OrbitLogo);
        d.battery = Some(BatteryStatus {
            percent: 82,
            charging: false,
        });
        assert_eq!(resolve_right_slot(&d), ResolvedRightSlot::Battery);

        let active_timer = FocusTimerState {
            paused_remaining_secs: Some(120),
            ..Default::default()
        };
        d.timer = &active_timer;
        d.compact_right_slot = CompactRightSlot::AppleLogo;
        assert_eq!(resolve_right_slot(&d), ResolvedRightSlot::Timer);
    }
}
