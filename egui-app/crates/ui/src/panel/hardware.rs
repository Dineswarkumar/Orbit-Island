//! Hardware Monitor section matching the user's screenshot:
//! SYSTEM header bar + 2x3 telemetry bento grid with Boost RAM and glowing Battery card.

use super::{PanelAction, PanelState, with_tile_transform};
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{Color32, Context, Rect, Sense, Stroke, Ui, pos2, vec2};
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

    // Top bar: SYSTEM (left) | GPU Name (right)
    let hdr_rect = Rect::from_min_size(inner.min, vec2(inner.width(), 24.0));
    with_tile_transform(ctx, "hw-header", hdr_rect, 0, reveal_t, |tui| {
        let p = tui.painter();
        p.rect_filled(hdr_rect, 11.0, theme.card_bg);
        p.rect_stroke(
            hdr_rect,
            11.0,
            Stroke::new(1.0, theme.card_border),
            egui::StrokeKind::Inside,
        );
        let lg = p.layout_no_wrap("SYSTEM".into(), Theme::font_semibold(10.0), theme.pill_text_dim);
        p.galley(
            pos2(hdr_rect.min.x + 10.0, hdr_rect.center().y - lg.size().y * 0.5),
            lg,
            theme.pill_text_dim,
        );

        let gpu_label = if hw.gpu_name.trim().is_empty() {
            "Graphics".to_string()
        } else {
            hw.gpu_name.clone()
        };
        let mut job = egui::text::LayoutJob::single_section(
            gpu_label,
            egui::TextFormat {
                font_id: Theme::font_semibold(10.5),
                color: theme.pill_text,
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
            theme.pill_text,
        );
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
        let p = tui.painter();
        p.rect_filled(ram_r, 12.0, theme.card_bg);
        p.rect_stroke(ram_r, 12.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);

        let ig = p.layout_no_wrap(icons::MEMORY.into(), icon_font(15.0, false), theme.accent_purple);
        p.galley(pos2(ram_r.min.x + 9.0, ram_r.center().y - ig.size().y * 0.5), ig, theme.accent_purple);

        let text_x = ram_r.min.x + 28.0;
        let lbl_g = p.layout_no_wrap("RAM".into(), Theme::font_semibold(10.0), theme.pill_text_dim);
        p.galley(pos2(text_x, ram_r.min.y + 7.0), lbl_g, theme.pill_text_dim);

        let boost_r = Rect::from_min_size(pos2(text_x + 25.0, ram_r.min.y + 5.0), vec2(52.0, 14.0));
        let bresp = tui
            .interact(boost_r, tui.id().with("hw-boost"), Sense::click())
            .on_hover_text("Frees Orbit's own memory only");
        let p = tui.painter();
        let boost_bg = if bresp.hovered() {
            theme::with_alpha(theme.accent_blue, 0.35)
        } else {
            theme.pill_btn_bg
        };
        p.rect_filled(boost_r, 6.0, boost_bg);
        p.rect_stroke(boost_r, 6.0, Stroke::new(1.0, theme.pill_btn_border), egui::StrokeKind::Inside);
        let bg = p.layout_no_wrap("Trim Orbit".into(), Theme::font_semibold(8.5), theme.accent_cyan);
        p.galley(boost_r.center() - bg.size() * 0.5, bg, theme.accent_cyan);
        if bresp.clicked() {
            actions.push(PanelAction::BoostRam);
        }

        let (val_str, val_col) = if let Some((ref msg, is_err, _)) = state.ram_boost_feedback {
            let col = if is_err {
                Color32::from_rgb(248, 113, 113)
            } else {
                theme.accent_green
            };
            (msg.clone(), col)
        } else {
            (format!("{used_gb:.1}/{total_gb:.0}G"), theme.pill_text)
        };
        let vg = p.layout_no_wrap(val_str, Theme::font_semibold(9.5), val_col);
        p.galley(pos2(ram_r.max.x - 8.0 - vg.size().x, ram_r.min.y + 7.0), vg, val_col);

        let bar_r = Rect::from_min_max(
            pos2(text_x, ram_r.max.y - 10.0),
            pos2(ram_r.max.x - 9.0, ram_r.max.y - 6.0),
        );
        progress_bar(p, bar_r, hw.ram_percentage / 100.0, theme.accent_purple, theme.track_bg);
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
        let p = tui.painter();
        p.rect_filled(net_r, 12.0, theme.card_bg);
        p.rect_stroke(net_r, 12.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);

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
        let connected = hw.ac_power_connected == Some(true) || hw.battery_charging == Some(true);
        let pct = hw.battery_percentage;
        let accent = if connected || pct.is_some_and(|v| v >= 80) {
            theme.accent_green
        } else if pct.is_some_and(|v| v >= 30) {
            theme.accent_amber
        } else if pct.is_some() {
            Color32::from_rgb(239, 68, 68)
        } else {
            theme.pill_text_dim
        };
        let label = match (pct, hw.battery_charging, connected) {
            (None, _, true) => "AC Power",
            (None, _, false) => "No Battery",
            (Some(_), Some(true), _) | (Some(_), _, true) => "Connected",
            (Some(_), _, false) => "Battery",
        };
        let pct_str = pct.map_or_else(|| "--".to_string(), |v| format!("{v}%"));

        let p = tui.painter();
        p.rect_filled(bat_r, 12.0, theme::mix(theme.card_bg, accent, 0.08));
        p.rect_stroke(
            bat_r,
            12.0,
            Stroke::new(1.1, theme::with_alpha(accent, 0.65)),
            egui::StrokeKind::Inside,
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
    let p = ui.painter();
    p.rect_filled(rect, 12.0, theme.card_bg);
    p.rect_stroke(rect, 12.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);

    let ig = p.layout_no_wrap(icon.into(), icon_font(14.5, false), accent);
    p.galley(pos2(rect.min.x + 9.0, rect.center().y - ig.size().y * 0.5), ig, accent);

    let text_x = rect.min.x + 29.0;
    let lg = p.layout_no_wrap(label.into(), Theme::font_semibold(10.0), theme.pill_text_dim);
    p.galley(pos2(text_x, rect.min.y + 7.0), lg, theme.pill_text_dim);

    let vg = p.layout_no_wrap(value.into(), Theme::font_semibold(10.5), theme.pill_text);
    p.galley(pos2(rect.max.x - 9.0 - vg.size().x, rect.min.y + 6.5), vg, theme.pill_text);

    let bar_r = Rect::from_min_max(
        pos2(text_x, rect.max.y - 10.0),
        pos2(rect.max.x - 9.0, rect.max.y - 6.0),
    );
    progress_bar(p, bar_r, frac, accent, theme.track_bg);
}

fn progress_bar(
    painter: &egui::Painter,
    rect: Rect,
    frac: f32,
    accent: Color32,
    track: Color32,
) {
    painter.rect_filled(rect, 2.0, track);
    let w = (rect.width() * frac.clamp(0.0, 1.0)).max(2.0);
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(w, rect.height())), 2.0, accent);
}

fn format_net_speed(bps: u64) -> String {
    let mbps = (bps as f64 * 8.0) / 1_000_000.0;
    format!("{mbps:.1} Mbps")
}
