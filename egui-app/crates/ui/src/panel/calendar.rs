//! Calendar section matching the user's screenshot:
//! left date hero + "+ Add Task" button, right 6-row month grid with category rings.

use super::{PanelAction, PanelState, with_tile_transform};
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::icons;
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
    let today_y = if snap.clock.year >= 2024 { snap.clock.year } else { 2026 };
    let today_m = if (1..=12).contains(&snap.clock.month) { snap.clock.month } else { 11 };
    let today_d = if (1..=31).contains(&snap.clock.day) { snap.clock.day } else { 21 };

    if state.calendar_year == 0 {
        state.calendar_year = today_y;
        state.calendar_month = today_m;
        state.calendar_selected_day = today_d;
    }

    let inner = Rect::from_min_max(
        pos2(panel_rect.min.x + 12.0, panel_rect.min.y + 11.0),
        pos2(panel_rect.max.x - 18.0, panel_rect.max.y - 11.0),
    );

    // Left hero sub-card (~108px wide) matching screenshot 4.
    let left_rect = Rect::from_min_size(inner.min, vec2(106.0, inner.height()));
    let view_year = state.calendar_year;
    let view_month = state.calendar_month;
    let sel_day = state
        .calendar_selected_day
        .clamp(1, days_in_month(view_year, view_month));
    state.calendar_selected_day = sel_day;

    with_tile_transform(ctx, "cal-left-hero", left_rect, 0, reveal_t, |tui| {
        let p = tui.painter();
        p.rect_filled(left_rect, 18.0, theme.card_bg);
        p.rect_stroke(
            left_rect,
            18.0,
            Stroke::new(1.0, theme.card_border),
            egui::StrokeKind::Inside,
        );

        let cx = left_rect.center().x;
        let month_g = p.layout_no_wrap(
            month_name(view_month).into(),
            Theme::font_semibold(13.0),
            theme.accent_blue,
        );
        p.galley(pos2(cx - month_g.size().x * 0.5, left_rect.min.y + 12.0), month_g, theme.accent_blue);

        let muted_text = theme::mix(theme.pill_text_dim, theme.pill_text, 0.30);
        let year_g = p.layout_no_wrap(
            view_year.to_string(),
            Theme::font(10.5),
            muted_text,
        );
        p.galley(pos2(cx - year_g.size().x * 0.5, left_rect.min.y + 29.0), year_g, muted_text);

        let day_g = p.layout_no_wrap(
            sel_day.to_string(),
            Theme::font_semibold(36.0),
            theme.pill_text,
        );
        p.galley(pos2(cx - day_g.size().x * 0.5, left_rect.min.y + 42.0), day_g, theme.pill_text);

        let wd = weekday_of_ymd(view_year, view_month, sel_day);
        let wd_g = p.layout_no_wrap(
            weekday_name(wd).into(),
            Theme::font(11.5),
            muted_text,
        );
        p.galley(pos2(cx - wd_g.size().x * 0.5, left_rect.max.y - 52.0), wd_g, muted_text);

        let btn_rect = Rect::from_min_size(
            pos2(left_rect.min.x + 10.0, left_rect.max.y - 31.0),
            vec2(left_rect.width() - 20.0, 22.0),
        );
        let resp = tui.interact(btn_rect, tui.id().with("cal-add-task"), Sense::click());
        let btn_col = if resp.hovered() {
            Color32::from_rgb(56, 148, 255)
        } else {
            theme.accent_blue
        };
        let p = tui.painter();
        p.rect_filled(btn_rect, 11.0, btn_col);
        p.rect_stroke(
            btn_rect,
            11.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(190, 225, 255, 160)),
            egui::StrokeKind::Inside,
        );
        let lbl = if state.adding_task { "Cancel" } else { "+ Add Task" };
        let bg = p.layout_no_wrap(lbl.into(), Theme::font_semibold(10.5), Color32::WHITE);
        p.galley(btn_rect.center() - bg.size() * 0.5, bg, Color32::WHITE);
        if resp.clicked() {
            state.adding_task = !state.adding_task;
            state.new_task_title.clear();
        }
    });

    // Right month grid (or inline task editor when + Add Task is toggled).
    let right_rect = Rect::from_min_max(
        pos2(left_rect.max.x + 10.0, inner.min.y),
        inner.max,
    );
    with_tile_transform(ctx, "cal-right-grid", right_rect, 1, reveal_t, |tui| {
        if state.adding_task {
            show_add_task_form(tui, right_rect, theme, state, view_year, view_month, sel_day, actions);
        } else {
            show_month_grid(
                tui,
                right_rect,
                snap,
                theme,
                state,
                (today_y, today_m, today_d),
                actions,
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn show_add_task_form(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    state: &mut PanelState,
    year: i32,
    month: u32,
    day: u32,
    actions: &mut Vec<PanelAction>,
) {
    let p = ui.painter();
    p.rect_filled(rect, 14.0, theme.card_bg);
    p.rect_stroke(rect, 14.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);

    let title_str = format!("Task for {} {}", month_name(month), day);
    let tg = p.layout_no_wrap(title_str, Theme::font_semibold(12.0), theme.pill_text);
    p.galley(pos2(rect.min.x + 12.0, rect.min.y + 12.0), tg, theme.pill_text);

    let edit_rect = Rect::from_min_size(
        pos2(rect.min.x + 12.0, rect.min.y + 38.0),
        vec2(rect.width() - 24.0, 28.0),
    );
    let mut submitted = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(edit_rect), |eui| {
        let resp = eui.add_sized(
            edit_rect.size(),
            egui::TextEdit::singleline(&mut state.new_task_title)
                .hint_text("Enter reminder title..."),
        );
        resp.request_focus();
        if resp.lost_focus() && eui.input(|i| i.key_pressed(egui::Key::Enter)) {
            submitted = true;
        }
    });

    let save_rect = Rect::from_min_size(
        pos2(rect.min.x + 12.0, rect.max.y - 36.0),
        vec2(84.0, 24.0),
    );
    let resp = ui.interact(save_rect, ui.id().with("save-task"), Sense::click());
    let p = ui.painter();
    p.rect_filled(save_rect, 12.0, theme.accent_blue);
    let sg = p.layout_no_wrap("Save Task".into(), Theme::font_semibold(10.5), Color32::WHITE);
    p.galley(save_rect.center() - sg.size() * 0.5, sg, Color32::WHITE);

    if (resp.clicked() || submitted) && !state.new_task_title.trim().is_empty() {
        actions.push(PanelAction::AddReminder {
            title: state.new_task_title.trim().to_string(),
            due_timestamp_ms: ymd_noon_unix_ms(year, month, day),
        });
        state.new_task_title.clear();
        state.adding_task = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn show_month_grid(
    ui: &mut Ui,
    rect: Rect,
    snap: &AppSnapshot,
    theme: &Theme,
    state: &mut PanelState,
    today: (i32, u32, u32),
    actions: &mut Vec<PanelAction>,
) {
    // Header row: < [S M T W T F S] > with prev at top-left and next at top-right.
    let grid_x0 = rect.min.x + 16.0;
    let grid_w = (rect.width() - 32.0).max(70.0);
    let col_w = grid_w / 7.0;
    let header_y = rect.min.y + 8.0;
    let day_headers = ["S", "M", "T", "W", "T", "F", "S"];
    let header_col = theme::mix(theme.pill_text_dim, theme.pill_text, 0.35);
    let p = ui.painter();
    for (i, d) in day_headers.iter().enumerate() {
        let g = p.layout_no_wrap((*d).into(), Theme::font(10.0), header_col);
        let cx = grid_x0 + (i as f32 + 0.5) * col_w;
        p.galley(pos2(cx - g.size().x * 0.5, header_y - g.size().y * 0.5), g, header_col);
    }

    let prev_r = Rect::from_center_size(pos2(rect.min.x + 7.0, header_y), vec2(14.0, 16.0));
    let next_r = Rect::from_center_size(pos2(rect.max.x - 7.0, header_y), vec2(14.0, 16.0));
    if nav_arrow_btn(ui, prev_r, theme, icons::CARET_LEFT) {
        if state.calendar_month == 1 {
            state.calendar_month = 12;
            state.calendar_year -= 1;
        } else {
            state.calendar_month -= 1;
        }
    }
    if nav_arrow_btn(ui, next_r, theme, icons::CARET_RIGHT) {
        if state.calendar_month == 12 {
            state.calendar_month = 1;
            state.calendar_year += 1;
        } else {
            state.calendar_month += 1;
        }
    }

    let div_y = rect.min.y + 20.0;
    ui.painter().line_segment(
        [pos2(rect.min.x, div_y), pos2(rect.max.x, div_y)],
        Stroke::new(1.0, theme.card_border),
    );

    let grid_top = div_y + 4.0;
    let row_h = (rect.max.y - grid_top) / 6.0;
    let first_wd = weekday_of_ymd(state.calendar_year, state.calendar_month, 1);
    let total_days = days_in_month(state.calendar_year, state.calendar_month);
    let (prev_y, prev_m) = if state.calendar_month == 1 {
        (state.calendar_year - 1, 12)
    } else {
        (state.calendar_year, state.calendar_month - 1)
    };
    let prev_days = days_in_month(prev_y, prev_m);

    for slot in 0..37u32 {
        let row = slot / 7;
        let col = slot % 7;
        let cx = grid_x0 + (col as f32 + 0.5) * col_w;
        let cy = grid_top + (row as f32 + 0.5) * row_h;

        if slot < first_wd {
            let day = prev_days - (first_wd - 1 - slot);
            let overflow_col = theme::with_alpha(
                theme::mix(theme.pill_text_dim, theme.pill_text, 0.30),
                0.65,
            );
            let g = ui.painter().layout_no_wrap(
                day.to_string(),
                Theme::font(10.5),
                overflow_col,
            );
            ui.painter()
                .galley(pos2(cx - g.size().x * 0.5, cy - g.size().y * 0.5), g, overflow_col);
            continue;
        }
        let day = slot - first_wd + 1;
        if day > total_days {
            continue;
        }

        let cell_r = Rect::from_center_size(pos2(cx, cy), vec2(21.0, 21.0));
        let resp = ui.interact(cell_r, ui.id().with(("cal-day", slot)), Sense::click());
        let is_selected = day == state.calendar_selected_day;
        let is_today = (state.calendar_year, state.calendar_month, day) == today;

        // Check if any reminder falls on this day.
        let matching_rem = snap.reminders.iter().find(|r| {
            let (ry, rm, rd) = unix_ms_to_ymd(r.due_timestamp_ms);
            (ry, rm, rd) == (state.calendar_year, state.calendar_month, day)
        });

        let p = ui.painter();
        if is_selected {
            p.circle_filled(pos2(cx, cy), 10.5, theme.accent_blue);
        } else if is_today {
            p.circle_filled(pos2(cx, cy), 10.5, theme::with_alpha(theme.accent_blue, 0.3));
        }

        if let Some(rem) = matching_rem
            && !is_selected
        {
            let ring_col = category_color(&rem.category, theme);
            p.circle_filled(pos2(cx, cy), 10.5, theme::with_alpha(ring_col, 0.16));
            p.circle_stroke(pos2(cx, cy), 10.5, Stroke::new(1.1, ring_col));
        } else if resp.hovered() && !is_selected {
            p.circle_stroke(pos2(cx, cy), 10.5, Stroke::new(1.0, theme.card_border));
        }

        let text_col = if is_selected {
            Color32::WHITE
        } else if is_today || matching_rem.is_some() {
            theme.pill_text
        } else if col == 0 || col == 6 {
            theme::mix(theme.pill_text_dim, theme.pill_text, 0.30)
        } else {
            theme.pill_text
        };
        let dg = p.layout_no_wrap(day.to_string(), Theme::font(10.5), text_col);
        p.galley(pos2(cx - dg.size().x * 0.5, cy - dg.size().y * 0.5), dg, text_col);

        if resp.clicked() {
            state.calendar_selected_day = day;
            if let Some(rem) = matching_rem
                && resp.double_clicked()
            {
                actions.push(PanelAction::ToggleReminder(rem.id.clone()));
            }
        }
    }
}

fn nav_arrow_btn(ui: &mut Ui, rect: Rect, theme: &Theme, icon: &str) -> bool {
    let resp = ui.interact(rect, ui.id().with(("cal-nav", icon)), Sense::click());
    let col = if resp.hovered() {
        theme.pill_text
    } else {
        theme::mix(theme.pill_text_dim, theme.pill_text, 0.30)
    };
    let g = ui
        .painter()
        .layout_no_wrap(icon.into(), icon_font(10.0, false), col);
    ui.painter().galley(rect.center() - g.size() * 0.5, g, col);
    resp.clicked()
}

fn category_color(cat: &str, theme: &Theme) -> Color32 {
    match cat.to_ascii_lowercase().as_str() {
        "exam" | "deadline" | "due" => Color32::from_rgb(251, 113, 133),
        "holiday" => theme.accent_green,
        "meeting" => theme.accent_purple,
        "event" | "class" => theme.accent_cyan,
        _ => theme.accent_green,
    }
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 30,
    }
}

/// Sakamoto weekday algorithm: returns `0 = Sunday .. 6 = Saturday`.
pub fn weekday_of_ymd(year: i32, month: u32, day: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    let idx = (month.clamp(1, 12) - 1) as usize;
    ((y + y / 4 - y / 100 + y / 400 + T[idx] + day as i32).rem_euclid(7)) as u32
}

/// Converts civil date to noon UTC epoch milliseconds so timezone offsets stay within the same day.
pub fn ymd_noon_unix_ms(year: i32, month: u32, day: u32) -> u64 {
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + 43_200;
    secs.max(0) as u64 * 1000
}

/// Converts Unix epoch ms back to `(year, month, day)`.
pub fn unix_ms_to_ymd(ms: u64) -> (i32, u32, u32) {
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1;
    }
    (y as i32, m as u32, d as u32)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = i64::from(year);
    let m = i64::from(month);
    let d = i64::from(day);
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Month",
    }
}

fn weekday_name(weekday: u32) -> &'static str {
    match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        _ => "Saturday",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nov_2026_matches_real_calendar() {
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(weekday_of_ymd(2026, 11, 1), 0); // Sunday
        assert_eq!(weekday_of_ymd(2026, 11, 21), 6); // Saturday
    }

    #[test]
    fn leap_year_and_roundtrip_ymd() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        let ms = ymd_noon_unix_ms(2026, 11, 21);
        assert_eq!(unix_ms_to_ymd(ms), (2026, 11, 21));
    }
}
