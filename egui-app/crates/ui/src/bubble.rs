//! Gate 4 Notification Bubble (display only):
//! - Compact 336x62 capsule with neumorphic brand badge, single-line title + body, inline close, and 3D progress bar.
//! - Hover 388x128 expansion pausing countdown, showing up to 3 lines, [Open], [Clear All], and [Dismiss].
//! - Strictly bounded queue (max 20, dropping oldest) and zero notification text written to disk.

use crate::anim;
use crate::paint;
use crate::panel::{PanelAction, with_tile_transform};
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, Context, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};
use orbit_core::events::{NotificationItem, NotificationSnapshot};
use std::collections::{HashSet, VecDeque};

pub const BUBBLE_COMPACT_SIZE: Vec2 = vec2(336.0, 62.0);
pub const BUBBLE_HOVER_SIZE: Vec2 = vec2(388.0, 128.0);
pub const BUBBLE_COMPACT_RADIUS: f32 = 26.0;
pub const BUBBLE_HOVER_RADIUS: f32 = 24.0;
pub const BUBBLE_LIFETIME_SECS: f32 = 5.0;
pub const MAX_BUBBLE_QUEUE: usize = 20;
const MAX_SEEN_HISTORY: usize = 128;

#[derive(Clone, Debug)]
pub struct ActiveBubble {
    pub item: NotificationItem,
    pub total_secs: f32,
    pub remaining_secs: f32,
    last_tick_time: Option<f64>,
}

impl ActiveBubble {
    fn with_lifetime(item: NotificationItem, lifetime_secs: f32) -> Self {
        let total_secs = lifetime_secs.clamp(2.0, 15.0);
        Self {
            item,
            total_secs,
            remaining_secs: total_secs,
            last_tick_time: None,
        }
    }

    pub fn progress_fraction(&self) -> f32 {
        (self.remaining_secs / self.total_secs.max(1.0)).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct BubbleState {
    seeded: bool,
    seen_ids: HashSet<u32>,
    seen_order: VecDeque<u32>,
    pub queue: VecDeque<NotificationItem>,
    pub active: Option<ActiveBubble>,
    pub preview_secs: f32,
}

impl BubbleState {
    pub fn set_preview_secs(&mut self, secs: u32) {
        self.preview_secs = (secs as f32).clamp(2.0, 15.0);
    }

    fn effective_lifetime(&self) -> f32 {
        if self.preview_secs >= 2.0 {
            self.preview_secs.clamp(2.0, 15.0)
        } else {
            BUBBLE_LIFETIME_SECS
        }
    }

    pub fn ingest_snapshot(
        &mut self,
        snap: &NotificationSnapshot,
        notifications_enabled: bool,
        suppressed: bool,
    ) {
        if !self.seeded {
            self.seeded = true;
            for item in &snap.notifications {
                self.record_seen(item.id);
            }
            return;
        }

        // Prune active/queued notifications that were externally dismissed.
        let present: HashSet<u32> = snap.notifications.iter().map(|n| n.id).collect();
        self.queue.retain(|q| present.contains(&q.id));
        if let Some(ref active) = self.active
            && !present.contains(&active.item.id)
        {
            let life = self.effective_lifetime();
            self.active = self
                .queue
                .pop_front()
                .map(|item| ActiveBubble::with_lifetime(item, life));
        }

        // Snapshots are newest-first; enqueue newly observed IDs oldest-to-newest so chronological order holds.
        let mut incoming: Vec<NotificationItem> = snap
            .notifications
            .iter()
            .filter(|n| !self.seen_ids.contains(&n.id))
            .cloned()
            .collect();
        incoming.reverse();

        for item in incoming {
            self.record_seen(item.id);
            if notifications_enabled && !suppressed {
                self.enqueue(item);
            }
        }
    }

    pub fn enqueue(&mut self, item: NotificationItem) {
        if self.active.is_none() {
            let life = self.effective_lifetime();
            self.active = Some(ActiveBubble::with_lifetime(item, life));
            return;
        }
        if let Some(active) = self.active.as_ref()
            && active.item.id == item.id
        {
            return;
        }
        if self.queue.iter().any(|q| q.id == item.id) {
            return;
        }
        self.queue.push_back(item);
        while self.queue.len() > MAX_BUBBLE_QUEUE {
            self.queue.pop_front();
        }
    }

    pub fn show_item_now(&mut self, item: NotificationItem) {
        self.record_seen(item.id);
        self.queue.retain(|q| q.id != item.id);
        let life = self.effective_lifetime();
        self.active = Some(ActiveBubble::with_lifetime(item, life));
    }

    pub fn show_all_pending(&mut self, items: &[NotificationItem]) {
        if items.is_empty() {
            return;
        }
        for item in items {
            self.record_seen(item.id);
        }
        let life = self.effective_lifetime();
        self.active = Some(ActiveBubble::with_lifetime(items[0].clone(), life));
        self.queue.clear();
        for item in items[1..].iter().take(MAX_BUBBLE_QUEUE) {
            self.queue.push_back(item.clone());
        }
    }

    pub fn dismiss_active(&mut self) -> Option<u32> {
        let prev = self.active.take().map(|b| b.item.id);
        let life = self.effective_lifetime();
        self.active = self
            .queue
            .pop_front()
            .map(|item| ActiveBubble::with_lifetime(item, life));
        prev
    }

    /// Advances the expiry countdown when `!paused`. Returns `true` while an active bubble is draining.
    pub fn tick(&mut self, now: f64, paused: bool) -> bool {
        let life = self.effective_lifetime();
        let Some(ref mut active) = self.active else {
            return false;
        };
        if paused {
            active.last_tick_time = Some(now);
            return false;
        }
        let dt = active
            .last_tick_time
            .map_or(0.0, |prev| (now - prev).clamp(0.0, 0.25) as f32);
        active.last_tick_time = Some(now);
        active.remaining_secs -= dt;
        if active.remaining_secs <= 0.0 {
            self.active = self
                .queue
                .pop_front()
                .map(|item| ActiveBubble::with_lifetime(item, life));
            return self.active.is_some();
        }
        true
    }

    fn record_seen(&mut self, id: u32) {
        if self.seen_ids.insert(id) {
            self.seen_order.push_back(id);
            while self.seen_order.len() > MAX_SEEN_HISTORY {
                if let Some(old) = self.seen_order.pop_front() {
                    self.seen_ids.remove(&old);
                }
            }
        }
    }
}

/// Paints compact or hovered notification bubble content inside `rect`.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &Context,
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    active: &ActiveBubble,
    queue_len: usize,
    hover_expanded: bool,
    actions: &mut Vec<PanelAction>,
) {
    let item = &active.item;
    let expand_t = anim::transition_tween(ctx, egui::Id::new("bubble-hover-expand"), hover_expanded);
    let compact = !hover_expanded || rect.height() < 86.0;

    let pad_x = if compact { 14.0 } else { 16.0 };
    let pad_y = if compact { 8.0 } else { 10.0 };
    let inner = Rect::from_min_max(
        pos2(rect.min.x + pad_x, rect.min.y + pad_y),
        pos2(rect.max.x - pad_x, rect.max.y - pad_y),
    );

    if compact {
        show_compact_layout(
            ui,
            inner,
            rect,
            theme,
            item,
            queue_len,
            active.progress_fraction(),
            actions,
        );
    } else {
        show_hover_layout(
            ctx,
            inner,
            theme,
            item,
            queue_len,
            expand_t.clamp(0.25, 1.0),
            actions,
        );
    }
}

fn activate_action(item: &NotificationItem) -> PanelAction {
    PanelAction::ActivateNotification {
        id: item.id,
        app_id: item.app_id.clone(),
        app_name: item.app_name.clone(),
        title: item.title.clone(),
    }
}

/// Measures and optionally paints a subtle pill chip for the app name, returning its width.
fn layout_app_chip(
    painter: &egui::Painter,
    app_name: &str,
    max_w: f32,
    theme: &Theme,
) -> Option<(std::sync::Arc<egui::Galley>, Vec2)> {
    let trimmed = app_name.trim();
    if trimmed.is_empty() || max_w < 28.0 {
        return None;
    }
    let mut job = egui::text::LayoutJob::single_section(
        trimmed.to_owned(),
        egui::TextFormat {
            font_id: Theme::font_semibold(9.5),
            color: theme.pill_text_dim,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: (max_w - 10.0).max(16.0),
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let galley = painter.layout_job(job);
    let size = vec2(galley.size().x + 10.0, 15.0);
    Some((galley, size))
}

fn paint_app_chip(
    painter: &egui::Painter,
    pos: Pos2,
    galley: std::sync::Arc<egui::Galley>,
    size: Vec2,
    theme: &Theme,
) {
    let chip_r = Rect::from_min_size(pos, size);
    let fill = theme::with_alpha(theme.pill_text, if theme.is_light { 0.08 } else { 0.11 });
    painter.rect_filled(chip_r, 5.0, fill);
    painter.rect_stroke(
        chip_r,
        5.0,
        Stroke::new(0.75, theme.card_border),
        egui::StrokeKind::Inside,
    );
    painter.galley(chip_r.center() - galley.size() * 0.5, galley, theme.pill_text_dim);
}

#[allow(clippy::too_many_arguments)]
fn show_compact_layout(
    ui: &mut Ui,
    inner: Rect,
    shell_rect: Rect,
    theme: &Theme,
    item: &NotificationItem,
    queue_len: usize,
    progress: f32,
    actions: &mut Vec<PanelAction>,
) {
    // Inline circular neumorphic close button on top row / right margin, interacted BEFORE activate_rect.
    let close_r = Rect::from_min_size(pos2(inner.max.x - 20.0, inner.min.y + 1.0), vec2(20.0, 20.0));
    let close_resp = ui.interact(
        close_r,
        ui.id().with(("notif-compact-close", item.id)),
        Sense::click(),
    );
    let close_hovered = close_resp.hovered();
    let close_pressed = close_resp.is_pointer_button_down_on();
    let rose = Color32::from_rgb(248, 113, 113);
    paint::neu_button(
        ui.painter(),
        close_r,
        10.0,
        theme.card_top,
        theme.card_bottom,
        if close_hovered { rose } else { theme.card_border },
        close_hovered.then_some(rose),
        close_hovered,
        close_pressed,
    );
    let x_color = if close_hovered { rose } else { theme.pill_text_dim };
    let xg = ui
        .painter()
        .layout_no_wrap(icons::X.into(), icon_font(10.0, false), x_color);
    ui.painter()
        .galley(close_r.center() - xg.size() * 0.5, xg, x_color);
    if close_resp.clicked() {
        actions.push(PanelAction::DismissNotification(item.id));
    }

    // Left 34x34 neumorphic app badge with brand halo and optional +N queue count pill.
    let badge_rect = Rect::from_min_size(pos2(inner.min.x, inner.min.y + 3.0), vec2(34.0, 34.0));
    let p = ui.painter();
    let brand_fg = paint_brand_badge(p, badge_rect, item, theme, (queue_len > 0).then_some(queue_len));

    let text_x = badge_rect.max.x + 12.0;
    let top_max_w = (close_r.min.x - 8.0 - text_x).max(40.0);

    // Middle top row: bright title (12.0px) + compact subtle pill chip for app_name.
    let chip = layout_app_chip(p, &item.app_name, (top_max_w * 0.36).min(88.0), theme);
    let title_max_w = match &chip {
        Some((_, csize)) => (top_max_w - csize.x - 6.0).max(36.0),
        None => top_max_w,
    };
    let mut title_job = egui::text::LayoutJob::single_section(
        item.title.clone(),
        egui::TextFormat {
            font_id: Theme::font_semibold(12.0),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    title_job.wrap = egui::text::TextWrapping {
        max_width: title_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let tg = p.layout_job(title_job);
    let title_y = inner.min.y + 1.0;
    p.galley(pos2(text_x, title_y), tg.clone(), theme.pill_text);
    if let Some((cg, csize)) = chip {
        let chip_x = (text_x + tg.size().x + 6.0).min(close_r.min.x - 8.0 - csize.x);
        let chip_y = title_y + (tg.size().y - csize.y) * 0.5;
        paint_app_chip(p, pos2(chip_x, chip_y), cg, csize, theme);
    }

    // Middle second row: bright readable single-line body (11.0px).
    let body_max_w = (close_r.min.x - 6.0 - text_x).max(40.0);
    let body_text = if item.body.trim().is_empty() {
        "Notification"
    } else {
        item.body.as_str()
    };
    let mut body_job = egui::text::LayoutJob::single_section(
        body_text.replace(['\r', '\n'], " "),
        egui::TextFormat {
            font_id: Theme::font(11.0),
            color: theme.pill_text_dim,
            ..Default::default()
        },
    );
    body_job.wrap = egui::text::TextWrapping {
        max_width: body_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let bg = p.layout_job(body_job);
    p.galley(pos2(text_x, inner.min.y + 19.5), bg, theme.pill_text_dim);

    // Bottom sleek 3D glowing drain bar (3.0px height).
    let bar_r = Rect::from_min_max(
        pos2(text_x, shell_rect.max.y - 9.5),
        pos2(inner.max.x, shell_rect.max.y - 6.5),
    );
    paint::progress_bar_3d(p, bar_r, progress, brand_fg, theme.track_bg);

    // Activate area strictly excludes close_r on the right to prevent click fall-through.
    let activate_rect = Rect::from_min_max(shell_rect.min, pos2(close_r.min.x - 4.0, shell_rect.max.y));
    if ui
        .interact(
            activate_rect,
            ui.id().with(("notif-compact", item.id)),
            Sense::click(),
        )
        .clicked()
    {
        actions.push(activate_action(item));
    }
}

#[allow(clippy::too_many_arguments)]
fn show_hover_layout(
    ctx: &Context,
    inner: Rect,
    theme: &Theme,
    item: &NotificationItem,
    queue_len: usize,
    expand_t: f32,
    actions: &mut Vec<PanelAction>,
) {
    let header_area = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.max.y - 34.0));
    with_tile_transform(ctx, "notif-hover-body", header_area, 0, expand_t, |tui| {
        let badge_rect = Rect::from_min_size(
            pos2(header_area.min.x, header_area.min.y + 2.0),
            vec2(34.0, 34.0),
        );
        let p = tui.painter();
        paint_brand_badge(p, badge_rect, item, theme, None);

        let text_x = badge_rect.max.x + 12.0;

        // Optional queue indicator badge ("1 of N") on top-right when queue_len > 0.
        let mut right_limit = header_area.max.x;
        if queue_len > 0 {
            let q_label = format!("{} of {}", 1, queue_len + 1);
            let qg = p.layout_no_wrap(q_label, Theme::font_semibold(9.5), theme.accent_cyan);
            let qw = qg.size().x + 12.0;
            let q_rect = Rect::from_min_size(
                pos2(header_area.max.x - qw, header_area.min.y + 1.0),
                vec2(qw, 16.0),
            );
            p.rect_filled(q_rect, 8.0, theme::with_alpha(theme.accent_cyan, 0.16));
            p.rect_stroke(
                q_rect,
                8.0,
                Stroke::new(0.85, theme::with_alpha(theme.accent_cyan, 0.45)),
                egui::StrokeKind::Inside,
            );
            p.galley(q_rect.center() - qg.size() * 0.5, qg, theme.accent_cyan);
            right_limit = q_rect.min.x - 8.0;
        }

        let top_max_w = (right_limit - text_x).max(48.0);
        let chip = layout_app_chip(p, &item.app_name, (top_max_w * 0.34).min(94.0), theme);
        let title_max_w = match &chip {
            Some((_, csize)) => (top_max_w - csize.x - 6.0).max(40.0),
            None => top_max_w,
        };
        let mut title_job = egui::text::LayoutJob::single_section(
            item.title.clone(),
            egui::TextFormat {
                font_id: Theme::font_semibold(12.5),
                color: theme.pill_text,
                ..Default::default()
            },
        );
        title_job.wrap = egui::text::TextWrapping {
            max_width: title_max_w,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let tg = p.layout_job(title_job);
        let title_y = header_area.min.y + 1.0;
        p.galley(pos2(text_x, title_y), tg.clone(), theme.pill_text);
        if let Some((cg, csize)) = chip {
            let chip_x = (text_x + tg.size().x + 6.0).min(right_limit - csize.x);
            let chip_y = title_y + (tg.size().y - csize.y) * 0.5;
            paint_app_chip(p, pos2(chip_x, chip_y), cg, csize, theme);
        }

        // Subtle header divider under title row.
        let div_y = header_area.min.y + 21.0;
        p.line_segment(
            [pos2(text_x, div_y), pos2(header_area.max.x, div_y)],
            Stroke::new(0.85, theme.card_border),
        );

        // Body: up to 3 wrapped lines in crisp bright text (11.5px).
        let body_color = theme::mix(theme.pill_text_dim, theme.pill_text, 0.45);
        let body_text = if item.body.trim().is_empty() {
            "Notification"
        } else {
            item.body.as_str()
        };
        let mut body_job = egui::text::LayoutJob::single_section(
            body_text.to_owned(),
            egui::TextFormat {
                font_id: Theme::font(11.5),
                color: body_color,
                ..Default::default()
            },
        );
        body_job.wrap = egui::text::TextWrapping {
            max_width: (header_area.max.x - text_x).max(60.0),
            max_rows: 3,
            break_anywhere: false,
            ..Default::default()
        };
        let bg = p.layout_job(body_job);
        p.galley(pos2(text_x, header_area.min.y + 25.0), bg, body_color);

        if tui
            .interact(
                header_area,
                tui.id().with(("notif-hover-activate", item.id)),
                Sense::click(),
            )
            .clicked()
        {
            actions.push(activate_action(item));
        }
    });

    // Divider & bottom [Open] + optional [Clear All] + [Dismiss] action bar via neu_button.
    let btn_row = Rect::from_min_max(
        pos2(inner.min.x, inner.max.y - 28.0),
        pos2(inner.max.x, inner.max.y),
    );
    with_tile_transform(ctx, "notif-hover-actions", btn_row, 1, expand_t, |tui| {
        tui.painter().line_segment(
            [
                pos2(btn_row.min.x, btn_row.min.y - 4.0),
                pos2(btn_row.max.x, btn_row.min.y - 4.0),
            ],
            Stroke::new(1.0, theme.card_border),
        );

        let open_r = Rect::from_min_size(pos2(btn_row.min.x, btn_row.min.y + 2.0), vec2(68.0, 24.0));
        let oresp = tui.interact(open_r, tui.id().with(("notif-open", item.id)), Sense::click());
        paint::neu_button(
            tui.painter(),
            open_r,
            9.0,
            theme::mix(theme.accent_blue, Color32::WHITE, 0.08),
            theme::mix(theme.accent_blue, Color32::BLACK, 0.18),
            Color32::from_rgba_unmultiplied(190, 225, 255, 150),
            Some(theme.accent_blue),
            oresp.hovered(),
            oresp.is_pointer_button_down_on(),
        );
        let og = tui
            .painter()
            .layout_no_wrap("Open".into(), Theme::font_semibold(10.5), Color32::WHITE);
        tui.painter()
            .galley(open_r.center() - og.size() * 0.5, og, Color32::WHITE);
        if oresp.clicked() {
            actions.push(activate_action(item));
        }

        let dis_r = Rect::from_min_size(pos2(btn_row.max.x - 76.0, btn_row.min.y + 2.0), vec2(76.0, 24.0));
        if queue_len > 0 {
            let clear_r = Rect::from_min_size(
                pos2(dis_r.min.x - 74.0 - 8.0, btn_row.min.y + 2.0),
                vec2(74.0, 24.0),
            );
            let cresp = tui.interact(
                clear_r,
                tui.id().with(("notif-clear-all", item.id)),
                Sense::click(),
            );
            paint::neu_button(
                tui.painter(),
                clear_r,
                9.0,
                theme.card_top,
                theme.card_bottom,
                theme.card_border,
                None,
                cresp.hovered(),
                cresp.is_pointer_button_down_on(),
            );
            let clear_fg = if cresp.hovered() {
                theme.pill_text
            } else {
                theme.pill_text_dim
            };
            let cg = tui
                .painter()
                .layout_no_wrap("Clear All".into(), Theme::font_semibold(10.0), clear_fg);
            tui.painter()
                .galley(clear_r.center() - cg.size() * 0.5, cg, clear_fg);
            if cresp.clicked() {
                actions.push(PanelAction::ClearAllNotifications);
            }
        }

        let dresp = tui.interact(dis_r, tui.id().with(("notif-dismiss", item.id)), Sense::click());
        paint::neu_button(
            tui.painter(),
            dis_r,
            9.0,
            theme.pill_btn_bg,
            theme.card_bottom,
            theme.pill_btn_border,
            Some(theme.accent_blue),
            dresp.hovered(),
            dresp.is_pointer_button_down_on(),
        );
        let dg = tui.painter().layout_no_wrap(
            "Dismiss".into(),
            Theme::font_semibold(10.5),
            theme.pill_text,
        );
        tui.painter()
            .galley(dis_r.center() - dg.size() * 0.5, dg, theme.pill_text);
        if dresp.clicked() {
            actions.push(PanelAction::DismissNotification(item.id));
        }
    });
}

fn brand_identity(item: &NotificationItem, theme: &Theme) -> (&'static str, Color32, bool) {
    let lower = format!("{} {}", item.app_name, item.app_id).to_ascii_lowercase();
    if lower.contains("snipping") || lower.contains("screensketch") {
        (icons::SCISSORS, Color32::from_rgb(56, 152, 248), false)
    } else if lower.contains("whatsapp") || lower.contains("telegram") {
        (icons_fill::CHAT_CIRCLE, theme.accent_green, true)
    } else if lower.contains("chrome")
        || lower.contains("brave")
        || lower.contains("edge")
        || lower.contains("firefox")
    {
        (icons_fill::GLOBE, theme.accent_blue, true)
    } else if lower.contains("mail") || lower.contains("outlook") {
        (icons_fill::ENVELOPE, theme.accent_cyan, true)
    } else if lower.contains("terminal") || lower.contains("powershell") || lower.contains("code") {
        (icons_fill::TERMINAL, theme.accent_purple, true)
    } else {
        (icons_fill::BELL_SIMPLE, theme.accent_blue, true)
    }
}

fn paint_brand_badge(
    painter: &egui::Painter,
    rect: Rect,
    item: &NotificationItem,
    theme: &Theme,
    queue_badge: Option<usize>,
) -> Color32 {
    let (glyph, brand_fg, is_fill) = brand_identity(item, theme);

    // Subtle outer brand halo + 3D neumorphic badge surface.
    painter.rect_filled(rect.expand(2.0), 11.5, theme::with_alpha(brand_fg, 0.22));
    paint::card_3d(
        painter,
        rect,
        10.0,
        theme.card_top,
        theme.card_bottom,
        theme::mix(theme.card_border, brand_fg, 0.35),
        theme.card_highlight,
        Some(brand_fg),
        false,
    );

    let g = painter.layout_no_wrap(glyph.into(), icon_font(17.0, is_fill), brand_fg);
    painter.galley(rect.center() - g.size() * 0.5, g, brand_fg);

    if let Some(count) = queue_badge.filter(|&c| c > 0) {
        let label = if count > 99 {
            "+99".to_string()
        } else {
            format!("+{count}")
        };
        let fg = if theme.is_light {
            Color32::WHITE
        } else {
            Color32::from_rgb(8, 15, 28)
        };
        let cg = painter.layout_no_wrap(label, Theme::font_bold(8.5), fg);
        let pill_w = 14.0_f32.max(cg.size().x + 4.0);
        let pill_r = Rect::from_min_size(
            pos2(rect.max.x - pill_w + 4.0, rect.max.y - 10.0),
            vec2(pill_w, 13.0),
        );
        painter.rect_filled(pill_r, 6.5, theme.accent_cyan);
        painter.rect_stroke(
            pill_r,
            6.5,
            Stroke::new(1.0, theme.pill_bottom),
            egui::StrokeKind::Outside,
        );
        painter.galley(pill_r.center() - cg.size() * 0.5, cg, fg);
    }

    brand_fg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_item(id: u32) -> NotificationItem {
        NotificationItem {
            id,
            app_name: format!("App {id}"),
            app_id: format!("app{id}.exe"),
            title: format!("Title {id}"),
            body: format!("Body line {id}"),
            timestamp_ms: id as u64 * 1000,
            is_critical: false,
            count: 1,
            actions: Vec::new(),
            app_icon: None,
            underlying_ids: vec![id],
        }
    }

    #[test]
    fn queue_cap_25_in_a_row_keeps_latest_20_and_drops_oldest_5() {
        let mut state = BubbleState::default();
        // Send 25 distinct notifications sequentially while bubble 1 remains active.
        for id in 1..=25u32 {
            state.enqueue(mk_item(id));
        }
        assert_eq!(state.active.as_ref().unwrap().item.id, 1);
        assert_eq!(state.queue.len(), 20);
        // Enqueued items 2..=25 -> oldest 4 of the queue (2,3,4,5) dropped, keeping 6..=25.
        assert_eq!(state.queue.front().unwrap().id, 6);
        assert_eq!(state.queue.back().unwrap().id, 25);
    }

    #[test]
    fn bubble_auto_collapses_after_5_seconds_and_pauses_while_hovered() {
        let mut state = BubbleState::default();
        state.enqueue(mk_item(10));
        assert!(state.tick(10.0, false));
        // Hover pauses countdown completely over 10 seconds.
        assert!(!state.tick(10.2, true));
        assert!(!state.tick(20.0, true));
        assert_eq!(
            state.active.as_ref().unwrap().remaining_secs,
            BUBBLE_LIFETIME_SECS
        );

        // Advance unhovered cleanly up to 4.8s -> still active, then past 5.0s -> auto-collapses.
        let mut t = 20.0;
        for _ in 0..24 {
            t += 0.2;
            assert!(state.tick(t, false));
        }
        assert!(state.active.is_some());
        assert!(!state.tick(t + 0.3, false));
        assert!(state.active.is_none());
    }

    #[test]
    fn dismiss_clears_active_and_promotes_next() {
        let mut state = BubbleState::default();
        state.enqueue(mk_item(1));
        state.enqueue(mk_item(2));
        assert_eq!(state.dismiss_active(), Some(1));
        assert_eq!(state.active.as_ref().unwrap().item.id, 2);
        assert_eq!(state.dismiss_active(), Some(2));
        assert!(state.active.is_none());
    }

    #[test]
    fn show_all_pending_queues_all_notifications_sequentially() {
        let mut state = BubbleState::default();
        state.show_all_pending(&[]);
        assert!(state.active.is_none());

        let items = vec![mk_item(10), mk_item(20), mk_item(30)];
        state.show_all_pending(&items);
        assert_eq!(state.active.as_ref().unwrap().item.id, 10);
        assert_eq!(state.queue.len(), 2);
        assert_eq!(state.queue[0].id, 20);
        assert_eq!(state.queue[1].id, 30);

        assert_eq!(state.dismiss_active(), Some(10));
        assert_eq!(state.active.as_ref().unwrap().item.id, 20);
        assert_eq!(state.queue.len(), 1);

        assert_eq!(state.dismiss_active(), Some(20));
        assert_eq!(state.active.as_ref().unwrap().item.id, 30);
        assert!(state.queue.is_empty());

        assert_eq!(state.dismiss_active(), Some(30));
        assert!(state.active.is_none());

        // Capped at MAX_BUBBLE_QUEUE items beyond active.
        let many: Vec<NotificationItem> = (1..=25u32).map(mk_item).collect();
        state.show_all_pending(&many);
        assert_eq!(state.active.as_ref().unwrap().item.id, 1);
        assert_eq!(state.queue.len(), MAX_BUBBLE_QUEUE);
        assert_eq!(state.queue.front().unwrap().id, 2);
        assert_eq!(state.queue.back().unwrap().id, 21);
    }
}
