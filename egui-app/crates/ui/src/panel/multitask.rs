//! Dynamic Multi-Task Hub section (shown when >= 2 live activities are active).

use super::{PanelAction, SectionKind, with_tile_transform};
use crate::theme::{self, Theme};
use egui::{Context, Rect, Sense, Stroke, Ui, pos2, vec2};
use orbit_core::store::AppSnapshot;

pub fn show(
    ctx: &Context,
    _ui: &mut Ui,
    panel_rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    reveal_t: f32,
    now_unix_ms: u64,
    active_section: &mut usize,
    actions: &mut Vec<PanelAction>,
) {
    let inner = Rect::from_min_max(
        pos2(panel_rect.min.x + 14.0, panel_rect.min.y + 12.0),
        pos2(panel_rect.max.x - 18.0, panel_rect.max.y - 12.0),
    );

    let header_r = Rect::from_min_size(inner.min, vec2(inner.width(), 20.0));
    with_tile_transform(ctx, "mt-header", header_r, 0, reveal_t, |tui| {
        let p = tui.painter();
        let g = p.layout_no_wrap(
            "LIVE MULTI-TASK HUB".into(),
            Theme::font_semibold(10.0),
            theme.pill_text_dim,
        );
        p.galley(pos2(header_r.min.x, header_r.center().y - g.size().y * 0.5), g, theme.pill_text_dim);
    });

    let grid_top = header_r.max.y + 6.0;
    let col_w = (inner.width() - 8.0) * 0.5;
    let row_h = ((inner.max.y - grid_top) - 8.0) * 0.5;

    let cards = [
        Rect::from_min_size(pos2(inner.min.x, grid_top), vec2(col_w, row_h)),
        Rect::from_min_size(pos2(inner.min.x + col_w + 8.0, grid_top), vec2(col_w, row_h)),
        Rect::from_min_size(pos2(inner.min.x, grid_top + row_h + 8.0), vec2(col_w, row_h)),
        Rect::from_min_size(
            pos2(inner.min.x + col_w + 8.0, grid_top + row_h + 8.0),
            vec2(col_w, row_h),
        ),
    ];

    // Tile 1: Media
    with_tile_transform(ctx, "mt-media", cards[0], 1, reveal_t, |tui| {
        let subtitle = snap
            .media
            .as_ref()
            .filter(|m| !m.track.title.trim().is_empty())
            .map(|m| m.track.title.as_str())
            .unwrap_or("No active media");
        if hub_card(tui, cards[0], theme, "MEDIA", subtitle, theme.accent_blue)
            && snap.media.is_some()
        {
            actions.push(PanelAction::MediaCommand("toggle"));
        }
    });

    // Tile 2: Focus Timer
    with_tile_transform(ctx, "mt-timer", cards[1], 2, reveal_t, |tui| {
        let subtitle = if snap.focus_timer.is_active() {
            let rem = snap.focus_timer.remaining_secs(now_unix_ms);
            format!("{:02}:{:02} · {}", rem / 60, rem % 60, snap.focus_timer.label)
        } else {
            "Idle · Click to start 25m".to_string()
        };
        if hub_card(tui, cards[1], theme, "FOCUS TIMER", &subtitle, theme.accent_cyan) {
            if snap.focus_timer.is_active() {
                actions.push(PanelAction::TogglePauseFocusTimer);
            } else {
                actions.push(PanelAction::StartFocusTimer(25 * 60));
            }
        }
    });

    // Tile 3: Shelf & Privacy
    with_tile_transform(ctx, "mt-shelf", cards[2], 3, reveal_t, |tui| {
        let subtitle = format!("{} held items", snap.shelf.len());
        if hub_card(tui, cards[2], theme, "FILE SHELF", &subtitle, theme.accent_purple) {
            actions.push(PanelAction::OpenShelfStub);
        }
    });

    // Tile 4: Next Task / Reminder
    with_tile_transform(ctx, "mt-agenda", cards[3], 4, reveal_t, |tui| {
        let next = snap
            .reminders
            .iter()
            .find(|r| !r.is_completed)
            .map(|r| r.title.as_str())
            .unwrap_or("No pending tasks");
        if hub_card(tui, cards[3], theme, "UPCOMING", next, theme.accent_green) {
            let _ = SectionKind::Calendar;
            *active_section = (*active_section + 1).min(3);
        }
    });
}

fn hub_card(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    title: &str,
    subtitle: &str,
    accent: egui::Color32,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(("mt", title)), Sense::click());
    let bg = if resp.hovered() {
        theme::mix(theme.card_bg, accent, 0.14)
    } else {
        theme.card_bg
    };
    let p = ui.painter();
    p.rect_filled(rect, 12.0, bg);
    p.rect_stroke(
        rect,
        12.0,
        Stroke::new(1.0, theme.card_border),
        egui::StrokeKind::Inside,
    );

    let tg = p.layout_no_wrap(title.into(), Theme::font_semibold(9.5), accent);
    p.galley(pos2(rect.min.x + 10.0, rect.min.y + 8.0), tg, accent);

    let mut sub_job = egui::text::LayoutJob::single_section(
        subtitle.to_owned(),
        egui::TextFormat {
            font_id: Theme::font_semibold(11.0),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    sub_job.wrap = egui::text::TextWrapping {
        max_width: (rect.width() - 20.0).max(20.0),
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let sg = p.layout_job(sub_job);
    p.galley(pos2(rect.min.x + 10.0, rect.max.y - sg.size().y - 9.0), sg, theme.pill_text);
    resp.clicked()
}
