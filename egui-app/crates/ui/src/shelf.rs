//! Gate 5 File Shelf UI matching the mockup (`media_1795283267358_98990467.jpg`):
//! - Header: [←] "Shelf" [count] ... [+] [⋯]
//! - Segmented tabs: All, Files, Links, Text
//! - ~3 visible rows with vertical ScrollArea, image thumbnails / file icons,
//!   red missing-file dot, click-to-open/copy, hover action icons (Copy, Reveal, Remove).

use crate::panel::{PanelAction, with_tile_transform};
use crate::pill::icon_font;
use crate::theme::{self, Theme};
use crate::{icons, icons_fill};
use egui::{
    Color32, Context, Rect, ScrollArea, Sense, Stroke, TextureId, Ui, Vec2, pos2, vec2,
};
use orbit_core::events::ShelfItem;
use std::collections::HashMap;

pub const SHELF_PANEL_SIZE: Vec2 = vec2(376.0, 200.0);
pub const SHELF_RADIUS: f32 = 24.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShelfFilter {
    #[default]
    All,
    Files,
    Links,
    Text,
}

#[derive(Clone, Debug, Default)]
pub struct ShelfUiState {
    pub open: bool,
    pub filter: ShelfFilter,
    pub adding: bool,
    pub add_input: String,
    pub menu_open: bool,
    pub status_msg: Option<(String, bool, f64)>,
}

impl ShelfUiState {
    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool, now_ui_secs: f64) {
        self.status_msg = Some((msg.into(), is_error, now_ui_secs + 3.2));
    }

    pub fn filtered_items<'a>(&self, items: &'a [ShelfItem]) -> Vec<&'a ShelfItem> {
        items
            .iter()
            .filter(|item| match self.filter {
                ShelfFilter::All => true,
                ShelfFilter::Files => item.kind == "file",
                ShelfFilter::Links => item.kind == "link",
                ShelfFilter::Text => item.kind == "text",
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &Context,
    _ui: &mut Ui,
    panel_rect: Rect,
    items: &[ShelfItem],
    storage_mode: &str,
    thumbnails: &HashMap<String, TextureId>,
    theme: &Theme,
    state: &mut ShelfUiState,
    reveal_t: f32,
    actions: &mut Vec<PanelAction>,
) {
    let now = ctx.input(|i| i.time);
    if let Some((_, _, until)) = state.status_msg
        && now >= until
    {
        state.status_msg = None;
    }

    let inner = Rect::from_min_max(
        pos2(panel_rect.min.x + 14.0, panel_rect.min.y + 11.0),
        pos2(panel_rect.max.x - 14.0, panel_rect.max.y - 10.0),
    );

    // Header row: [←] Shelf [N] ... [+] [⋯]
    let hdr_rect = Rect::from_min_size(inner.min, vec2(inner.width(), 24.0));
    with_tile_transform(ctx, "shelf-header", hdr_rect, 0, reveal_t, |tui| {
        let back_r = Rect::from_min_size(hdr_rect.min, vec2(24.0, 22.0));
        if hdr_icon_btn(tui, back_r, theme, "shelf-back", icons::ARROW_LEFT, theme.pill_text) {
            state.open = false;
            state.adding = false;
            state.menu_open = false;
        }

        let p = tui.painter();
        let title_g = p.layout_no_wrap("Shelf".into(), Theme::font_semibold(13.0), theme.pill_text);
        let title_x = back_r.max.x + 10.0;
        let title_w = title_g.size().x;
        p.galley(
            pos2(title_x, hdr_rect.center().y - title_g.size().y * 0.5),
            title_g,
            theme.pill_text,
        );

        let badge_r = Rect::from_center_size(
            pos2(title_x + title_w + 15.0, hdr_rect.center().y),
            vec2(20.0, 16.0),
        );
        p.rect_filled(
            badge_r,
            6.0,
            Color32::from_rgba_unmultiplied(46, 98, 152, 160),
        );
        let cg = p.layout_no_wrap(
            items.len().to_string(),
            Theme::font_semibold(10.0),
            Color32::from_rgb(185, 220, 255),
        );
        p.galley(badge_r.center() - cg.size() * 0.5, cg, Color32::WHITE);

        let more_r = Rect::from_min_size(pos2(hdr_rect.max.x - 24.0, hdr_rect.min.y), vec2(24.0, 22.0));
        let add_r = Rect::from_min_size(pos2(more_r.min.x - 29.0, hdr_rect.min.y), vec2(24.0, 22.0));
        if hdr_icon_btn(tui, add_r, theme, "shelf-add-btn", icons::PLUS, theme.pill_text) {
            state.adding = !state.adding;
            state.menu_open = false;
            if state.adding {
                state.add_input.clear();
            }
        }
        if hdr_icon_btn(tui, more_r, theme, "shelf-more-btn", icons::DOTS_THREE, theme.pill_text) {
            state.menu_open = !state.menu_open;
            state.adding = false;
        }
    });

    // Centered 4-segment filter tabs
    let tabs_rect = Rect::from_min_size(
        pos2(inner.center().x - 106.0, hdr_rect.max.y + 7.0),
        vec2(212.0, 22.0),
    );
    with_tile_transform(ctx, "shelf-tabs", tabs_rect, 1, reveal_t, |tui| {
        let p = tui.painter();
        p.rect_filled(tabs_rect, 11.0, Color32::from_rgba_unmultiplied(255, 255, 255, 10));
        p.rect_stroke(
            tabs_rect,
            11.0,
            Stroke::new(1.0, theme.card_border),
            egui::StrokeKind::Inside,
        );

        let tabs = [
            (ShelfFilter::All, icons::SQUARES_FOUR),
            (ShelfFilter::Files, icons::FOLDER),
            (ShelfFilter::Links, icons::LINK),
            (ShelfFilter::Text, icons::FILE_TEXT),
        ];
        let seg_w = tabs_rect.width() / 4.0;
        for (idx, (filter, glyph)) in tabs.into_iter().enumerate() {
            let seg_r = Rect::from_min_size(
                pos2(tabs_rect.min.x + idx as f32 * seg_w, tabs_rect.min.y),
                vec2(seg_w, tabs_rect.height()),
            );
            let resp = tui.interact(seg_r, tui.id().with(("shelf-tab", idx)), Sense::click());
            let selected = state.filter == filter;
            let p = tui.painter();
            if selected {
                p.rect_filled(
                    seg_r.shrink(1.0),
                    10.0,
                    Color32::from_rgba_unmultiplied(46, 98, 152, 235),
                );
            } else if resp.hovered() {
                p.rect_filled(
                    seg_r.shrink(1.0),
                    10.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 16),
                );
            }
            let col = if selected {
                Color32::WHITE
            } else {
                theme::mix(theme.pill_text_dim, theme.pill_text, 0.30)
            };
            let ig = p.layout_no_wrap(glyph.into(), icon_font(12.0, false), col);
            p.galley(seg_r.center() - ig.size() * 0.5, ig, col);
            if resp.clicked() {
                state.filter = filter;
            }
        }
    });

    let div_y = tabs_rect.max.y + 6.0;
    let body_top = div_y + 4.0;
    let list_rect = Rect::from_min_max(pos2(inner.min.x, body_top), inner.max);

    with_tile_transform(ctx, "shelf-body", list_rect, 2, reveal_t, |tui| {
        tui.painter().line_segment(
            [pos2(inner.min.x, div_y), pos2(inner.max.x, div_y)],
            Stroke::new(1.0, theme.card_border),
        );

        let mut content_top = list_rect.min.y;

        if state.menu_open {
            let menu_r = Rect::from_min_size(pos2(list_rect.min.x, content_top), vec2(list_rect.width(), 24.0));
            show_overflow_bar(tui, menu_r, storage_mode, theme, state, actions);
            content_top = menu_r.max.y + 4.0;
        } else if state.adding {
            let add_bar_r = Rect::from_min_size(pos2(list_rect.min.x, content_top), vec2(list_rect.width(), 26.0));
            show_add_bar(tui, add_bar_r, theme, state, actions);
            content_top = add_bar_r.max.y + 4.0;
        } else if let Some((msg, is_err, _)) = &state.status_msg {
            let msg_r = Rect::from_min_size(pos2(list_rect.min.x, content_top), vec2(list_rect.width(), 20.0));
            let bg = if *is_err {
                Color32::from_rgba_unmultiplied(180, 45, 55, 70)
            } else {
                Color32::from_rgba_unmultiplied(46, 98, 152, 70)
            };
            let fg = if *is_err {
                Color32::from_rgb(255, 165, 170)
            } else {
                Color32::from_rgb(175, 220, 255)
            };
            tui.painter().rect_filled(msg_r, 7.0, bg);
            let mut job = egui::text::LayoutJob::single_section(
                msg.clone(),
                egui::TextFormat {
                    font_id: Theme::font_semibold(9.5),
                    color: fg,
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: (msg_r.width() - 12.0).max(40.0),
                max_rows: 1,
                break_anywhere: true,
                ..Default::default()
            };
            let mg = tui.painter().layout_job(job);
            tui.painter().galley(
                pos2(msg_r.min.x + 8.0, msg_r.center().y - mg.size().y * 0.5),
                mg,
                fg,
            );
            content_top = msg_r.max.y + 3.0;
        }

        let scroll_rect = Rect::from_min_max(pos2(list_rect.min.x, content_top), list_rect.max);
        let visible = state.filtered_items(items);

        if visible.is_empty() {
            let empty_msg = match state.filter {
                ShelfFilter::All => "Drop files onto ORBIT or click + to add files, links, or text",
                ShelfFilter::Files => "No held files",
                ShelfFilter::Links => "No saved links",
                ShelfFilter::Text => "No saved text snippets",
            };
            let empty_col = theme::mix(theme.pill_text_dim, theme.pill_text, 0.30);
            let eg = tui
                .painter()
                .layout_no_wrap(empty_msg.into(), Theme::font(10.5), empty_col);
            tui.painter()
                .galley(scroll_rect.center() - eg.size() * 0.5, eg, empty_col);
        } else {
            tui.scope_builder(egui::UiBuilder::new().max_rect(scroll_rect), |sui| {
                ScrollArea::vertical()
                    .id_salt("orbit-shelf-scroll")
                    .max_height(scroll_rect.height())
                    .auto_shrink([false, false])
                    .show(sui, |list_ui| {
                        list_ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                        for item in visible {
                            let (row_rect, _) = list_ui.allocate_exact_size(
                                vec2(scroll_rect.width() - 4.0, 38.0),
                                Sense::hover(),
                            );
                            show_shelf_row(
                                list_ui,
                                row_rect,
                                item,
                                thumbnails.get(&item.file_path).copied(),
                                theme,
                                actions,
                            );
                        }
                    });
            });
        }
    });
}

fn show_overflow_bar(
    ui: &mut Ui,
    rect: Rect,
    storage_mode: &str,
    theme: &Theme,
    state: &mut ShelfUiState,
    actions: &mut Vec<PanelAction>,
) {
    let mode_lbl = if storage_mode == "copy" {
        "Mode: Copy-file"
    } else {
        "Mode: Path-only"
    };
    let w3 = (rect.width() - 12.0) / 3.0;
    let r0 = Rect::from_min_size(rect.min, vec2(w3, rect.height()));
    let r1 = Rect::from_min_size(pos2(r0.max.x + 6.0, rect.min.y), vec2(w3, rect.height()));
    let r2 = Rect::from_min_size(pos2(r1.max.x + 6.0, rect.min.y), vec2(w3, rect.height()));

    if small_menu_btn(ui, r0, theme, "ov-mode", mode_lbl, false) {
        actions.push(PanelAction::ShelfToggleStorageMode);
        state.menu_open = false;
    }
    if small_menu_btn(ui, r1, theme, "ov-share", "Share Files", false) {
        actions.push(PanelAction::ShelfShareFiles);
        state.menu_open = false;
    }
    if small_menu_btn(ui, r2, theme, "ov-clear", "Clear Shelf", true) {
        actions.push(PanelAction::ShelfClear);
        state.menu_open = false;
    }
}

fn show_add_bar(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    state: &mut ShelfUiState,
    actions: &mut Vec<PanelAction>,
) {
    let browse_r = Rect::from_min_size(pos2(rect.max.x - 54.0, rect.min.y + 1.0), vec2(54.0, 24.0));
    let add_r = Rect::from_min_size(pos2(browse_r.min.x - 44.0, rect.min.y + 1.0), vec2(40.0, 24.0));
    let edit_r = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 1.0), pos2(add_r.min.x - 6.0, rect.max.y - 1.0));

    let mut submit = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(edit_r), |eui| {
        let resp = eui.add_sized(
            edit_r.size(),
            egui::TextEdit::singleline(&mut state.add_input)
                .hint_text("Link, text, or path..."),
        );
        resp.request_focus();
        if resp.lost_focus() && eui.input(|i| i.key_pressed(egui::Key::Enter)) {
            submit = true;
        }
    });

    if small_menu_btn(ui, add_r, theme, "add-submit", "Add", false) {
        submit = true;
    }
    if small_menu_btn(ui, browse_r, theme, "add-browse", "Browse", false) {
        state.adding = false;
        actions.push(PanelAction::ShelfPickFiles);
    }

    if submit && !state.add_input.trim().is_empty() {
        actions.push(PanelAction::ShelfAddEntry(state.add_input.trim().to_string()));
        state.add_input.clear();
        state.adding = false;
    }
}

fn show_shelf_row(
    ui: &mut Ui,
    rect: Rect,
    item: &ShelfItem,
    thumb_tex: Option<TextureId>,
    theme: &Theme,
    actions: &mut Vec<PanelAction>,
) {
    let hovered = ui.rect_contains_pointer(rect);
    let p = ui.painter();
    if hovered {
        p.rect_filled(rect, 10.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
        p.rect_stroke(rect, 10.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);
    }

    // Left 30x30 icon / thumbnail
    let icon_r = Rect::from_min_size(pos2(rect.min.x + 6.0, rect.center().y - 15.0), vec2(30.0, 30.0));
    paint_item_thumb_or_icon(p, icon_r, item, thumb_tex, theme);

    // Reserve right edge cleanly for hover actions OR missing-file red dot.
    let actions_w = if hovered { 68.0 } else if item.kind == "file" && !item.exists { 20.0 } else { 8.0 };
    let body_click_r = Rect::from_min_max(rect.min, pos2(rect.max.x - actions_w, rect.max.y));

    let text_x = icon_r.max.x + 10.0;
    let text_w = (body_click_r.max.x - text_x - 4.0).max(30.0);

    let mut name_job = egui::text::LayoutJob::single_section(
        item.display_name.clone(),
        egui::TextFormat {
            font_id: Theme::font_semibold(11.5),
            color: theme.pill_text,
            ..Default::default()
        },
    );
    name_job.wrap = egui::text::TextWrapping {
        max_width: text_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let ng = p.layout_job(name_job);
    p.galley(pos2(text_x, rect.min.y + 5.0), ng, theme.pill_text);

    let subtitle = format_item_subtitle(item);
    let sub_col = theme::mix(theme.pill_text_dim, theme.pill_text, 0.30);
    let mut sub_job = egui::text::LayoutJob::single_section(
        subtitle,
        egui::TextFormat {
            font_id: Theme::font(9.5),
            color: sub_col,
            ..Default::default()
        },
    );
    sub_job.wrap = egui::text::TextWrapping {
        max_width: text_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let sg = p.layout_job(sub_job);
    p.galley(pos2(text_x, rect.max.y - sg.size().y - 5.0), sg, sub_col);

    // Primary click / secondary click on row body
    let body_resp = ui.interact(
        body_click_r,
        ui.id().with(("shelf-row-body", &item.id)),
        Sense::click(),
    );
    if body_resp.clicked() {
        if item.kind == "text" {
            actions.push(PanelAction::ShelfCopyItem(item.id.clone()));
        } else {
            actions.push(PanelAction::ShelfOpenItem(item.id.clone()));
        }
    } else if body_resp.secondary_clicked() && item.kind == "file" {
        actions.push(PanelAction::ShelfOpenWithItem(item.id.clone()));
    }

    if !hovered {
        if item.kind == "file" && !item.exists {
            ui.painter().circle_filled(
                pos2(rect.max.x - 10.0, rect.center().y),
                3.2,
                Color32::from_rgb(239, 68, 68),
            );
        }
    } else {
        let cy = rect.center().y;
        let rem_r = Rect::from_center_size(pos2(rect.max.x - 12.0, cy), vec2(18.0, 22.0));
        let mid_r = Rect::from_center_size(pos2(rem_r.min.x - 11.0, cy), vec2(18.0, 22.0));
        let copy_r = Rect::from_center_size(pos2(mid_r.min.x - 11.0, cy), vec2(18.0, 22.0));

        if row_action_btn(ui, copy_r, theme, (&item.id, "copy"), icons::COPY) {
            actions.push(PanelAction::ShelfCopyItem(item.id.clone()));
        }
        let mid_icon = if item.kind == "link" {
            icons::ARROW_SQUARE_OUT
        } else {
            icons::FOLDER
        };
        if row_action_btn(ui, mid_r, theme, (&item.id, "reveal"), mid_icon) {
            if item.kind == "link" {
                actions.push(PanelAction::ShelfOpenItem(item.id.clone()));
            } else {
                actions.push(PanelAction::ShelfRevealItem(item.id.clone()));
            }
        }
        if row_action_btn(ui, rem_r, theme, (&item.id, "rm"), icons::X) {
            actions.push(PanelAction::ShelfRemoveItem(item.id.clone()));
        }
    }
}

fn paint_item_thumb_or_icon(
    painter: &egui::Painter,
    rect: Rect,
    item: &ShelfItem,
    thumb_tex: Option<TextureId>,
    theme: &Theme,
) {
    if let Some(tex) = thumb_tex {
        let mut mesh = egui::Mesh::with_texture(tex);
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.add(egui::Shape::mesh(mesh));
        painter.rect_stroke(rect, 7.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);
        return;
    }

    let ext = item.extension.as_str();
    let (glyph, bg, fg) = if item.kind == "link" {
        (icons::LINK, Color32::from_rgba_unmultiplied(46, 98, 152, 160), theme.accent_blue)
    } else if item.kind == "text" {
        (icons::FILE_TEXT, theme.card_bg, theme.accent_cyan)
    } else if ext == "pdf" {
        (icons_fill::FILE_PDF, Color32::from_rgb(236, 240, 246), Color32::from_rgb(220, 56, 56))
    } else if matches!(ext, "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp") {
        (icons::IMAGE, theme.card_bg, theme.accent_purple)
    } else {
        (icons::FILE_TEXT, theme.card_bg, theme::mix(theme.pill_text_dim, theme.pill_text, 0.30))
    };

    painter.rect_filled(rect, 7.0, bg);
    painter.rect_stroke(rect, 7.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);
    let g = painter.layout_no_wrap(glyph.into(), icon_font(15.0, ext == "pdf"), fg);
    painter.galley(rect.center() - g.size() * 0.5, g, fg);
}

fn format_item_subtitle(item: &ShelfItem) -> String {
    if item.kind == "link" {
        return "Link · saved".to_string();
    }
    if item.kind == "text" {
        return format!("{} chars · Text", item.file_size);
    }
    let size_str = if item.file_size < 1024 {
        format!("{} B", item.file_size)
    } else if item.file_size < 1024 * 1024 {
        format!("{:.0} KB", item.file_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", item.file_size as f64 / (1024.0 * 1024.0))
    };
    if matches!(
        item.extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
    ) {
        format!("{size_str} · Image")
    } else if !item.exists {
        format!("{size_str} · Missing")
    } else {
        size_str
    }
}

fn hdr_icon_btn(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    key: &str,
    icon: &str,
    color: Color32,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(key), Sense::click());
    let bg = if resp.hovered() {
        Color32::from_rgba_unmultiplied(255, 255, 255, 24)
    } else {
        theme.card_bg
    };
    let p = ui.painter();
    p.rect_filled(rect, 7.0, bg);
    p.rect_stroke(rect, 7.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);
    let g = p.layout_no_wrap(icon.into(), icon_font(12.5, false), color);
    p.galley(rect.center() - g.size() * 0.5, g, color);
    resp.clicked()
}

fn small_menu_btn(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    key: &str,
    label: &str,
    danger: bool,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(key), Sense::click());
    let base = if danger {
        Color32::from_rgba_unmultiplied(180, 45, 55, 65)
    } else {
        theme.pill_btn_bg
    };
    let bg = if resp.hovered() {
        theme::mix(base, if danger { Color32::RED } else { theme.accent_blue }, 0.25)
    } else {
        base
    };
    let p = ui.painter();
    p.rect_filled(rect, 7.0, bg);
    p.rect_stroke(rect, 7.0, Stroke::new(1.0, theme.card_border), egui::StrokeKind::Inside);
    let col = if danger {
        Color32::from_rgb(255, 165, 170)
    } else {
        theme.pill_text
    };
    let g = p.layout_no_wrap(label.into(), Theme::font_semibold(9.5), col);
    p.galley(rect.center() - g.size() * 0.5, g, col);
    resp.clicked()
}

fn row_action_btn(
    ui: &mut Ui,
    rect: Rect,
    theme: &Theme,
    key: (&str, &str),
    icon: &str,
) -> bool {
    let resp = ui.interact(rect, ui.id().with(("shelf-act", key.0, key.1)), Sense::click());
    let col = if resp.hovered() {
        theme.pill_text
    } else {
        theme::mix(theme.pill_text_dim, theme.pill_text, 0.30)
    };
    let g = ui
        .painter()
        .layout_no_wrap(icon.into(), icon_font(11.5, false), col);
    ui.painter().galley(rect.center() - g.size() * 0.5, g, col);
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_entry(id: &str, kind: &str) -> ShelfItem {
        ShelfItem {
            id: id.into(),
            file_path: format!("val_{id}"),
            display_name: format!("item_{id}"),
            file_size: 100,
            extension: if kind == "link" { "link" } else { "txt" }.into(),
            added_at: 1,
            exists: true,
            is_managed_copy: false,
            kind: kind.into(),
        }
    }

    #[test]
    fn shelf_tabs_filter_all_files_links_text() {
        let items = vec![
            mk_entry("1", "file"),
            mk_entry("2", "link"),
            mk_entry("3", "text"),
        ];
        let mut st = ShelfUiState::default();
        assert_eq!(st.filtered_items(&items).len(), 3);
        st.filter = ShelfFilter::Files;
        assert_eq!(st.filtered_items(&items).len(), 1);
        assert_eq!(st.filtered_items(&items)[0].id, "1");
        st.filter = ShelfFilter::Links;
        assert_eq!(st.filtered_items(&items).len(), 1);
        assert_eq!(st.filtered_items(&items)[0].id, "2");
        st.filter = ShelfFilter::Text;
        assert_eq!(st.filtered_items(&items).len(), 1);
        assert_eq!(st.filtered_items(&items)[0].id, "3");
    }
}
