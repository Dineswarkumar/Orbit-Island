//! Hardware Monitor section matching the user's screenshot:
//! SYSTEM header bar + 2x3 telemetry bento grid with Boost RAM and glowing Battery card.

use super::{PanelAction, PanelState, with_tile_transform};
use crate::paint;
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, Context, Rect, Sense, Ui, pos2, vec2};
use orbit_core::store::AppSnapshot;

pub fn show(
    ctx: &Context,
    _ui: &mut Ui,
    panel_rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    state: &mut PanelState,
    reveal_t: f32,
    actions: &mut Vec<PanelAction>,
) {
    let hw = &snap.hardware;
    let inner = Rect::from_min_max(
        pos2(panel_rect.min.x + 12.0, panel_rect.min.y + 10.0),
        pos2(panel_rect.max.x - 18.0, panel_rect.max.y - 10.0),
    );
    let muted_label = theme::mix(theme.pill_text_dim, theme.pill_text, 0.55);
    let bright_val = if theme.is_light {
        theme.pill_text
    } else {
        Color32::WHITE
    };

    // Top bar: SYSTEM (left) | Boost summary or GPU Name (right)
    let hdr_rect = Rect::from_min_size(inner.min, vec2(inner.width(), 24.0));
    with_tile_transform(ctx, "hw-header", hdr_rect, 0, reveal_t, |tui| {
        let hovered = tui.rect_contains_pointer(hdr_rect);
        let p = tui.painter();
        let hdr_tint = state.last_ram_boost.map(|_| theme.accent_green);
        paint::card_3d(
            p,
            hdr_rect,
            11.0,
            theme.card_top,
            theme.card_bottom,
            theme.card_border,
            theme.card_highlight,
            hdr_tint,
            hovered,
        );
        let lg = p.layout_no_wrap("SYSTEM".into(), Theme::font_semibold(10.0), muted_label);
        p.galley(
            pos2(hdr_rect.min.x + 10.0, hdr_rect.center().y - lg.size().y * 0.5),
            lg,
            muted_label,
        );

        if let Some((procs, freed_mb, _)) = state.last_ram_boost {
            let emerald = if theme.is_light {
                Color32::from_rgb(5, 150, 105)
            } else {
                theme.accent_green
            };
            let summary = if freed_mb >= 0.5 {
                format!("Trimmed {procs} apps · Freed {freed_mb:.1} MB")
            } else {
                format!("Trimmed {procs} apps · Clean")
            };
            let mut job = egui::text::LayoutJob::single_section(
                summary,
                egui::TextFormat {
                    font_id: Theme::font_semibold(10.5),
                    color: emerald,
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: (hdr_rect.width() - 90.0).max(60.0),
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let rg = p.layout_job(job);
            let text_x = hdr_rect.max.x - 10.0 - rg.size().x;
            let cy = hdr_rect.center().y;
            let dot_pos = pos2(text_x - 8.0, cy);
            p.circle_filled(dot_pos, 5.0, theme::with_alpha(emerald, 0.28));
            p.circle_filled(dot_pos, 2.6, emerald);
            p.galley(pos2(text_x, cy - rg.size().y * 0.5), rg, emerald);
        } else {
            let gpu_label = if hw.gpu_name.trim().is_empty() {
                "Graphics".to_string()
            } else {
                hw.gpu_name.clone()
            };
            let mut job = egui::text::LayoutJob::single_section(
                gpu_label,
                egui::TextFormat {
                    font_id: Theme::font_semibold(10.5),
                    color: bright_val,
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: (hdr_rect.width() - 80.0).max(60.0),
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let rg = p.layout_job(job);
            p.galley(
                pos2(hdr_rect.max.x - 10.0 - rg.size().x, hdr_rect.center().y - rg.size().y * 0.5),
                rg,
                bright_val,
            );
        }
    });

    // 2x3 Grid
    let top_y = hdr_rect.max.y + 6.0;
    let gap = 6.0;
    let col_w = (inner.width() - gap) * 0.5;
    let row_h = ((inner.max.y - top_y) - gap * 2.0) / 3.0;
    let cell = |row: usize, col: usize| {
        Rect::from_min_size(
            pos2(inner.min.x + col as f32 * (col_w + gap), top_y + row as f32 * (row_h + gap)),
            vec2(col_w, row_h),
        )
    };

    // Row 0, Col 0: CPU
    let cpu_r = cell(0, 0);
    with_tile_transform(ctx, "hw-cpu", cpu_r, 1, reveal_t, |tui| {
        metric_bar_card(
            tui,
            cpu_r,
            theme,
            icons::CPU,
            theme.accent_blue,
            "CPU",
            &format!("{:.1}%", hw.cpu_usage),
            hw.cpu_usage / 100.0,
        );
    });

    // Row 0, Col 1: GPU
    let gpu_r = cell(0, 1);
    with_tile_transform(ctx, "hw-gpu", gpu_r, 2, reveal_t, |tui| {
        let gpu_pct = hw.gpu_usage.unwrap_or(0.0);
        let gpu_val = hw
            .gpu_usage
            .map_or_else(|| "N/A".to_string(), |v| format!("{v:.1}%"));
        metric_bar_card(
            tui,
            gpu_r,
            theme,
            icons::MEMORY,
            theme.accent_pink,
            "GPU",
            &gpu_val,
            gpu_pct / 100.0,
        );
    });

    // Row 1, Col 0: RAM + Boost
    let ram_r = cell(1, 0);
    with_tile_transform(ctx, "hw-ram", ram_r, 3, reveal_t, |tui| {
        let used_gb = hw.ram_used_bytes as f64 / 1_073_741_824.0;
        let total_gb = hw.ram_total_bytes as f64 / 1_073_741_824.0;
        let card_hovered = tui.rect_contains_pointer(ram_r);

        let boost_r = Rect::from_min_size(pos2(ram_r.min.x + 53.0, ram_r.min.y + 5.0), vec2(42.0, 15.0));
        let tooltip = match state.ram_boost_feedback {
            Some((ref msg, true, _)) => format!("Trim app working sets across your active session ({msg})"),
            _ => "Trim app working sets across your active session".to_string(),
        };
        let bresp = tui
            .interact(boost_r, tui.id().with("hw-boost"), Sense::click())
            .on_hover_text(tooltip);

        let p = tui.painter();
        paint::card_3d(
            p,
            ram_r,
            12.0,
            theme.card_top,
            theme.card_bottom,
            theme.card_border,
            theme.card_highlight,
            Some(theme.accent_purple),
            card_hovered,
        );

        let ig = p.layout_no_wrap(icons::MEMORY.into(), icon_font(15.0, false), theme.accent_purple);
        p.galley(pos2(ram_r.min.x + 9.0, ram_r.center().y - ig.size().y * 0.5), ig, theme.accent_purple);

        let text_x = ram_r.min.x + 28.0;
        let lbl_g = p.layout_no_wrap("RAM".into(), Theme::font_semibold(10.0), muted_label);
        p.galley(pos2(text_x, ram_r.min.y + 7.0), lbl_g, muted_label);

        let (boost_accent, boost_lbl, boost_fg) = match state.ram_boost_feedback {
            Some((_, true, _)) => (
                Color32::from_rgb(248, 113, 113),
                "Retry".to_string(),
                if theme.is_light {
                    Color32::from_rgb(185, 28, 28)
                } else {
                    Color32::from_rgb(254, 202, 202)
                },
            ),
            Some((_, false, _)) => {
                let emerald = if theme.is_light {
                    Color32::from_rgb(5, 150, 105)
                } else {
                    theme.accent_green
                };
                let short = match state.last_ram_boost {
                    Some((_, freed_mb, _)) if freed_mb >= 0.5 => format!("-{freed_mb:.0}M"),
                    _ => "Clean".to_string(),
                };
                (
                    emerald,
                    short,
                    if theme.is_light {
                        Color32::from_rgb(6, 95, 70)
                    } else {
                        Color32::WHITE
                    },
                )
            }
            None => (
                theme.accent_purple,
                "Boost".to_string(),
                if theme.is_light {
                    theme.accent_purple
                } else {
                    Color32::from_rgb(233, 213, 255)
                },
            ),
        };
        let boost_top = theme::mix(theme.card_top, boost_accent, if bresp.hovered() { 0.42 } else { 0.26 });
        let boost_bot = theme::mix(theme.card_bottom, boost_accent, 0.16);
        let boost_border = theme::with_alpha(boost_accent, if bresp.hovered() { 0.85 } else { 0.55 });
        paint::card_3d(
            p,
            boost_r,
            7.5,
            boost_top,
            boost_bot,
            boost_border,
            theme.card_highlight,
            Some(boost_accent),
            bresp.hovered(),
        );
        let bg = p.layout_no_wrap(boost_lbl, Theme::font_semibold(8.5), boost_fg);
        p.galley(boost_r.center() - bg.size() * 0.5, bg, boost_fg);
        if bresp.clicked() {
            actions.push(PanelAction::BoostRam);
        }

        let val_str = format!("{used_gb:.1}/{total_gb:.0}G");
        let vg = p.layout_no_wrap(val_str, Theme::font_semibold(9.5), bright_val);
        p.galley(pos2(ram_r.max.x - 8.0 - vg.size().x, ram_r.min.y + 7.0), vg, bright_val);

        let bar_r = Rect::from_min_max(
            pos2(text_x, ram_r.max.y - 10.0),
            pos2(ram_r.max.x - 9.0, ram_r.max.y - 6.0),
        );
        paint::progress_bar_3d(p, bar_r, hw.ram_percentage / 100.0, theme.accent_purple, theme.track_bg);
    });

    // Row 1, Col 1: DISK C:
    let disk_r = cell(1, 1);
    with_tile_transform(ctx, "hw-disk", disk_r, 4, reveal_t, |tui| {
        let free_gb = hw.disk_free_bytes as f64 / 1_073_741_824.0;
        let val = format!("{:.0}% ({free_gb:.0}G)", hw.disk_percentage);
        metric_bar_card(
            tui,
            disk_r,
            theme,
            icons::HARD_DRIVE,
            theme.accent_amber,
            "DISK C:",
            &val,
            hw.disk_percentage / 100.0,
        );
    });

    // Row 2, Col 0: Network Up / Down
    let net_r = cell(2, 0);
    with_tile_transform(ctx, "hw-net", net_r, 5, reveal_t, |tui| {
        let hovered = tui.rect_contains_pointer(net_r);
        let p = tui.painter();
        paint::card_3d(
            p,
            net_r,
            12.0,
            theme.card_top,
            theme.card_bottom,
            theme.card_border,
            theme.card_highlight,
            Some(theme.accent_cyan),
            hovered,
        );

        let up = format_net_speed(hw.network_upload_speed_bps);
        let dn = format_net_speed(hw.network_download_speed_bps);
        let ug = p.layout_no_wrap(format!("↑ {up}"), Theme::font_semibold(10.5), theme.accent_green);
        let dg = p.layout_no_wrap(format!("↓ {dn}"), Theme::font_semibold(10.5), theme.accent_cyan);
        let cy = net_r.center().y;
        p.galley(pos2(net_r.min.x + 11.0, cy - ug.size().y * 0.5), ug, theme.accent_green);
        p.galley(pos2(net_r.max.x - 11.0 - dg.size().x, cy - dg.size().y * 0.5), dg, theme.accent_cyan);
    });

    // Row 2, Col 1: Glowing Battery card
    let bat_r = cell(2, 1);
    with_tile_transform(ctx, "hw-bat", bat_r, 6, reveal_t, |tui| {
        let hovered = tui.rect_contains_pointer(bat_r);
        let connected = hw.ac_power_connected == Some(true) || hw.battery_charging == Some(true);
        let pct = hw.battery_percentage;
        let accent = if connected || pct.is_some_and(|v| v >= 80) {
            theme.accent_green
        } else if pct.is_some_and(|v| v >= 30) {
            theme.accent_amber
        } else if pct.is_some() {
            Color32::from_rgb(239, 68, 68)
        } else {
            muted_label
        };
        let label = match (pct, hw.battery_charging, connected) {
            (None, _, true) => "AC Power",
            (None, _, false) => "No Battery",
            (Some(_), Some(true), _) | (Some(_), _, true) => "Connected",
            (Some(_), _, false) => "Battery",
        };
        let pct_str = pct.map_or_else(|| "--".to_string(), |v| format!("{v}%"));

        let p = tui.painter();
        paint::card_3d(
            p,
            bat_r,
            12.0,
            theme.card_top,
            theme.card_bottom,
            theme::with_alpha(accent, 0.58),
            theme.card_highlight,
            Some(accent),
            hovered,
        );
        let glyph = if connected {
            icons_fill::BATTERY_CHARGING
        } else {
            icons_fill::BATTERY_FULL
        };
        let ig = p.layout_no_wrap(glyph.into(), icon_font(16.0, true), accent);
        let lg = p.layout_no_wrap(label.into(), Theme::font_semibold(10.5), accent);
        let vg = p.layout_no_wrap(pct_str, Theme::font_semibold(11.0), accent);
        let cy = bat_r.center().y;
        p.galley(pos2(bat_r.min.x + 11.0, cy - ig.size().y * 0.5), ig, accent);
        p.galley(pos2(bat_r.center().x - lg.size().x * 0.45, cy - lg.size().y * 0.5), lg, accent);
        p.galley(pos2(bat_r.max.x - 11.0 - vg.size().x, cy - vg.size().y * 0.5), vg, accent);
    });
}

#[allow(clippy::too_many_arguments)]
fn metric_bar_card(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    icon: &str,
    accent: Color32,
    label: &str,
    value: &str,
    frac: f32,
) {
    let hovered = ui.rect_contains_pointer(rect);
    let muted_label = theme::mix(theme.pill_text_dim, theme.pill_text, 0.55);
    let bright_val = if theme.is_light {
        theme.pill_text
    } else {
        Color32::WHITE
    };
    let p = ui.painter();
    paint::card_3d(
        p,
        rect,
        12.0,
        theme.card_top,
        theme.card_bottom,
        theme.card_border,
        theme.card_highlight,
        Some(accent),
        hovered,
    );

    let ig = p.layout_no_wrap(icon.into(), icon_font(14.5, false), accent);
    p.galley(pos2(rect.min.x + 9.0, rect.center().y - ig.size().y * 0.5), ig, accent);

    let text_x = rect.min.x + 29.0;
    let lg = p.layout_no_wrap(label.into(), Theme::font_semibold(10.0), muted_label);
    p.galley(pos2(text_x, rect.min.y + 7.0), lg, muted_label);

    let vg = p.layout_no_wrap(value.into(), Theme::font_semibold(10.5), bright_val);
    p.galley(pos2(rect.max.x - 9.0 - vg.size().x, rect.min.y + 6.5), vg, bright_val);

    let bar_r = Rect::from_min_max(
        pos2(text_x, rect.max.y - 10.0),
        pos2(rect.max.x - 9.0, rect.max.y - 6.0),
    );
    paint::progress_bar_3d(p, bar_r, frac, accent, theme.track_bg);
}

fn format_net_speed(bps: u64) -> String {
    let mbps = (bps as f64 * 8.0) / 1_000_000.0;
    format!("{mbps:.1} Mbps")
}
