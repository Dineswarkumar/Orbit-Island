//! Gate 4 Notification Bubble (display only):
//! - Compact 316x56 capsule with app identity, single-line title + body, and 5s progress bar.
//! - Hover 368x112 expansion pausing countdown, showing up to 3 lines, [Open], and [Dismiss].
//! - Strictly bounded queue (max 20, dropping oldest) and zero notification text written to disk.

use crate::anim;
use crate::panel::{PanelAction, with_tile_transform};
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, Context, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};
use orbit_core::events::{NotificationItem, NotificationSnapshot};
use std::collections::{HashSet, VecDeque};

pub const BUBBLE_COMPACT_SIZE: Vec2 = vec2(316.0, 56.0);
pub const BUBBLE_HOVER_SIZE: Vec2 = vec2(368.0, 112.0);
pub const BUBBLE_COMPACT_RADIUS: f32 = 28.0;
pub const BUBBLE_HOVER_RADIUS: f32 = 22.0;
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
    hover_expanded: bool,
    actions: &mut Vec<PanelAction>,
) {
    let item = &active.item;
    let expand_t = anim::tween(ctx, egui::Id::new("bubble-hover-expand"), hover_expanded, 0.16);

    let pad_x = 16.0;
    let pad_y = if hover_expanded { 12.0 } else { 9.0 };
    let inner = Rect::from_min_max(
        pos2(rect.min.x + pad_x, rect.min.y + pad_y),
        pos2(rect.max.x - pad_x, rect.max.y - pad_y),
    );

    if !hover_expanded || rect.height() < 78.0 {
        show_compact_layout(ctx, ui, inner, rect, theme, item, active.progress_fraction(), actions);
    } else {
        show_hover_layout(ctx, inner, theme, item, expand_t.clamp(0.25, 1.0), actions);
    }
}

#[allow(clippy::too_many_arguments)]
fn show_compact_layout(
    ctx: &Context,
    ui: &mut Ui,
    inner: Rect,
    shell_rect: Rect,
    theme: &Theme,
    item: &NotificationItem,
    progress: f32,
    actions: &mut Vec<PanelAction>,
) {
    let badge_rect = Rect::from_min_size(
        pos2(inner.min.x, inner.min.y + 3.0),
        vec2(26.0, 26.0),
    );
    paint_brand_badge(ui.painter(), badge_rect, item, theme);

    let text_x = badge_rect.max.x + 11.0;
    let text_max_w = (inner.max.x - text_x).max(40.0);
    let p = ui.painter();

    // Title · AppName (1 line, truncated cleanly)
    let mut title_job = egui::text::LayoutJob::default();
    title_job.append(
        &item.title,
        0.0,
        egui::TextFormat {
            font_id: Theme::font_semibold(11.5),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    if !item.app_name.is_empty() {
        title_job.append(
            &format!("  · {}", item.app_name),
            0.0,
            egui::TextFormat {
                font_id: Theme::font(10.5),
                color: theme.pill_text_dim,
                ..Default::default()
            },
        );
    }
    title_job.wrap = egui::text::TextWrapping {
        max_width: text_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let tg = p.layout_job(title_job);
    p.galley(pos2(text_x, inner.min.y), tg, theme.pill_text);

    // Body line
    let body_text = if item.body.trim().is_empty() {
        "Notification"
    } else {
        item.body.as_str()
    };
    let mut body_job = egui::text::LayoutJob::single_section(
        body_text.replace(['\r', '\n'], " "),
        egui::TextFormat {
            font_id: Theme::font(10.5),
            color: theme.pill_text_dim,
            ..Default::default()
        },
    );
    body_job.wrap = egui::text::TextWrapping {
        max_width: text_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let bg = p.layout_job(body_job);
    p.galley(pos2(text_x, inner.min.y + 15.5), bg, theme.pill_text_dim);

    // Bottom 5-second drain bar matching screenshot 1.
    let bar_r = Rect::from_min_max(
        pos2(text_x, shell_rect.max.y - 9.0),
        pos2(inner.max.x, shell_rect.max.y - 6.5),
    );
    p.rect_filled(bar_r, 1.25, Color32::from_rgba_unmultiplied(255, 255, 255, 26));
    let fill_w = (bar_r.width() * progress).clamp(0.0, bar_r.width());
    if fill_w > 1.0 {
        p.rect_filled(
            Rect::from_min_size(bar_r.min, vec2(fill_w, bar_r.height())),
            1.25,
            theme.accent_blue,
        );
    }

    let _ = ctx;
    if ui
        .interact(shell_rect, ui.id().with(("notif-compact", item.id)), Sense::click())
        .clicked()
    {
        actions.push(PanelAction::ActivateNotification {
            id: item.id,
            app_id: item.app_id.clone(),
            app_name: item.app_name.clone(),
            title: item.title.clone(),
        });
    }
}

fn show_hover_layout(
    ctx: &Context,
    inner: Rect,
    theme: &Theme,
    item: &NotificationItem,
    expand_t: f32,
    actions: &mut Vec<PanelAction>,
) {
    let header_area = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.max.y - 36.0));
    with_tile_transform(ctx, "notif-hover-body", header_area, 0, expand_t, |tui| {
        let badge_rect = Rect::from_min_size(
            pos2(header_area.min.x, header_area.min.y + 4.0),
            vec2(28.0, 28.0),
        );
        paint_brand_badge(tui.painter(), badge_rect, item, theme);

        let text_x = badge_rect.max.x + 12.0;
        let text_max_w = (header_area.max.x - text_x).max(60.0);
        let p = tui.painter();

        let mut title_job = egui::text::LayoutJob::default();
        title_job.append(
            &item.title,
            0.0,
            egui::TextFormat {
                font_id: Theme::font_semibold(12.0),
                color: theme.pill_text,
                ..Default::default()
            },
        );
        if !item.app_name.is_empty() {
            title_job.append(
                &format!("  · {}", item.app_name),
                0.0,
                egui::TextFormat {
                    font_id: Theme::font(10.5),
                    color: theme.pill_text_dim,
                    ..Default::default()
                },
            );
        }
        title_job.wrap = egui::text::TextWrapping {
            max_width: text_max_w,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let tg = p.layout_job(title_job);
        p.galley(pos2(text_x, header_area.min.y), tg, theme.pill_text);

        let body_text = if item.body.trim().is_empty() {
            "Notification"
        } else {
            item.body.as_str()
        };
        let mut body_job = egui::text::LayoutJob::single_section(
            body_text.to_owned(),
            egui::TextFormat {
                font_id: Theme::font(11.0),
                color: theme.pill_text_dim,
                ..Default::default()
            },
        );
        body_job.wrap = egui::text::TextWrapping {
            max_width: text_max_w,
            max_rows: 3,
            break_anywhere: false,
            ..Default::default()
        };
        let bg = p.layout_job(body_job);
        p.galley(pos2(text_x, header_area.min.y + 17.0), bg, theme.pill_text_dim);

        if tui
            .interact(
                header_area,
                tui.id().with(("notif-hover-activate", item.id)),
                Sense::click(),
            )
            .clicked()
        {
            actions.push(PanelAction::ActivateNotification {
                id: item.id,
                app_id: item.app_id.clone(),
                app_name: item.app_name.clone(),
                title: item.title.clone(),
            });
        }
    });

    // Divider & bottom [Open] + [Dismiss] bar matching screenshot 6.
    let btn_row = Rect::from_min_max(
        pos2(inner.min.x, inner.max.y - 30.0),
        pos2(inner.max.x, inner.max.y),
    );
    with_tile_transform(ctx, "notif-hover-actions", btn_row, 1, expand_t, |tui| {
        tui.painter().line_segment(
            [pos2(btn_row.min.x, btn_row.min.y - 4.0), pos2(btn_row.max.x, btn_row.min.y - 4.0)],
            Stroke::new(1.0, theme.card_border),
        );

        let open_r = Rect::from_min_size(pos2(btn_row.min.x, btn_row.min.y + 2.0), vec2(66.0, 24.0));
        let oresp = tui.interact(open_r, tui.id().with(("notif-open", item.id)), Sense::click());
        let obg = if oresp.hovered() {
            Color32::from_rgb(58, 148, 255)
        } else {
            theme.accent_blue
        };
        tui.painter().rect_filled(open_r, 9.0, obg);
        tui.painter().rect_stroke(
            open_r,
            9.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(190, 225, 255, 160)),
            egui::StrokeKind::Inside,
        );
        let og = tui.painter().layout_no_wrap("Open".into(), Theme::font_semibold(10.5), Color32::WHITE);
        tui.painter().galley(open_r.center() - og.size() * 0.5, og, Color32::WHITE);
        if oresp.clicked() {
            actions.push(PanelAction::ActivateNotification {
                id: item.id,
                app_id: item.app_id.clone(),
                app_name: item.app_name.clone(),
                title: item.title.clone(),
            });
        }

        let dis_r = Rect::from_min_size(pos2(btn_row.max.x - 76.0, btn_row.min.y + 2.0), vec2(76.0, 24.0));
        let dresp = tui.interact(dis_r, tui.id().with(("notif-dismiss", item.id)), Sense::click());
        let dbg = if dresp.hovered() {
            theme::mix(theme.pill_btn_bg, theme.pill_text_dim, 0.2)
        } else {
            theme.pill_btn_bg
        };
        tui.painter().rect_filled(dis_r, 9.0, dbg);
        tui.painter().rect_stroke(
            dis_r,
            9.0,
            Stroke::new(1.0, theme.pill_btn_border),
            egui::StrokeKind::Inside,
        );
        let dg = tui.painter().layout_no_wrap("Dismiss".into(), Theme::font_semibold(10.5), theme.pill_text);
        tui.painter().galley(dis_r.center() - dg.size() * 0.5, dg, theme.pill_text);
        if dresp.clicked() {
            actions.push(PanelAction::DismissNotification(item.id));
        }
    });
}

fn paint_brand_badge(
    painter: &egui::Painter,
    rect: Rect,
    item: &NotificationItem,
    theme: &Theme,
) {
    let lower = format!("{} {}", item.app_name, item.app_id).to_ascii_lowercase();
    let (glyph, bg, fg) = if lower.contains("snipping") || lower.contains("screensketch") {
        (icons::SCISSORS, Color32::from_rgb(236, 240, 246), Color32::from_rgb(38, 122, 224))
    } else if lower.contains("whatsapp") || lower.contains("telegram") {
        (icons_fill::CHAT_CIRCLE, theme::with_alpha(theme.accent_green, 0.22), theme.accent_green)
    } else if lower.contains("chrome") || lower.contains("brave") || lower.contains("edge") || lower.contains("firefox") {
        (icons_fill::GLOBE, theme::with_alpha(theme.accent_blue, 0.22), theme.accent_blue)
    } else if lower.contains("mail") || lower.contains("outlook") {
        (icons_fill::ENVELOPE, theme::with_alpha(theme.accent_cyan, 0.22), theme.accent_cyan)
    } else if lower.contains("terminal") || lower.contains("powershell") || lower.contains("code") {
        (icons_fill::TERMINAL, theme::with_alpha(theme.accent_purple, 0.22), theme.accent_purple)
    } else {
        (icons_fill::BELL_SIMPLE, theme::with_alpha(theme.accent_blue, 0.22), theme.accent_blue)
    };

    painter.rect_filled(rect, 7.0, bg);
    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(1.0, theme.card_border),
        egui::StrokeKind::Inside,
    );
    let g = painter.layout_no_wrap(glyph.into(), icon_font(15.0, glyph != icons::SCISSORS), fg);
    painter.galley(rect.center() - g.size() * 0.5, g, fg);
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
}
