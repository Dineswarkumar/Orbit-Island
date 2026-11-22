//! Gate 3 expanded panel: unified squircle morph from compact pill to expanded bento panel,
//! per-tile `TSTransform` stagger/hover/press physics, and non-wrapping vertical sections.

pub mod calendar;
pub mod hardware;
pub mod home;
pub mod multitask;

use crate::anim;
use crate::bubble::{
    self, BUBBLE_COMPACT_RADIUS, BUBBLE_COMPACT_SIZE, BUBBLE_HOVER_RADIUS, BUBBLE_HOVER_SIZE,
    BubbleState,
};
use crate::geometry::Squircle;
use crate::paint;
use crate::pill::{
    self, PILL_COMPACT_HEIGHT, PILL_HOVER_SWELL, PILL_MIN_WIDTH, PILL_TOP_MARGIN, PillData,
    compact_target_size, target_x_for_placement,
};
use crate::shelf::{self, SHELF_PANEL_SIZE, SHELF_RADIUS, ShelfUiState};
use crate::theme::{self, Theme};
use crate::icons_fill;
use egui::emath::TSTransform;
use egui::{Area, Color32, Context, Id, Order, Rect, Sense, Stroke, TextureId, Ui, Vec2, pos2, vec2};
use orbit_core::config::ExpandTrigger;
use orbit_core::store::AppSnapshot;
use std::collections::HashMap;
use std::time::Duration;

pub const EXPANDED_SIZE: Vec2 = vec2(360.0, 176.0);
pub const EXPANDED_RADIUS: f32 = 28.0;
pub const DROP_HOVER_SIZE: Vec2 = vec2(320.0, 96.0);
pub const DROP_HOVER_RADIUS: f32 = 24.0;
pub const DROP_FEEDBACK_SIZE: Vec2 = vec2(320.0, 50.0);
pub const DROP_FEEDBACK_RADIUS: f32 = 25.0;
const SCROLL_DEBOUNCE_SECS: f64 = 0.21;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Home,
    MultiTask,
    Calendar,
    Hardware,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomeSubView {
    #[default]
    Auto,
    Clock,
    Media,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UtilityAlertKind {
    PowerConnected,
    PowerDisconnected,
    WifiConnected,
    WifiDisconnected,
    BluetoothOn,
    BluetoothOff,
    LocationActive,
    TimerComplete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UtilityToast {
    pub kind: UtilityAlertKind,
    pub title: String,
    pub detail: String,
    pub accent: Color32,
    pub until_secs: f64,
    pub is_timer_complete: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PanelState {
    pub expanded: bool,
    pub active_section: usize,
    pub last_scroll_time: f64,
    pub hover_since: Option<f64>,
    pub leave_since: Option<f64>,
    pub home_sub_view: HomeSubView,
    pub calendar_year: i32,
    pub calendar_month: u32,
    pub calendar_selected_day: u32,
    pub adding_task: bool,
    pub new_task_title: String,
    pub ram_boost_feedback: Option<(String, bool, f64)>,
    pub last_ram_boost: Option<(u32, f32, f64)>,
    pub drop_feedback: Option<(String, bool, f64)>,
    pub bubble: BubbleState,
    pub shelf: ShelfUiState,
    pub return_expanded_after_bubble: bool,
    pub utility_toast: Option<UtilityToast>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PanelAction {
    StartFocusTimer(u64),
    TogglePauseFocusTimer,
    ResetFocusTimer,
    MediaCommand(&'static str),
    MediaSeek(f64),
    BoostRam,
    AddReminder {
        title: String,
        due_timestamp_ms: u64,
    },
    ToggleReminder(String),
    ActivateNotification {
        id: u32,
        app_id: String,
        app_name: String,
        title: String,
    },
    DismissNotification(u32),
    ClearAllNotifications,
    ShowLatestNotification,
    DismissUtilityToast,
    ShelfAddEntry(String),
    ShelfPickFiles,
    ShelfOpenItem(String),
    ShelfOpenWithItem(String),
    ShelfCopyItem(String),
    ShelfRevealItem(String),
    ShelfShareFiles,
    ShelfRemoveItem(String),
    ShelfClear,
    ShelfToggleStorageMode,
    OpenShelfStub,
    OpenSettings,
}

pub struct PanelOutput {
    pub shape: Squircle,
    pub hovered: bool,
    pub hover_t: f32,
    pub anim_active: bool,
    pub bubble_draining: bool,
    pub current_section: SectionKind,
    pub actions: Vec<PanelAction>,
}

/// Computes available sections in strict non-wrapping PRD order.
pub fn sections_for(snap: &AppSnapshot) -> Vec<SectionKind> {
    let media_active = snap.media.as_ref().is_some_and(|m| {
        m.playback_status.eq_ignore_ascii_case("playing")
            || (m.playback_status.eq_ignore_ascii_case("paused") && !m.track.title.trim().is_empty())
    });
    let privacy_active = snap.privacy.camera_active || snap.privacy.microphone_active;
    let timer_active = snap.focus_timer.is_active();
    let shelf_active = !snap.shelf.is_empty();
    let reminder_active = snap.reminders.iter().any(|r| !r.is_completed);

    let active_count = [
        media_active,
        privacy_active,
        timer_active,
        shelf_active,
        reminder_active,
    ]
    .into_iter()
    .filter(|&v| v)
    .count();

    let mut out = Vec::with_capacity(4);
    out.push(SectionKind::Home);
    if active_count >= 2 {
        out.push(SectionKind::MultiTask);
    }
    out.push(SectionKind::Calendar);
    out.push(SectionKind::Hardware);
    out
}

/// Advances or rewinds section index on wheel input with strict non-wrapping bounds.
pub fn apply_wheel_step(current: usize, section_count: usize, delta_y: f32) -> usize {
    if section_count == 0 {
        return 0;
    }
    let last = section_count - 1;
    let cur = current.min(last);
    if delta_y < -0.5 {
        (cur + 1).min(last)
    } else if delta_y > 0.5 {
        cur.saturating_sub(1)
    } else {
        cur
    }
}

/// Renders the unified ORBIT island (compact pill, notification bubble, expanded panel, or Shelf) inside `ui`.
#[allow(clippy::too_many_arguments)]
pub fn show_island(
    ui: &mut Ui,
    snap: &AppSnapshot,
    theme: &Theme,
    state: &mut PanelState,
    shelf_thumbs: &HashMap<String, TextureId>,
    media_art: Option<TextureId>,
    now_unix_ms: u64,
    live_glass: bool,
    opaque_surface: bool,
) -> PanelOutput {
    let ctx = ui.ctx().clone();
    let window = ctx.content_rect();
    let id = Id::new("orbit-island");
    let now = ctx.input(|i| i.time);
    let prev_expanded = state.expanded;
    let mut actions = Vec::new();

    if let Some((_, _, until)) = state.ram_boost_feedback {
        if now >= until {
            state.ram_boost_feedback = None;
        } else {
            ctx.request_repaint_after(Duration::from_secs_f64((until - now + 0.02).max(0.02)));
        }
    }
    if let Some((_, _, until)) = state.drop_feedback {
        if now >= until {
            state.drop_feedback = None;
        } else {
            ctx.request_repaint_after(Duration::from_secs_f64((until - now + 0.02).max(0.02)));
        }
    }
    if let Some(ref t) = state.utility_toast {
        if now >= t.until_secs {
            state.utility_toast = None;
        } else {
            let remaining = Duration::from_secs_f64((t.until_secs - now + 0.03).max(0.03));
            if t.is_timer_complete && !state.expanded && state.bubble.active.is_none() {
                ctx.request_repaint_after(Duration::from_millis(33).min(remaining));
            } else {
                ctx.request_repaint_after(remaining);
            }
        }
    }

    let sections = sections_for(snap);
    if state.active_section >= sections.len() {
        state.active_section = sections.len().saturating_sub(1);
    }
    let current_section = sections[state.active_section];

    if let Some((_, _, until)) = state.last_ram_boost {
        if now >= until {
            state.last_ram_boost = None;
        } else if state.expanded && current_section == SectionKind::Hardware {
            ctx.request_repaint_after(Duration::from_secs_f64((until - now + 0.05).max(0.05)));
        }
    }

    let pill_data = PillData::from_snapshot_with_media(snap, media_art, now_unix_ms);
    let compact_size = compact_target_size(&pill_data);

    let prev_shape = ctx
        .data(|d| d.get_temp::<Squircle>(id.with("shape")))
        .unwrap_or_else(|| {
            let r = pill::pill_rect_for_placement(window, compact_size, snap.config.placement);
            Squircle::new(r, r.height() * 0.5).with_exponent(2.0)
        });
    let hovered = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| prev_shape.contains(p));
    let hovered_file_count = ctx.input(|i| i.raw.hovered_files.len());
    let files_hovered = hovered_file_count > 0;
    let drop_hover_active = files_hovered && !state.shelf.open;
    let drop_toast_active = !drop_hover_active && state.drop_feedback.is_some() && !state.shelf.open;

    let modal_input_active = state.adding_task || state.shelf.adding || state.shelf.menu_open;
    let collapse_grace_secs = (snap.config.collapse_grace_ms.clamp(50, 1500) as f64) / 1000.0;
    if state.expanded {
        state.hover_since = None;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if state.shelf.menu_open {
                state.shelf.menu_open = false;
            } else if state.shelf.adding {
                state.shelf.adding = false;
            } else if state.shelf.open {
                state.shelf.open = false;
            } else {
                state.expanded = false;
                state.adding_task = false;
                state.leave_since = None;
            }
        } else if !hovered && !modal_input_active && !files_hovered {
            let since = *state.leave_since.get_or_insert(now);
            let elapsed = now - since;
            if elapsed >= collapse_grace_secs {
                state.expanded = false;
                state.adding_task = false;
                state.shelf.open = false;
                state.shelf.adding = false;
                state.shelf.menu_open = false;
                state.leave_since = None;
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(
                    (collapse_grace_secs - elapsed + 0.01).max(0.01),
                ));
            }
        } else {
            state.leave_since = None;
        }
    } else {
        state.leave_since = None;
        if snap.config.expand_trigger == ExpandTrigger::Hover
            && hovered
            && !files_hovered
            && !drop_toast_active
            && state.bubble.active.is_none()
            && state.utility_toast.is_none()
        {
            let hover_delay_secs = (snap.config.hover_delay_ms.min(1000) as f64) / 1000.0;
            let since = *state.hover_since.get_or_insert(now);
            let elapsed = now - since;
            if elapsed >= hover_delay_secs {
                state.expanded = true;
                state.hover_since = None;
                state.leave_since = None;
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(
                    (hover_delay_secs - elapsed + 0.01).max(0.01),
                ));
            }
        } else {
            state.hover_since = None;
        }
    }

    if state.expanded != prev_expanded {
        state.active_section = 0;
    }

    // Handle mouse wheel section navigation when expanded (disabled while Shelf ScrollArea is active).
    if state.expanded && hovered && !modal_input_active && !state.shelf.open {
        let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.5 && now - state.last_scroll_time >= SCROLL_DEBOUNCE_SECS {
            let next = apply_wheel_step(state.active_section, sections.len(), scroll_y);
            if next != state.active_section {
                state.active_section = next;
                state.last_scroll_time = now;
            }
        }
    }

    state
        .bubble
        .set_preview_secs(snap.config.notification_preview_secs);
    let bubble_draining = state.bubble.tick(now, hovered || state.expanded);
    if !state.expanded && state.bubble.active.is_none() && state.return_expanded_after_bubble {
        state.return_expanded_after_bubble = false;
        state.expanded = true;
        state.active_section = 0;
        state.leave_since = None;
        ctx.request_repaint();
    }

    let utility_toast_active =
        !state.expanded && state.bubble.active.is_none() && state.utility_toast.is_some();

    let hover_t = anim::transition_tween(
        &ctx,
        id.with("hover-t"),
        (hovered && !state.expanded && state.bubble.active.is_none() && !utility_toast_active)
            || drop_hover_active,
    );

    let (raw_w, raw_h) = if drop_hover_active {
        (DROP_HOVER_SIZE.x, DROP_HOVER_SIZE.y)
    } else if drop_toast_active && !state.expanded {
        (DROP_FEEDBACK_SIZE.x, DROP_FEEDBACK_SIZE.y)
    } else if state.expanded {
        if state.shelf.open {
            (SHELF_PANEL_SIZE.x, SHELF_PANEL_SIZE.y)
        } else {
            (EXPANDED_SIZE.x, EXPANDED_SIZE.y)
        }
    } else if state.bubble.active.is_some() {
        if hovered {
            (BUBBLE_HOVER_SIZE.x, BUBBLE_HOVER_SIZE.y)
        } else {
            (BUBBLE_COMPACT_SIZE.x, BUBBLE_COMPACT_SIZE.y)
        }
    } else if let Some(ref toast) = state.utility_toast {
        if toast.is_timer_complete {
            (336.0, 62.0)
        } else {
            (252.0, 46.0)
        }
    } else {
        (
            compact_size.x + if hovered { PILL_HOVER_SWELL.x } else { 0.0 },
            PILL_COMPACT_HEIGHT + if hovered { PILL_HOVER_SWELL.y } else { 0.0 },
        )
    };
    let max_w = (window.width() - 16.0).max(PILL_MIN_WIDTH);
    let max_h = (window.height() - PILL_TOP_MARGIN - 8.0).max(PILL_COMPACT_HEIGHT);
    let target_w = raw_w.min(max_w);
    let target_h = raw_h.min(max_h);

    let opening =
        state.expanded || state.bubble.active.is_some() || utility_toast_active || hovered;
    let (w, w_moving) = anim::transition_value(&ctx, id.with("w"), target_w, opening);
    let (h, h_moving) = anim::transition_value(&ctx, id.with("h"), target_h, opening);
    let target_x = target_x_for_placement(window, target_w, snap.config.placement);
    let (x, x_moving) = anim::transition_value(&ctx, id.with("x"), target_x, opening);

    let open_t = ((w - compact_size.x) / (EXPANDED_SIZE.x - compact_size.x).max(1.0)).clamp(0.0, 1.0);
    let radius = if drop_hover_active {
        DROP_HOVER_RADIUS
    } else if drop_toast_active && !state.expanded {
        DROP_FEEDBACK_RADIUS
    } else if state.expanded {
        let target_r = if state.shelf.open { SHELF_RADIUS } else { EXPANDED_RADIUS };
        (h * 0.5) + (target_r - h * 0.5) * open_t
    } else if state.bubble.active.is_some() {
        let bt = ((h - BUBBLE_COMPACT_SIZE.y) / (BUBBLE_HOVER_SIZE.y - BUBBLE_COMPACT_SIZE.y).max(1.0))
            .clamp(0.0, 1.0);
        BUBBLE_COMPACT_RADIUS + (BUBBLE_HOVER_RADIUS - BUBBLE_COMPACT_RADIUS) * bt
    } else if utility_toast_active {
        24.0
    } else {
        h * 0.5
    };
    let drop_t = anim::entry_drop_progress(&ctx, id, true);
    let drop_y = -28.0 * (1.0 - drop_t);
    let hover_float_y = if hovered
        && !state.expanded
        && state.bubble.active.is_none()
        && !utility_toast_active
        && !drop_hover_active
    {
        1.0 * hover_t
    } else {
        0.0
    };
    let rect = Rect::from_min_size(
        pos2(x, window.min.y + PILL_TOP_MARGIN + drop_y + hover_float_y),
        vec2(w, h),
    );
    let shape = Squircle::new(rect, radius.min(h * 0.5).max(12.0)).with_exponent(2.0);
    ctx.data_mut(|d| d.insert_temp(id.with("shape"), shape));

    if opaque_surface {
        pill::paint_opaque_shell(ui.painter(), &shape, theme);
    } else if !live_glass {
        pill::paint_tinted_fallback(ui.painter(), &shape, theme, hover_t);
    } else {
        pill::paint_glass_frame(ui.painter(), &shape, theme, hover_t);
    }

    if drop_hover_active {
        paint_drop_zone(ui.painter(), rect, theme, hovered_file_count);
    } else if drop_toast_active && !state.expanded {
        if let Some((ref msg, is_err, _)) = state.drop_feedback {
            paint_drop_feedback(ui.painter(), rect, theme, msg, is_err);
        }
    } else if !state.expanded
        && let Some(active_bubble) = state.bubble.active.clone()
    {
        if w >= BUBBLE_COMPACT_SIZE.x - 40.0 && h >= BUBBLE_COMPACT_SIZE.y - 10.0 {
            bubble::show(
                &ctx,
                ui,
                rect,
                theme,
                &active_bubble,
                state.bubble.queue.len(),
                hovered,
                &mut actions,
            );
        }
    } else if !state.expanded
        && let Some(toast) = state.utility_toast.clone()
    {
        if w >= 210.0 && h >= 40.0 {
            paint_utility_toast(&ctx, ui, rect, theme, &toast, &mut actions);
        }
    } else if !state.expanded && open_t < 0.28 {
        let compact_alpha = (1.0 - open_t / 0.28).clamp(0.0, 1.0);
        pill::paint_compact_contents(ui.painter(), rect, theme, &pill_data, compact_alpha);
        if ui.interact(rect, id.with("pill-click"), Sense::click()).clicked() {
            state.expanded = true;
            state.active_section = 0;
            state.leave_since = None;
            ctx.request_repaint();
        }
    } else if state.expanded && open_t > 0.25 {
        let reveal_t = ((open_t - 0.25) / 0.75).clamp(0.0, 1.0);
        if state.shelf.open {
            shelf::show(
                &ctx,
                ui,
                rect,
                &snap.shelf,
                &snap.config.shelf_storage_mode,
                shelf_thumbs,
                theme,
                &mut state.shelf,
                reveal_t,
                &mut actions,
            );
        } else {
            paint_section_dots(
                ui.painter(),
                rect,
                theme,
                sections.len(),
                state.active_section,
                reveal_t,
            );
            match current_section {
                SectionKind::Home => home::show(
                    &ctx,
                    ui,
                    rect,
                    snap,
                    theme,
                    state,
                    media_art,
                    reveal_t,
                    now_unix_ms,
                    &mut actions,
                ),
                SectionKind::MultiTask => multitask::show(
                    &ctx,
                    ui,
                    rect,
                    snap,
                    theme,
                    reveal_t,
                    now_unix_ms,
                    &mut state.active_section,
                    &mut actions,
                ),
                SectionKind::Calendar => calendar::show(
                    &ctx,
                    ui,
                    rect,
                    snap,
                    theme,
                    state,
                    reveal_t,
                    &mut actions,
                ),
                SectionKind::Hardware => hardware::show(
                    &ctx,
                    ui,
                    rect,
                    snap,
                    theme,
                    state,
                    reveal_t,
                    &mut actions,
                ),
            }
        }
    }

    let anim_active = w_moving || h_moving || x_moving || drop_t < 0.999;
    PanelOutput {
        shape,
        hovered,
        hover_t,
        anim_active,
        bubble_draining,
        current_section,
        actions,
    }
}

/// Applies per-tile `TSTransform` (open 0.96..1.0 scale, stagger slide, hover lift, press 0.97 scale)
/// at the `Area` layer level so both visual drawing and pointer hit-testing transform identically.
pub fn with_tile_transform(
    ctx: &Context,
    key: &'static str,
    rect: Rect,
    index: usize,
    reveal_t: f32,
    add_contents: impl FnOnce(&mut Ui),
) {
    let tile_id = Id::new(("orbit-tile", key));
    let area = Area::new(tile_id)
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .movable(false)
        .interactable(true);
    let layer_id = area.layer();

    let prev_hovered = ctx
        .data(|d| d.get_temp::<bool>(tile_id.with("hov")))
        .unwrap_or(false);
    let prev_pressed = ctx
        .data(|d| d.get_temp::<bool>(tile_id.with("prs")))
        .unwrap_or(false);

    let delay = (index as f32 * 0.05).min(0.4);
    let local_t = ((reveal_t - delay) / (1.0 - delay).max(0.2)).clamp(0.0, 1.0);
    let lift_t = anim::tween(ctx, tile_id.with("lift"), prev_hovered && !prev_pressed, 0.12);
    let press_t = anim::tween(ctx, tile_id.with("press"), prev_pressed, 0.08);

    let scale = (0.96 + 0.04 * local_t) * (1.0 - 0.03 * press_t);
    let delta = vec2(0.0, (1.0 - local_t) * 8.0 - 1.5 * lift_t);
    let pivot = rect.center();
    let ts = TSTransform::new(pivot.to_vec2() * (1.0 - scale) + delta, scale);
    ctx.set_transform_layer(layer_id, ts);

    area.show(ctx, |tui| {
        tui.set_clip_rect(rect.expand(4.0));
        let now_hov = tui.rect_contains_pointer(rect);
        let now_prs = now_hov && tui.input(|i| i.pointer.primary_down());
        ctx.data_mut(|d| {
            d.insert_temp(tile_id.with("hov"), now_hov);
            d.insert_temp(tile_id.with("prs"), now_prs);
        });
        add_contents(tui);
    });
}

fn paint_section_dots(
    painter: &egui::Painter,
    rect: Rect,
    theme: &Theme,
    count: usize,
    active: usize,
    alpha: f32,
) {
    if count <= 1 || alpha <= 0.01 {
        return;
    }
    let spacing = 10.0;
    let total_h = (count - 1) as f32 * spacing;
    let x = rect.max.x - 9.5;
    let y0 = rect.center().y - total_h * 0.5;

    for idx in 0..count {
        let cy = y0 + idx as f32 * spacing;
        let is_active = idx == active;
        if is_active {
            painter.circle_filled(
                pos2(x, cy),
                4.2,
                theme::with_alpha(theme.accent_blue, 0.30 * alpha),
            );
            painter.circle_filled(
                pos2(x, cy),
                2.5,
                theme::with_alpha(theme.pill_text, 0.96 * alpha),
            );
        } else {
            painter.circle_filled(
                pos2(x, cy),
                2.2,
                theme::with_alpha(theme.pill_text, 0.28 * alpha),
            );
        }
    }
}

fn paint_drop_zone(painter: &egui::Painter, rect: Rect, theme: &Theme, file_count: usize) {
    let inner = rect.shrink(9.0);
    if inner.width() < 80.0 || inner.height() < 32.0 {
        return;
    }
    painter.rect_filled(inner, 16.0, theme::with_alpha(theme.accent_blue, 0.14));
    painter.rect_stroke(
        inner,
        16.0,
        Stroke::new(1.5, theme::with_alpha(theme.accent_blue, 0.85)),
        egui::StrokeKind::Inside,
    );
    let ig = painter.layout_no_wrap(
        icons_fill::CLOUD_ARROW_UP.into(),
        pill::icon_font(24.0, true),
        theme.accent_blue,
    );
    let label = if file_count <= 1 {
        "Drop file to add to Orbit Shelf".to_string()
    } else {
        format!("Drop {file_count} files to add to Orbit Shelf")
    };
    let lg = painter.layout_no_wrap(label, Theme::font_semibold(12.5), theme.pill_text);
    let sub = painter.layout_no_wrap(
        "Executable scripts (.js, .exe, .bat) are blocked".into(),
        Theme::font(10.0),
        theme.pill_text_dim,
    );
    let total_h = ig.size().y + 4.0 + lg.size().y + 2.0 + sub.size().y;
    let mut y = inner.center().y - total_h * 0.5;
    let cx = inner.center().x;
    painter.galley(pos2(cx - ig.size().x * 0.5, y), ig, theme.accent_blue);
    y += 26.0;
    painter.galley(pos2(cx - lg.size().x * 0.5, y), lg, theme.pill_text);
    y += 17.0;
    painter.galley(pos2(cx - sub.size().x * 0.5, y), sub, theme.pill_text_dim);
}

fn paint_drop_feedback(
    painter: &egui::Painter,
    rect: Rect,
    theme: &Theme,
    msg: &str,
    is_error: bool,
) {
    let (glyph, accent) = if is_error {
        (icons_fill::SHIELD_WARNING, Color32::from_rgb(248, 113, 113))
    } else {
        (icons_fill::CHECK_CIRCLE, theme.accent_green)
    };
    let inner = rect.shrink2(vec2(14.0, 6.0));
    if inner.width() < 40.0 {
        return;
    }
    let ig = painter.layout_no_wrap(glyph.into(), pill::icon_font(16.0, true), accent);
    let mut job = egui::text::LayoutJob::single_section(
        msg.to_owned(),
        egui::TextFormat {
            font_id: Theme::font_semibold(12.0),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: (inner.width() - ig.size().x - 10.0).max(30.0),
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let tg = painter.layout_job(job);
    let total_w = ig.size().x + 8.0 + tg.size().x;
    let x0 = inner.center().x - total_w * 0.5;
    let cy = inner.center().y;
    painter.galley(pos2(x0, cy - ig.size().y * 0.5), ig, accent);
    painter.galley(pos2(x0 + 24.0, cy - tg.size().y * 0.5), tg, theme.pill_text);
}

fn utility_alert_icon(kind: UtilityAlertKind) -> &'static str {
    match kind {
        UtilityAlertKind::PowerConnected => icons_fill::PLUG_CHARGING,
        UtilityAlertKind::PowerDisconnected => icons_fill::BATTERY_MEDIUM,
        UtilityAlertKind::WifiConnected => icons_fill::WIFI_HIGH,
        UtilityAlertKind::WifiDisconnected => icons_fill::WIFI_SLASH,
        UtilityAlertKind::BluetoothOn => icons_fill::BLUETOOTH,
        UtilityAlertKind::BluetoothOff => icons_fill::BLUETOOTH_SLASH,
        UtilityAlertKind::LocationActive => icons_fill::MAP_PIN,
        UtilityAlertKind::TimerComplete => icons_fill::TIMER,
    }
}

fn paint_utility_toast(
    ctx: &Context,
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    toast: &UtilityToast,
    actions: &mut Vec<PanelAction>,
) {
    let inner = rect.shrink2(vec2(12.0, 7.0));
    if inner.width() < 60.0 || inner.height() < 24.0 {
        return;
    }

    if toast.is_timer_complete {
        let badge_rect = Rect::from_min_size(
            pos2(inner.min.x + 2.0, rect.center().y - 17.0),
            vec2(34.0, 34.0),
        );
        let done_r = Rect::from_min_size(
            pos2(inner.max.x - 46.0, rect.center().y - 12.0),
            vec2(46.0, 24.0),
        );
        let plus5_r = Rect::from_min_size(
            pos2(done_r.min.x - 6.0 - 42.0, rect.center().y - 12.0),
            vec2(42.0, 24.0),
        );

        let plus5_resp = ui.interact(plus5_r, Id::new("utility-timer-plus5"), Sense::click());
        let done_resp = ui.interact(done_r, Id::new("utility-timer-done"), Sense::click());

        let pulse = 0.5 + 0.5 * (ctx.input(|i| i.time) * 4.5).sin() as f32;
        let p = ui.painter();

        // Pulsing accent ring around 34x34 timer icon badge.
        p.rect_stroke(
            badge_rect.expand(1.5 + 1.5 * pulse),
            11.5,
            Stroke::new(1.5, theme::with_alpha(toast.accent, 0.35 + 0.40 * pulse)),
            egui::StrokeKind::Outside,
        );
        p.rect_filled(badge_rect.expand(1.5), 11.0, theme::with_alpha(toast.accent, 0.20));
        paint::card_3d(
            p,
            badge_rect,
            10.0,
            theme.card_top,
            theme.card_bottom,
            theme::mix(theme.card_border, toast.accent, 0.45),
            theme.card_highlight,
            Some(toast.accent),
            false,
        );
        let ig = p.layout_no_wrap(
            icons_fill::TIMER.into(),
            pill::icon_font(17.0, true),
            toast.accent,
        );
        p.galley(badge_rect.center() - ig.size() * 0.5, ig, toast.accent);

        let text_x = badge_rect.max.x + 10.0;
        let text_max_w = (plus5_r.min.x - 8.0 - text_x).max(40.0);
        let has_detail = !toast.detail.trim().is_empty();

        let mut title_job = egui::text::LayoutJob::single_section(
            "Focus Timer Complete!".to_owned(),
            egui::TextFormat {
                font_id: Theme::font_semibold(11.5),
                color: theme.pill_text,
                ..Default::default()
            },
        );
        title_job.wrap = egui::text::TextWrapping {
            max_width: text_max_w,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let tg = p.layout_job(title_job);
        let title_y = if has_detail {
            rect.center().y - tg.size().y - 0.5
        } else {
            rect.center().y - tg.size().y * 0.5
        };
        p.galley(pos2(text_x, title_y), tg, theme.pill_text);

        if has_detail {
            let mut detail_job = egui::text::LayoutJob::single_section(
                toast.detail.clone(),
                egui::TextFormat {
                    font_id: Theme::font(10.5),
                    color: theme.pill_text_dim,
                    ..Default::default()
                },
            );
            detail_job.wrap = egui::text::TextWrapping {
                max_width: text_max_w,
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let dg = p.layout_job(detail_job);
            p.galley(pos2(text_x, rect.center().y + 1.5), dg, theme.pill_text_dim);
        }

        // [+5m] and [Done] neumorphic buttons.
        paint::neu_button(
            p,
            plus5_r,
            8.5,
            theme::mix(toast.accent, Color32::WHITE, 0.08),
            theme::mix(toast.accent, Color32::BLACK, 0.18),
            theme::with_alpha(toast.accent, 0.65),
            Some(toast.accent),
            plus5_resp.hovered(),
            plus5_resp.is_pointer_button_down_on(),
        );
        let p5g = p.layout_no_wrap("+5m".into(), Theme::font_semibold(10.0), Color32::WHITE);
        p.galley(plus5_r.center() - p5g.size() * 0.5, p5g, Color32::WHITE);

        paint::neu_button(
            p,
            done_r,
            8.5,
            theme.card_top,
            theme.card_bottom,
            theme.card_border,
            None,
            done_resp.hovered(),
            done_resp.is_pointer_button_down_on(),
        );
        let done_g = p.layout_no_wrap("Done".into(), Theme::font_semibold(10.0), theme.pill_text);
        p.galley(done_r.center() - done_g.size() * 0.5, done_g, theme.pill_text);

        if plus5_resp.clicked() {
            actions.push(PanelAction::StartFocusTimer(300));
            actions.push(PanelAction::DismissUtilityToast);
        } else if done_resp.clicked() {
            actions.push(PanelAction::DismissUtilityToast);
        }
        return;
    }

    // Normal utility alert toast (252 x 46).
    let badge_rect = Rect::from_min_size(
        pos2(inner.min.x, rect.center().y - 14.0),
        vec2(28.0, 28.0),
    );
    let p = ui.painter();
    p.rect_filled(badge_rect.expand(1.5), 10.0, theme::with_alpha(toast.accent, 0.22));
    paint::card_3d(
        p,
        badge_rect,
        9.0,
        theme.card_top,
        theme.card_bottom,
        theme::mix(theme.card_border, toast.accent, 0.38),
        theme.card_highlight,
        Some(toast.accent),
        false,
    );
    let ig = p.layout_no_wrap(
        utility_alert_icon(toast.kind).into(),
        pill::icon_font(14.5, true),
        toast.accent,
    );
    p.galley(badge_rect.center() - ig.size() * 0.5, ig, toast.accent);

    let text_x = badge_rect.max.x + 10.0;
    let text_max_w = (inner.max.x - text_x).max(40.0);
    let has_detail = !toast.detail.trim().is_empty();

    let mut title_job = egui::text::LayoutJob::single_section(
        toast.title.clone(),
        egui::TextFormat {
            font_id: Theme::font_semibold(11.5),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    title_job.wrap = egui::text::TextWrapping {
        max_width: text_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let tg = p.layout_job(title_job);
    let title_y = if has_detail {
        rect.center().y - tg.size().y
    } else {
        rect.center().y - tg.size().y * 0.5
    };
    p.galley(pos2(text_x, title_y), tg, theme.pill_text);

    if has_detail {
        let mut detail_job = egui::text::LayoutJob::single_section(
            toast.detail.clone(),
            egui::TextFormat {
                font_id: Theme::font(10.0),
                color: theme.pill_text_dim,
                ..Default::default()
            },
        );
        detail_job.wrap = egui::text::TextWrapping {
            max_width: text_max_w,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let dg = p.layout_job(detail_job);
        p.galley(pos2(text_x, rect.center().y + 1.0), dg, theme.pill_text_dim);
    }

    if ui
        .interact(rect, Id::new("utility-toast-dismiss"), Sense::click())
        .clicked()
    {
        actions.push(PanelAction::DismissUtilityToast);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_core::config::Config;
    use orbit_core::events::{ClockStatus, FocusTimerState, PrivacyState, SystemSnapshot};
    use orbit_core::store::AppStore;

    fn dummy_store() -> AppStore {
        AppStore::new(
            Config::default(),
            SystemSnapshot::default(),
            ClockStatus::default(),
            false,
        )
    }

    #[test]
    fn sections_default_and_multitask_threshold() {
        let mut store = dummy_store();
        assert_eq!(
            sections_for(store.snapshot()),
            vec![SectionKind::Home, SectionKind::Calendar, SectionKind::Hardware]
        );

        // One active feature keeps 3 sections.
        store.apply(orbit_core::events::CoreEvent::FocusTimerUpdated(
            FocusTimerState {
                label: "Focus".into(),
                total_secs: 300,
                deadline_unix_ms: Some(9_999_999_999_999),
                paused_remaining_secs: None,
            },
        ));
        assert_eq!(sections_for(store.snapshot()).len(), 3);

        // Second active feature activates MultiTask.
        store.apply(orbit_core::events::CoreEvent::PrivacyUpdated(PrivacyState {
            camera_active: true,
            ..Default::default()
        }));
        assert_eq!(
            sections_for(store.snapshot()),
            vec![
                SectionKind::Home,
                SectionKind::MultiTask,
                SectionKind::Calendar,
                SectionKind::Hardware
            ]
        );
    }

    #[test]
    fn wheel_step_clamps_without_wrapping() {
        assert_eq!(apply_wheel_step(0, 3, 1.0), 0);
        assert_eq!(apply_wheel_step(0, 3, -1.0), 1);
        assert_eq!(apply_wheel_step(1, 3, -1.0), 2);
        assert_eq!(apply_wheel_step(2, 3, -1.0), 2);
        assert_eq!(apply_wheel_step(2, 3, 1.0), 1);
    }
}
