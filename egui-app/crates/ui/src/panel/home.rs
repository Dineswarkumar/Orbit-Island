//! Home section: idle clock + focus timer (or active media controls + 20-bar spectrum)
//! and top Shelf / Notifications / Settings action bar.

use super::{HomeSubView, PanelAction, PanelState, with_tile_transform};
use crate::paint;
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, Context, Rect, Sense, Stroke, TextureId, Ui, pos2, vec2};
use orbit_core::store::AppSnapshot;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &Context,
    ui: &mut Ui,
    panel_rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    state: &mut PanelState,
    media_art: Option<TextureId>,
    reveal_t: f32,
    now_unix_ms: u64,
    actions: &mut Vec<PanelAction>,
) {
    let inner = Rect::from_min_max(
        pos2(panel_rect.min.x + 14.0, panel_rect.min.y + 11.0),
        pos2(panel_rect.max.x - 18.0, panel_rect.max.y - 11.0),
    );

    // Top row: [Shelf] ... [Bell N] [Gear]
    let top_y = inner.min.y;
    let shelf_w = if snap.shelf.is_empty() { 70.0 } else { 84.0 };
    let shelf_rect = Rect::from_min_size(pos2(inner.min.x, top_y), vec2(shelf_w, 24.0));
    with_tile_transform(ctx, "home-shelf-btn", shelf_rect, 0, reveal_t, |tui| {
        if top_pill_button(
            tui,
            shelf_rect,
            theme,
            icons::FOLDER,
            theme.accent_blue,
            if snap.shelf.is_empty() {
                "Shelf".to_string()
            } else {
                format!("Shelf {}", snap.shelf.len())
            },
        ) {
            actions.push(PanelAction::OpenShelfStub);
        }
    });

    let gear_rect = Rect::from_min_size(pos2(inner.max.x - 28.0, top_y), vec2(28.0, 24.0));
    let notif_rect = Rect::from_min_size(pos2(gear_rect.min.x - 52.0, top_y), vec2(46.0, 24.0));

    with_tile_transform(ctx, "home-notif-btn", notif_rect, 1, reveal_t, |tui| {
        if top_pill_button(
            tui,
            notif_rect,
            theme,
            icons::BELL,
            theme.accent_blue,
            snap.notifications.total_count.to_string(),
        ) && snap.notifications.total_count > 0
        {
            actions.push(PanelAction::ShowLatestNotification);
        }
    });

    with_tile_transform(ctx, "home-gear-btn", gear_rect, 2, reveal_t, |tui| {
        if top_icon_button(tui, gear_rect, theme, icons::GEAR, theme.accent_blue) {
            actions.push(PanelAction::OpenSettings);
        }
    });

    // Activity priority (PRD Section 4): active media takes Home unless user clicked Clock toggle.
    let media_active = snap.media.as_ref().is_some_and(|m| {
        m.playback_status.eq_ignore_ascii_case("playing")
            || (m.playback_status.eq_ignore_ascii_case("paused") && !m.track.title.trim().is_empty())
    });

    let body_rect = Rect::from_min_max(
        pos2(inner.min.x, top_y + 28.0),
        pos2(inner.max.x, inner.max.y),
    );

    if media_active && state.home_sub_view != HomeSubView::Clock {
        if let Some(media) = snap.media.as_ref() {
            show_media_view(
                ctx,
                ui,
                body_rect,
                snap,
                media,
                media_art,
                theme,
                state,
                reveal_t,
                actions,
            );
        }
    } else {
        show_clock_focus_view(
            ctx,
            ui,
            body_rect,
            snap,
            theme,
            state,
            media_active,
            reveal_t,
            now_unix_ms,
            actions,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn show_clock_focus_view(
    ctx: &Context,
    _ui: &mut Ui,
    body_rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    state: &mut PanelState,
    media_active: bool,
    reveal_t: f32,
    now_unix_ms: u64,
    actions: &mut Vec<PanelAction>,
) {
    let clock_card = Rect::from_min_max(
        pos2(body_rect.min.x, body_rect.min.y),
        pos2(body_rect.max.x, body_rect.max.y - 34.0),
    );
    with_tile_transform(ctx, "home-clock-hero", clock_card, 3, reveal_t, |tui| {
        let p = tui.painter();
        let digits = if snap.clock.time_digits.is_empty() {
            snap.clock.time.as_str()
        } else {
            snap.clock.time_digits.as_str()
        };
        let time_galley =
            p.layout_no_wrap(digits.to_owned(), Theme::font_semibold(34.0), theme.pill_text);
        let ampm_galley = if !snap.clock.ampm.is_empty() {
            Some(p.layout_no_wrap(
                snap.clock.ampm.clone(),
                Theme::font_semibold(12.0),
                theme.pill_text_dim,
            ))
        } else {
            None
        };
        let total_w = time_galley.size().x
            + ampm_galley
                .as_ref()
                .map_or(0.0, |g| 6.0 + g.size().x);
        let cx = clock_card.center().x;
        let top_y = clock_card.min.y + 4.0;
        let x0 = cx - total_w * 0.5;
        let time_h = time_galley.size().y;
        p.galley(pos2(x0, top_y), time_galley, theme.pill_text);
        if let Some(ampm) = ampm_galley {
            let ampm_x = x0 + total_w - ampm.size().x;
            let ampm_y = top_y + time_h - ampm.size().y - 7.0;
            p.galley(pos2(ampm_x, ampm_y), ampm, theme.pill_text_dim);
        }

        let date_str = if snap.clock.full_date.is_empty() {
            snap.clock.date.as_str()
        } else {
            snap.clock.full_date.as_str()
        };
        let date_galley = p.layout_no_wrap(
            date_str.to_owned(),
            Theme::font_semibold(11.5),
            theme.pill_text,
        );
        p.galley(
            pos2(cx - date_galley.size().x * 0.5, top_y + time_h - 1.0),
            date_galley,
            theme.pill_text,
        );

        // Privacy badge or Media switch icon button when active.
        if media_active {
            let chip = Rect::from_min_size(pos2(clock_card.max.x - 26.0, clock_card.min.y), vec2(26.0, 20.0));
            if small_icon_chip_button(tui, chip, theme, "to-media", icons_fill::MUSIC_NOTES, true, theme.accent_blue) {
                state.home_sub_view = HomeSubView::Auto;
            }
        } else if snap.privacy.camera_active || snap.privacy.microphone_active {
            let text = privacy_label(&snap.privacy);
            let pg = p.layout_no_wrap(text, Theme::font(9.5), theme.accent_amber);
            p.galley(
                pos2(cx - pg.size().x * 0.5, clock_card.max.y - pg.size().y),
                pg,
                theme.accent_amber,
            );
        }
    });

    // Bottom row: Focus bar (`FOCUS: [5m] [15m] [25m] [45m]`) OR active timer controls.
    let focus_rect = Rect::from_min_size(
        pos2(body_rect.center().x - 122.0, body_rect.max.y - 26.0),
        vec2(244.0, 26.0),
    );
    with_tile_transform(ctx, "home-focus-bar", focus_rect, 4, reveal_t, |tui| {
        if snap.focus_timer.is_active() {
            show_active_timer_bar(tui, focus_rect, snap, theme, now_unix_ms, actions);
        } else {
            show_focus_presets_bar(tui, focus_rect, theme, actions);
        }
    });
}

fn show_focus_presets_bar(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    actions: &mut Vec<PanelAction>,
) {
    let p = ui.painter();
    let label_rect = Rect::from_min_size(rect.min, vec2(58.0, rect.height()));
    p.rect_filled(label_rect, 13.0, theme.pill_btn_bg);
    p.rect_stroke(
        label_rect,
        13.0,
        Stroke::new(1.0, theme.pill_btn_border),
        egui::StrokeKind::Inside,
    );
    let lg = p.layout_no_wrap("FOCUS:".into(), Theme::font_semibold(10.5), theme.pill_text);
    p.galley(label_rect.center() - lg.size() * 0.5, lg, theme.pill_text);

    let presets = [(5u64, "5m"), (15, "15m"), (25, "25m"), (45, "45m")];
    let mut x = label_rect.max.x + 6.0;
    for (mins, lbl) in presets {
        let btn_rect = Rect::from_min_size(pos2(x, rect.min.y), vec2(40.0, rect.height()));
        let resp = ui.interact(btn_rect, ui.id().with(("focus-preset", mins)), Sense::click());
        let bg = if resp.hovered() {
            theme::mix(theme.pill_btn_bg, theme.accent_blue, 0.25)
        } else {
            theme.pill_btn_bg
        };
        let border = if resp.hovered() {
            theme.accent_blue
        } else {
            theme.pill_btn_border
        };
        let p = ui.painter();
        p.rect_filled(btn_rect, 13.0, bg);
        p.rect_stroke(
            btn_rect,
            13.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let g = p.layout_no_wrap(lbl.into(), Theme::font_semibold(10.5), theme.pill_text);
        p.galley(btn_rect.center() - g.size() * 0.5, g, theme.pill_text);
        if resp.clicked() {
            actions.push(PanelAction::StartFocusTimer(mins * 60));
        }
        x += 45.0;
    }
}

fn show_active_timer_bar(
    ui: &mut Ui,
    rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    now_unix_ms: u64,
    actions: &mut Vec<PanelAction>,
) {
    let rem = snap.focus_timer.remaining_secs(now_unix_ms);
    let prog = snap.focus_timer.progress(now_unix_ms);
    let p = ui.painter();
    p.rect_filled(rect, 13.0, theme.pill_btn_bg);
    p.rect_stroke(
        rect,
        13.0,
        Stroke::new(1.0, theme.pill_btn_border),
        egui::StrokeKind::Inside,
    );
    let fill_w = (rect.width() * prog).clamp(0.0, rect.width());
    if fill_w > 4.0 {
        let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, rect.height()));
        p.rect_filled(fill_rect, 13.0, theme::with_alpha(theme.accent_blue, 0.22));
    }

    let ig = p.layout_no_wrap(
        icons_fill::TIMER.into(),
        icon_font(12.5, true),
        theme.accent_blue,
    );
    p.galley(pos2(rect.min.x + 11.0, rect.center().y - ig.size().y * 0.5), ig, theme.accent_blue);

    let time_str = format!("{}  {:02}:{:02}", snap.focus_timer.label, rem / 60, rem % 60);
    let tg = p.layout_no_wrap(time_str, Theme::font_semibold(11.0), theme.pill_text);
    p.galley(pos2(rect.min.x + 28.0, rect.center().y - tg.size().y * 0.5), tg, theme.pill_text);

    let reset_r = Rect::from_min_size(pos2(rect.max.x - 30.0, rect.min.y + 3.0), vec2(24.0, 20.0));
    let pause_r = Rect::from_min_size(pos2(reset_r.min.x - 28.0, rect.min.y + 3.0), vec2(24.0, 20.0));
    let pause_icon = if snap.focus_timer.is_running() {
        icons_fill::PAUSE
    } else {
        icons_fill::PLAY
    };
    if small_icon_chip_button(ui, pause_r, theme, "timer-pause", pause_icon, true, theme.pill_text) {
        actions.push(PanelAction::TogglePauseFocusTimer);
    }
    if small_icon_chip_button(
        ui,
        reset_r,
        theme,
        "timer-reset",
        icons::ARROW_COUNTER_CLOCKWISE,
        false,
        theme.pill_text_dim,
    ) {
        actions.push(PanelAction::ResetFocusTimer);
    }
}

#[allow(clippy::too_many_arguments)]
fn show_media_view(
    ctx: &Context,
    _ui: &mut Ui,
    body_rect: Rect,
    snap: &AppSnapshot,
    media: &orbit_core::events::MediaSessionSnapshot,
    media_art: Option<TextureId>,
    theme: &Theme,
    state: &mut PanelState,
    reveal_t: f32,
    actions: &mut Vec<PanelAction>,
) {
    let art_rect = Rect::from_min_size(pos2(body_rect.min.x, body_rect.min.y + 4.0), vec2(52.0, 52.0));
    with_tile_transform(ctx, "home-media-art", art_rect, 3, reveal_t, |tui| {
        let p = tui.painter();
        if let Some(tex) = media_art {
            paint::rounded_texture(p, art_rect, 11.0, tex, 1.0, theme.card_border);
        } else {
            p.rect_filled(art_rect, 11.0, theme::with_alpha(theme.accent_blue, 0.16));
            p.rect_stroke(
                art_rect,
                11.0,
                Stroke::new(1.0, theme::with_alpha(theme.accent_blue, 0.45)),
                egui::StrokeKind::Inside,
            );
            let note = p.layout_no_wrap(
                icons_fill::MUSIC_NOTES.into(),
                icon_font(22.0, true),
                theme.accent_blue,
            );
            p.galley(art_rect.center() - note.size() * 0.5, note, theme.accent_blue);
        }
    });

    let info_rect = Rect::from_min_max(
        pos2(art_rect.max.x + 10.0, body_rect.min.y + 2.0),
        pos2(body_rect.max.x, art_rect.max.y + 4.0),
    );
    with_tile_transform(ctx, "home-media-meta", info_rect, 4, reveal_t, |tui| {
        let clock_chip = Rect::from_min_size(
            pos2(info_rect.max.x - 26.0, info_rect.min.y - 1.0),
            vec2(26.0, 20.0),
        );
        if small_icon_chip_button(tui, clock_chip, theme, "to-clock", icons::CLOCK, false, theme.accent_blue) {
            state.home_sub_view = HomeSubView::Clock;
        }

        let p = tui.painter();
        let src_g = p.layout_no_wrap(
            media.source_title.clone(),
            Theme::font_semibold(9.5),
            theme.accent_blue,
        );
        p.galley(pos2(info_rect.min.x, info_rect.min.y), src_g, theme.accent_blue);

        let title = if media.track.title.trim().is_empty() {
            "Unknown Track"
        } else {
            media.track.title.as_str()
        };
        let mut title_job = egui::text::LayoutJob::single_section(
            title.to_owned(),
            egui::TextFormat {
                font_id: Theme::font_semibold(13.0),
                color: theme.pill_text,
                ..Default::default()
            },
        );
        title_job.wrap = egui::text::TextWrapping {
            max_width: (info_rect.width() - 32.0).max(40.0),
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let tg = p.layout_job(title_job);
        p.galley(pos2(info_rect.min.x, info_rect.min.y + 15.0), tg, theme.pill_text);

        let artist = if media.track.artist.trim().is_empty() {
            &media.source_title
        } else {
            &media.track.artist
        };
        let mut artist_job = egui::text::LayoutJob::single_section(
            artist.to_owned(),
            egui::TextFormat {
                font_id: Theme::font_semibold(10.5),
                color: theme.pill_text_dim,
                ..Default::default()
            },
        );
        artist_job.wrap = egui::text::TextWrapping {
            max_width: (info_rect.width() - 4.0).max(40.0),
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        };
        let ag = p.layout_job(artist_job);
        p.galley(pos2(info_rect.min.x, info_rect.min.y + 34.0), ag, theme.pill_text_dim);
    });

    // Seek bar + Transport controls + 20-bar spectrum
    let ctrl_rect = Rect::from_min_max(
        pos2(body_rect.min.x, art_rect.max.y + 8.0),
        pos2(body_rect.max.x, body_rect.max.y),
    );
    with_tile_transform(ctx, "home-media-controls", ctrl_rect, 5, reveal_t, |tui| {
        if let Some(tl) = &media.timeline
            && tl.duration_secs > 0.5
        {
            let bar_r = Rect::from_min_size(
                pos2(ctrl_rect.min.x + 36.0, ctrl_rect.min.y + 5.0),
                vec2((ctrl_rect.width() - 72.0).max(40.0), 4.5),
            );
            let frac = (tl.position_secs / tl.duration_secs).clamp(0.0, 1.0) as f32;
            let p = tui.painter();
            let el = format_mm_ss(tl.position_secs as u64);
            let tot = format_mm_ss(tl.duration_secs as u64);
            let eg = p.layout_no_wrap(el, Theme::font_semibold(9.5), theme.pill_text_dim);
            p.galley(pos2(ctrl_rect.min.x, bar_r.center().y - eg.size().y * 0.5), eg, theme.pill_text_dim);
            let dg = p.layout_no_wrap(tot, Theme::font_semibold(9.5), theme.pill_text_dim);
            p.galley(pos2(ctrl_rect.max.x - dg.size().x, bar_r.center().y - dg.size().y * 0.5), dg, theme.pill_text_dim);

            p.rect_filled(bar_r, 2.5, theme.track_bg);
            let fill_w = (bar_r.width() * frac).max(0.0);
            p.rect_filled(
                Rect::from_min_size(bar_r.min, vec2(fill_w, bar_r.height())),
                2.5,
                theme.accent_blue,
            );
            let knob_x = (bar_r.min.x + fill_w).clamp(bar_r.min.x, bar_r.max.x);
            p.circle_filled(
                pos2(knob_x, bar_r.center().y),
                6.0,
                theme::with_alpha(theme.accent_blue, 0.28),
            );
            p.circle_filled(pos2(knob_x, bar_r.center().y), 4.0, theme.pill_text);
            if media.controls.can_seek {
                let resp = tui.interact(bar_r.expand2(vec2(0.0, 6.0)), tui.id().with("seek"), Sense::click_and_drag());
                if (resp.clicked() || resp.dragged())
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    let ratio = ((pos.x - bar_r.min.x) / bar_r.width()).clamp(0.0, 1.0) as f64;
                    actions.push(PanelAction::MediaSeek(ratio * tl.duration_secs));
                }
            }
        }

        // Transport buttons centered cleanly at bottom + flanking 20-bar symmetric equalizer
        let row_y = ctrl_rect.max.y - 28.0;
        let cx = ctrl_rect.center().x;
        let prev_r = Rect::from_center_size(pos2(cx - 42.0, row_y + 13.0), vec2(26.0, 26.0));
        let play_r = Rect::from_center_size(pos2(cx, row_y + 13.0), vec2(32.0, 32.0));
        let next_r = Rect::from_center_size(pos2(cx + 42.0, row_y + 13.0), vec2(26.0, 26.0));

        if media_btn(tui, prev_r, theme, icons_fill::SKIP_BACK, media.controls.can_skip_previous, false) {
            actions.push(PanelAction::MediaCommand("previous"));
        }
        let is_playing = media.playback_status.eq_ignore_ascii_case("playing");
        let play_icon = if is_playing { icons_fill::PAUSE } else { icons_fill::PLAY };
        if media_btn(
            tui,
            play_r,
            theme,
            play_icon,
            media.controls.can_play || media.controls.can_pause,
            true,
        ) {
            actions.push(PanelAction::MediaCommand("toggle"));
        }
        if media_btn(tui, next_r, theme, icons_fill::SKIP_FORWARD, media.controls.can_skip_next, false) {
            actions.push(PanelAction::MediaCommand("next"));
        }

        if is_playing {
            let p = tui.painter();
            let bar_w = 2.4;
            let gap = 2.0;
            let start_l = ctrl_rect.min.x;
            for (i, amp) in snap.spectrum.wave_bars.iter().take(10).enumerate() {
                let h = (2.5 + amp.clamp(0.0, 1.0) * 14.5).min(17.0);
                let bx = start_l + i as f32 * (bar_w + gap);
                if bx + bar_w < prev_r.min.x - 6.0 {
                    let r = Rect::from_min_size(pos2(bx, ctrl_rect.max.y - h - 2.0), vec2(bar_w, h));
                    p.rect_filled(r, 1.2, theme::with_alpha(theme.accent_blue, 0.80));
                }
            }
            let start_r = ctrl_rect.max.x - bar_w;
            for (i, amp) in snap.spectrum.wave_bars.iter().skip(10).take(10).enumerate() {
                let h = (2.5 + amp.clamp(0.0, 1.0) * 14.5).min(17.0);
                let bx = start_r - i as f32 * (bar_w + gap);
                if bx > next_r.max.x + 6.0 {
                    let r = Rect::from_min_size(pos2(bx, ctrl_rect.max.y - h - 2.0), vec2(bar_w, h));
                    p.rect_filled(r, 1.2, theme::with_alpha(theme.accent_blue, 0.80));
                }
            }
        }
    });
}

fn top_pill_button(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    icon: &str,
    icon_color: Color32,
    label: String,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(("top-pill", &label)), Sense::click());
    let bg = if resp.hovered() {
        theme::mix(theme.card_bg, theme.accent_blue, 0.16)
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
    let ig = p.layout_no_wrap(icon.into(), icon_font(12.5, false), icon_color);
    let lg = p.layout_no_wrap(label, Theme::font_semibold(10.5), theme.pill_text);
    let ig_w = ig.size().x;
    let ig_h = ig.size().y;
    let lg_w = lg.size().x;
    let lg_h = lg.size().y;
    let gap = 5.0;
    let total_w = ig_w + gap + lg_w;
    let x0 = rect.center().x - total_w * 0.5;
    p.galley(pos2(x0, rect.center().y - ig_h * 0.5), ig, icon_color);
    p.galley(
        pos2(x0 + ig_w + gap, rect.center().y - lg_h * 0.5),
        lg,
        theme.pill_text,
    );
    resp.clicked()
}

fn top_icon_button(ui: &mut Ui, rect: Rect, theme: &Theme, icon: &str, color: Color32) -> bool {
    let resp = ui.interact(rect, ui.id().with(("top-icon", icon)), Sense::click());
    let bg = if resp.hovered() {
        theme::mix(theme.card_bg, theme.accent_blue, 0.18)
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
    let g = p.layout_no_wrap(icon.into(), icon_font(13.0, false), color);
    p.galley(rect.center() - g.size() * 0.5, g, color);
    resp.clicked()
}

fn small_icon_chip_button(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    id_key: &str,
    icon: &str,
    filled: bool,
    color: Color32,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(("icon-chip", id_key)), Sense::click());
    let bg = if resp.hovered() {
        theme::mix(theme.pill_btn_bg, theme.accent_blue, 0.28)
    } else {
        theme.pill_btn_bg
    };
    let border = if resp.hovered() {
        theme.accent_blue
    } else {
        theme.pill_btn_border
    };
    let p = ui.painter();
    p.rect_filled(rect, 9.0, bg);
    p.rect_stroke(
        rect,
        9.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let g = p.layout_no_wrap(icon.into(), icon_font(12.0, filled), color);
    p.galley(rect.center() - g.size() * 0.5, g, color);
    resp.clicked()
}

fn media_btn(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    icon: &str,
    enabled: bool,
    primary: bool,
) -> bool {
    let resp = ui.interact(
        rect,
        ui.id().with(("media-btn", icon)),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    let p = ui.painter();
    let radius = rect.height() * 0.5;
    if primary {
        p.circle_filled(
            rect.center(),
            radius + 2.5,
            theme::with_alpha(theme.accent_blue, if enabled { 0.22 } else { 0.08 }),
        );
        let fill = if resp.hovered() && enabled {
            theme.accent_blue
        } else {
            theme::with_alpha(theme.accent_blue, if enabled { 0.90 } else { 0.35 })
        };
        p.rect_filled(rect, radius, fill);
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::with_alpha(theme.pill_text, 0.28)),
            egui::StrokeKind::Inside,
        );
    } else {
        let fill = if resp.hovered() && enabled {
            theme.card_border
        } else {
            theme::with_alpha(theme.card_bg, 0.65)
        };
        p.rect_filled(rect, radius, fill);
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme.card_border),
            egui::StrokeKind::Inside,
        );
    }
    let col = if enabled {
        theme.pill_text
    } else {
        theme::with_alpha(theme.pill_text_dim, 0.4)
    };
    let g = p.layout_no_wrap(icon.into(), icon_font(if primary { 14.0 } else { 13.0 }, true), col);
    p.galley(rect.center() - g.size() * 0.5, g, col);
    enabled && resp.clicked()
}

fn privacy_label(priv_state: &orbit_core::events::PrivacyState) -> String {
    match (priv_state.camera_active, priv_state.microphone_active) {
        (true, true) => format!(
            "Camera & Mic active{}",
            priv_state
                .microphone_app
                .as_deref()
                .or(priv_state.camera_app.as_deref())
                .map(|a| format!(" · {a}"))
                .unwrap_or_default()
        ),
        (true, false) => format!(
            "Camera active{}",
            priv_state
                .camera_app
                .as_deref()
                .map(|a| format!(" · {a}"))
                .unwrap_or_default()
        ),
        (false, true) => format!(
            "Mic active{}",
            priv_state
                .microphone_app
                .as_deref()
                .map(|a| format!(" · {a}"))
                .unwrap_or_default()
        ),
        (false, false) => String::new(),
    }
}

fn format_mm_ss(total_secs: u64) -> String {
    format!("{}:{:02}", total_secs / 60, total_secs % 60)
}
