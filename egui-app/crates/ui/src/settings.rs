//! Gate 6 / Bento Settings Studio: pure immediate-mode Settings panels rendered inside the
//! on-demand second viewport (`orbit-settings`). No Win32 calls or disk I/O live here.

use crate::icons;
use crate::theme::{self, Theme};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Slider, Stroke, TextEdit, Ui, Visuals, pos2, vec2,
};
use orbit_core::config::{
    ColorTheme, CompactRightSlot, Config, ExpandTrigger, GlassMode, MotionPreset, Placement,
    SettingsAppearance, ThemeMode, TimeFormat, WeatherUnit,
};

/// Filter category for the Settings Studio navigation bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    #[default]
    All,
    Themes,
    MotionSurface,
    BehaviorPill,
    PrivacyNotifications,
    Data,
}

impl SettingsCategory {
    const ALL_TABS: [(Self, &str); 6] = [
        (Self::All, "All"),
        (Self::Themes, "Themes"),
        (Self::MotionSurface, "3D & Motion"),
        (Self::BehaviorPill, "Behavior & Pill"),
        (Self::PrivacyNotifications, "Privacy & Alerts"),
        (Self::Data, "Memory & Data"),
    ];
}

/// Transient UI-only state owned by the Settings window while open and dropped on close.
#[derive(Clone, Debug)]
pub struct SettingsUiState {
    pub blocked_app_input: String,
    pub confirm_clear_data: bool,
    pub status_banner: Option<(String, bool)>,
    pub notif_access_status: String,
    pub requesting_notif_access: bool,
    pub show_privacy_terms: bool,
    pub active_category: SettingsCategory,
    pub orbit_mem_mb: Option<f32>,
    pub is_refreshing_mem: bool,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            blocked_app_input: String::new(),
            confirm_clear_data: false,
            status_banner: None,
            notif_access_status: "checking...".to_string(),
            requesting_notif_access: false,
            show_privacy_terms: false,
            active_category: SettingsCategory::All,
            orbit_mem_mb: None,
            is_refreshing_mem: false,
        }
    }
}

/// Actions emitted by `show_settings` for `OrbitApp` to persist or execute off-thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    ConfigChanged(Config),
    RequestNotificationAccess,
    ClearNotifications,
    ResetDefaults,
    ClearLocalData,
    RevealConfigFolder,
    RefreshMemory,
    TrimMemoryNow,
    Close,
}

#[derive(Clone, Copy)]
struct Palette {
    is_light: bool,
    canvas_bg: Color32,
    card_bg: Color32,
    card_border: Color32,
    card_highlight: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    accent_text: Color32,
    accent_soft: Color32,
    chip_bg: Color32,
    chip_border: Color32,
    input_bg: Color32,
    danger: Color32,
    danger_soft: Color32,
    success: Color32,
    success_soft: Color32,
    warning: Color32,
    warning_soft: Color32,
}

impl Palette {
    fn from_config(cfg: &Config, system_light: bool) -> Self {
        let active_theme = Theme::for_config(cfg.theme, cfg.color_theme, system_light);
        match cfg.settings_appearance {
            SettingsAppearance::Dark => Self::studio_dark(active_theme.accent),
            SettingsAppearance::Light => Self::studio_light(active_theme.accent),
            SettingsAppearance::System => {
                if system_light {
                    Self::studio_light(active_theme.accent)
                } else {
                    Self::studio_dark(active_theme.accent)
                }
            }
            SettingsAppearance::Orbit => Self::from_theme(&active_theme),
        }
    }

    fn studio_dark(raw_accent: Color32) -> Self {
        let accent = ensure_bright_accent(raw_accent);
        let accent_text = theme::mix(accent, Color32::WHITE, 0.28);
        let danger = Color32::from_rgb(248, 113, 113);
        let success = Color32::from_rgb(52, 211, 153);
        let warning = Color32::from_rgb(251, 191, 36);
        Self {
            is_light: false,
            canvas_bg: Color32::from_rgb(0x09, 0x0D, 0x16),
            card_bg: Color32::from_rgb(0x12, 0x18, 0x26),
            card_border: Color32::from_rgba_unmultiplied(148, 163, 184, 50),
            card_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            text: Color32::WHITE,
            muted: Color32::from_rgb(0xD4, 0xDE, 0xEE),
            accent,
            accent_text,
            accent_soft: theme::with_alpha(accent, 0.18),
            chip_bg: Color32::from_rgb(0x19, 0x22, 0x34),
            chip_border: Color32::from_rgba_unmultiplied(148, 163, 184, 68),
            input_bg: Color32::from_rgb(0x0B, 0x10, 0x1B),
            danger,
            danger_soft: theme::with_alpha(danger, 0.16),
            success,
            success_soft: theme::with_alpha(success, 0.16),
            warning,
            warning_soft: theme::with_alpha(warning, 0.16),
        }
    }

    fn studio_light(raw_accent: Color32) -> Self {
        let accent = ensure_vivid_light_accent(raw_accent);
        let accent_text = darken_for_light(accent);
        let danger = Color32::from_rgb(220, 38, 38);
        let success = Color32::from_rgb(5, 150, 105);
        let warning = Color32::from_rgb(180, 83, 9);
        Self {
            is_light: true,
            canvas_bg: Color32::from_rgb(0xF1, 0xF5, 0xF9),
            card_bg: Color32::WHITE,
            card_border: Color32::from_rgb(0xD8, 0xE2, 0xEE),
            card_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 235),
            text: Color32::from_rgb(0x0F, 0x17, 0x2A),
            muted: Color32::from_rgb(0x47, 0x55, 0x69),
            accent,
            accent_text,
            accent_soft: theme::with_alpha(accent, 0.14),
            chip_bg: Color32::from_rgb(0xF8, 0xFA, 0xFC),
            chip_border: Color32::from_rgb(0xCB, 0xD5, 0xE1),
            input_bg: Color32::WHITE,
            danger,
            danger_soft: theme::with_alpha(danger, 0.12),
            success,
            success_soft: theme::with_alpha(success, 0.12),
            warning,
            warning_soft: theme::with_alpha(warning, 0.13),
        }
    }

    fn from_theme(theme: &Theme) -> Self {
        let opaque = |c: Color32| Color32::from_rgb(c.r(), c.g(), c.b());
        if theme.is_light {
            let base_canvas = opaque(theme.opaque_surface);
            let accent = ensure_vivid_light_accent(theme.accent);
            let danger = Color32::from_rgb(220, 38, 38);
            let success = Color32::from_rgb(5, 150, 105);
            let warning = Color32::from_rgb(180, 83, 9);
            Self {
                is_light: true,
                canvas_bg: theme::mix(base_canvas, accent, 0.05),
                card_bg: theme::mix(Color32::WHITE, accent, 0.025),
                card_border: theme::mix(Color32::from_rgb(212, 222, 235), accent, 0.16),
                card_highlight: Color32::from_rgba_unmultiplied(255, 255, 255, 225),
                text: Color32::from_rgb(0x0F, 0x17, 0x2A),
                muted: Color32::from_rgb(0x47, 0x55, 0x69),
                accent,
                accent_text: darken_for_light(accent),
                accent_soft: theme::with_alpha(accent, 0.14),
                chip_bg: theme::mix(Color32::from_rgb(241, 245, 249), accent, 0.05),
                chip_border: theme::mix(Color32::from_rgb(198, 210, 226), accent, 0.20),
                input_bg: Color32::WHITE,
                danger,
                danger_soft: theme::with_alpha(danger, 0.12),
                success,
                success_soft: theme::with_alpha(success, 0.12),
                warning,
                warning_soft: theme::with_alpha(warning, 0.13),
            }
        } else {
            let base_surface = opaque(theme.opaque_surface);
            let card_bg = theme::mix(base_surface, opaque(theme.card_top), 0.42);
            let accent = ensure_bright_accent(theme.accent);
            let danger = Color32::from_rgb(248, 113, 113);
            let success = theme.accent_green;
            let warning = theme.accent_amber;
            Self {
                is_light: false,
                canvas_bg: theme::mix(Color32::BLACK, base_surface, 0.72),
                card_bg,
                card_border: theme.card_border,
                card_highlight: theme.card_highlight,
                text: Color32::WHITE,
                muted: Color32::from_rgb(0xD4, 0xDE, 0xEE),
                accent,
                accent_text: theme::mix(accent, Color32::WHITE, 0.28),
                accent_soft: theme::with_alpha(accent, 0.18),
                chip_bg: theme::mix(card_bg, Color32::WHITE, 0.06),
                chip_border: theme::with_alpha(theme::mix(Color32::WHITE, accent, 0.26), 0.22),
                input_bg: theme::mix(Color32::BLACK, base_surface, 0.50),
                danger,
                danger_soft: theme::with_alpha(danger, 0.16),
                success,
                success_soft: theme::with_alpha(success, 0.16),
                warning,
                warning_soft: theme::with_alpha(warning, 0.16),
            }
        }
    }
}

fn ensure_bright_accent(c: Color32) -> Color32 {
    let luma = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
    if luma < 115.0 {
        theme::mix(c, Color32::from_rgb(56, 189, 248), 0.55)
    } else {
        c
    }
}

fn ensure_vivid_light_accent(c: Color32) -> Color32 {
    let luma = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
    if luma > 185.0 {
        theme::mix(c, Color32::from_rgb(2, 132, 199), 0.48)
    } else {
        c
    }
}

fn darken_for_light(c: Color32) -> Color32 {
    let luma = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
    if luma > 140.0 {
        theme::mix(c, Color32::from_rgb(15, 23, 42), 0.36)
    } else {
        c
    }
}

pub fn canvas_color(theme_mode: ThemeMode, system_light: bool) -> Color32 {
    let theme = Theme::for_config(theme_mode, ColorTheme::Obsidian, system_light);
    Palette::from_theme(&theme).canvas_bg
}

pub fn canvas_color_for(cfg: &Config, system_light: bool) -> Color32 {
    Palette::from_config(cfg, system_light).canvas_bg
}

/// Renders the complete ORBIT Bento Settings Studio inside a native viewport's `CentralPanel`.
pub fn show_settings(
    ui: &mut Ui,
    cfg: &Config,
    system_light: bool,
    os_transparency: bool,
    logo_tex: Option<egui::TextureId>,
    state: &mut SettingsUiState,
) -> Vec<SettingsAction> {
    let pal = Palette::from_config(cfg, system_light);
    apply_local_visuals(ui, &pal);

    let mut next_cfg = cfg.clone();
    let mut actions = Vec::new();

    header_bar(ui, &pal, &mut next_cfg, logo_tex, &mut actions);
    ui.add_space(8.0);
    category_filter_bar(ui, &pal, &mut state.active_category);
    ui.add_space(10.0);

    if let Some((msg, is_err)) = state.status_banner.clone() {
        status_banner(ui, &pal, &msg, is_err);
        ui.add_space(10.0);
    }

    ScrollArea::vertical()
        .id_salt("orbit-settings-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            ui.set_min_width(avail);

            match state.active_category {
                SettingsCategory::All => {
                    theme_showcase_card(ui, &pal, &mut next_cfg);
                    ui.add_space(12.0);
                    if avail >= 740.0 {
                        let gap = 12.0;
                        let col_w = ((avail - gap) * 0.5).floor().max(260.0);
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            ui.allocate_ui_with_layout(
                                vec2(col_w, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.set_min_width(col_w);
                                    ui.set_max_width(col_w);
                                    glass_placement_bento_card(
                                        ui,
                                        &pal,
                                        system_light,
                                        os_transparency,
                                        &mut next_cfg,
                                    );
                                },
                            );
                            ui.allocate_ui_with_layout(
                                vec2(col_w, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.set_min_width(col_w);
                                    ui.set_max_width(col_w);
                                    right_bento_stack(ui, &pal, &mut next_cfg, state, &mut actions);
                                },
                            );
                        });
                    } else {
                        glass_placement_bento_card(
                            ui,
                            &pal,
                            system_light,
                            os_transparency,
                            &mut next_cfg,
                        );
                        ui.add_space(12.0);
                        right_bento_stack(ui, &pal, &mut next_cfg, state, &mut actions);
                    }
                }
                SettingsCategory::Themes => {
                    theme_showcase_card(ui, &pal, &mut next_cfg);
                }
                SettingsCategory::MotionSurface => {
                    motion_surface_focused_card(
                        ui,
                        &pal,
                        system_light,
                        os_transparency,
                        &mut next_cfg,
                    );
                }
                SettingsCategory::BehaviorPill => {
                    behavior_pill_focused_card(ui, &pal, &mut next_cfg);
                }
                SettingsCategory::PrivacyNotifications => {
                    notifications_privacy_card(ui, &pal, &mut next_cfg, state, &mut actions);
                }
                SettingsCategory::Data => {
                    memory_usage_card(ui, &pal, state, &mut actions);
                    ui.add_space(12.0);
                    storage_shelf_about_card(ui, &pal, &mut next_cfg, state, &mut actions);
                }
            }

            ui.add_space(12.0);
        });

    if next_cfg != *cfg {
        actions.push(SettingsAction::ConfigChanged(next_cfg));
    }

    actions
}

fn right_bento_stack(
    ui: &mut Ui,
    pal: &Palette,
    next_cfg: &mut Config,
    state: &mut SettingsUiState,
    actions: &mut Vec<SettingsAction>,
) {
    notifications_privacy_card(ui, pal, next_cfg, state, actions);
    ui.add_space(12.0);
    memory_usage_card(ui, pal, state, actions);
    ui.add_space(12.0);
    storage_shelf_about_card(ui, pal, next_cfg, state, actions);
}

fn theme_showcase_card(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    section_card_with_badge(
        ui,
        pal,
        "CURATED 3D THEME SHOWCASE",
        icons::PALETTE,
        Some("9 Signature Styles"),
        |ui| {
            // Compact Color Mode strip
            if let Some(v) = inline_segmented_row(
                ui,
                pal,
                "Color Mode",
                "Follow Windows brightness or lock dark/light island surfaces.",
                next_cfg.theme,
                &[
                    (ThemeMode::Auto, "Auto"),
                    (ThemeMode::Dark, "Dark"),
                    (ThemeMode::Light, "Light"),
                ],
            ) {
                next_cfg.theme = v;
            }

            ui.add_space(10.0);

            const PRESETS: [ColorTheme; 9] = [
                ColorTheme::Obsidian,
                ColorTheme::OledBlack,
                ColorTheme::LinearIndigo,
                ColorTheme::VisionSpatial,
                ColorTheme::CatppuccinMocha,
                ColorTheme::Cyberpunk,
                ColorTheme::SunsetVelvet,
                ColorTheme::EmeraldAurora,
                ColorTheme::CeramicPearl,
            ];

            let avail = ui.available_width();
            let cols: usize = if avail >= 760.0 {
                3
            } else if avail >= 480.0 {
                2
            } else {
                1
            };
            let gap = 10.0;
            let tile_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(160.0);
            let tile_h = 128.0;

            for row in PRESETS.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &preset in row {
                        let selected = next_cfg.color_theme == preset;
                        if theme_showcase_tile(ui, pal, preset, selected, tile_w, tile_h) {
                            if preset == ColorTheme::CeramicPearl {
                                next_cfg.color_theme = ColorTheme::CeramicPearl;
                                next_cfg.theme = ThemeMode::Light;
                            } else {
                                next_cfg.color_theme = preset;
                                if next_cfg.theme == ThemeMode::Light {
                                    next_cfg.theme = ThemeMode::Dark;
                                }
                            }
                        }
                    }
                });
                ui.add_space(gap);
            }
        },
    );
}

fn theme_showcase_tile(
    ui: &mut Ui,
    pal: &Palette,
    preset: ColorTheme,
    selected: bool,
    width: f32,
    height: f32,
) -> bool {
    let (name, tagline, [bg_col, accent_col, sec_col]) = Theme::color_theme_meta(preset);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let painter = ui.painter();
    let card_fill = if selected {
        theme::mix(pal.card_bg, accent_col, if pal.is_light { 0.08 } else { 0.16 })
    } else if resp.hovered() {
        theme::mix(pal.chip_bg, accent_col, 0.08)
    } else {
        pal.chip_bg
    };

    let card_stroke = if selected {
        Stroke::new(1.75, accent_col)
    } else if resp.hovered() {
        Stroke::new(1.1, theme::with_alpha(accent_col, 0.65))
    } else {
        Stroke::new(1.0, pal.chip_border)
    };

    if selected {
        painter.rect_stroke(
            rect.expand(1.5),
            CornerRadius::same(13),
            Stroke::new(1.25, theme::with_alpha(accent_col, 0.32)),
            egui::StrokeKind::Outside,
        );
    }
    painter.rect(
        rect,
        CornerRadius::same(12),
        card_fill,
        card_stroke,
        egui::StrokeKind::Inside,
    );

    // 1. Mini Island Capsule Preview (height: 30px, pill rounded)
    let pad_x = 10.0;
    let capsule_rect = Rect::from_min_size(
        rect.min + vec2(pad_x, 10.0),
        vec2((width - pad_x * 2.0).max(80.0), 30.0),
    );
    let preview_is_light = bg_col.r() > 160 && bg_col.g() > 160;
    let capsule_ink = if preview_is_light {
        Color32::from_rgb(0x0F, 0x17, 0x2A)
    } else {
        Color32::WHITE
    };

    // Subtle rim glow around the mini capsule
    painter.rect_stroke(
        capsule_rect.expand(0.75),
        CornerRadius::same(16),
        Stroke::new(1.0, theme::with_alpha(accent_col, 0.35)),
        egui::StrokeKind::Outside,
    );
    painter.rect(
        capsule_rect,
        CornerRadius::same(15),
        bg_col,
        Stroke::new(1.1, theme::with_alpha(accent_col, 0.80)),
        egui::StrokeKind::Inside,
    );
    // Top 1px specular highlight on the capsule
    painter.hline(
        (capsule_rect.min.x + 12.0)..=(capsule_rect.max.x - 12.0),
        capsule_rect.min.y + 1.5,
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, if preview_is_light { 180 } else { 76 }),
        ),
    );

    let cy = capsule_rect.center().y;
    // Left colored dot + bright "9:41"
    painter.circle_filled(pos2(capsule_rect.min.x + 12.0, cy), 3.5, accent_col);
    painter.text(
        pos2(capsule_rect.min.x + 20.0, cy),
        egui::Align2::LEFT_CENTER,
        "9:41",
        Theme::font_semibold(11.0),
        capsule_ink,
    );

    // Right secondary dot + "Island" pill chip
    let island_chip_w = 50.0_f32.min((capsule_rect.width() * 0.42).max(38.0));
    let island_chip_rect = Rect::from_min_size(
        pos2(capsule_rect.max.x - island_chip_w - 5.0, cy - 9.0),
        vec2(island_chip_w, 18.0),
    );
    painter.rect(
        island_chip_rect,
        CornerRadius::same(9),
        theme::with_alpha(sec_col, if preview_is_light { 0.16 } else { 0.24 }),
        Stroke::new(0.8, theme::with_alpha(sec_col, 0.55)),
        egui::StrokeKind::Inside,
    );
    painter.circle_filled(pos2(island_chip_rect.min.x + 7.5, cy), 2.5, sec_col);
    painter.text(
        pos2(island_chip_rect.min.x + 13.5, cy),
        egui::Align2::LEFT_CENTER,
        "Island",
        FontId::proportional(9.5),
        capsule_ink,
    );

    // 2. Bold preset name + glowing cyan pulse dot if active + 2-line tagline
    let text_x = rect.min.x + pad_x;
    let title_y = capsule_rect.max.y + 9.0;
    let title_color = if selected {
        if pal.is_light {
            darken_for_light(accent_col)
        } else {
            theme::mix(accent_col, Color32::WHITE, 0.25)
        }
    } else {
        pal.text
    };
    painter.text(
        pos2(text_x, title_y),
        egui::Align2::LEFT_TOP,
        name,
        Theme::font_semibold(12.5),
        title_color,
    );
    if selected {
        let dot_center = pos2(rect.max.x - pad_x - 5.0, title_y + 7.0);
        let pulse_cyan = Color32::from_rgb(0x38, 0xBD, 0xF8);
        painter.circle_filled(dot_center, 6.0, theme::with_alpha(pulse_cyan, 0.26));
        painter.circle_filled(dot_center, 3.3, pulse_cyan);
    }

    let inner_w = (width - pad_x * 2.0).max(60.0);
    let mut job = egui::epaint::text::LayoutJob::simple(
        tagline.to_string(),
        FontId::proportional(11.0),
        pal.muted,
        inner_w,
    );
    job.wrap.max_rows = 2;
    let tagline_galley = painter.layout_job(job);
    painter.galley(pos2(text_x, title_y + 18.0), tagline_galley, pal.muted);

    // 3. Divider + 3 circular palette swatches + [ACTIVE] / [SELECT] badge
    let div_y = rect.max.y - 30.0;
    painter.hline(
        (rect.min.x + pad_x)..=(rect.max.x - pad_x),
        div_y,
        Stroke::new(1.0, pal.chip_border),
    );

    let bot_y = rect.max.y - 15.0;
    let swatch_ring = if pal.is_light {
        Color32::from_rgba_unmultiplied(15, 23, 42, 70)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 135)
    };
    for (idx, col) in [bg_col, accent_col, sec_col].into_iter().enumerate() {
        let cx = text_x + 6.5 + idx as f32 * 14.0;
        painter.circle(pos2(cx, bot_y), 5.5, col, Stroke::new(1.0, swatch_ring));
    }

    let badge_w = 56.0;
    let badge_rect = Rect::from_min_size(
        pos2(rect.max.x - pad_x - badge_w, bot_y - 9.0),
        vec2(badge_w, 18.0),
    );
    if selected {
        let active_text_col = if pal.is_light {
            darken_for_light(accent_col)
        } else {
            Color32::WHITE
        };
        painter.rect(
            badge_rect,
            CornerRadius::same(9),
            theme::with_alpha(accent_col, 0.24),
            Stroke::new(1.0, accent_col),
            egui::StrokeKind::Inside,
        );
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            "ACTIVE",
            Theme::font_semibold(9.5),
            active_text_col,
        );
    } else {
        painter.rect(
            badge_rect,
            CornerRadius::same(9),
            pal.card_bg,
            Stroke::new(1.0, pal.chip_border),
            egui::StrokeKind::Inside,
        );
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            "SELECT",
            Theme::font_semibold(9.5),
            pal.muted,
        );
    }

    resp.clicked()
}

fn glass_placement_bento_card(
    ui: &mut Ui,
    pal: &Palette,
    system_light: bool,
    os_transparency: bool,
    next_cfg: &mut Config,
) {
    section_card_with_badge(
        ui,
        pal,
        "3D GLASS & PLACEMENT",
        icons::SPARKLE,
        Some("Live Shell"),
        |ui| {
            placement_and_time_row(ui, pal, next_cfg);
            ui.add_space(12.0);

            glass_style_preview_grid(ui, pal, next_cfg);
            ui.add_space(8.0);
            performance_tip_banner(ui, pal, os_transparency);
            ui.add_space(10.0);

            live_split_desktop_preview(ui, pal, system_light, next_cfg);
            ui.add_space(12.0);

            motion_profile_grid(ui, pal, next_cfg);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            behavior_toggles_and_sliders(ui, pal, next_cfg);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            compact_right_slot_grid(ui, pal, next_cfg);
        },
    );
}

fn motion_surface_focused_card(
    ui: &mut Ui,
    pal: &Palette,
    system_light: bool,
    os_transparency: bool,
    next_cfg: &mut Config,
) {
    section_card_with_badge(
        ui,
        pal,
        "3D GLASS, SURFACE & MOTION",
        icons::SPARKLE,
        Some("Shell Optics"),
        |ui| {
            placement_and_time_row(ui, pal, next_cfg);
            ui.add_space(12.0);
            glass_style_preview_grid(ui, pal, next_cfg);
            ui.add_space(8.0);
            performance_tip_banner(ui, pal, os_transparency);
            ui.add_space(10.0);
            live_split_desktop_preview(ui, pal, system_light, next_cfg);
            ui.add_space(12.0);
            motion_profile_grid(ui, pal, next_cfg);
            ui.add_space(10.0);
            toggle_row(
                ui,
                pal,
                "Always on Top",
                "Keep ORBIT anchored above normal desktop windows.",
                &mut next_cfg.on_top,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "Hide in Screen Recording & Share",
                "Apply Windows display affinity so screenshots and recordings skip ORBIT.",
                &mut next_cfg.hide_from_screen_capture,
            );
        },
    );
}

fn behavior_pill_focused_card(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    section_card_with_badge(
        ui,
        pal,
        "BEHAVIOR & COMPACT PILL",
        icons::SLIDERS_HORIZONTAL,
        Some("Interaction"),
        |ui| {
            placement_and_time_row(ui, pal, next_cfg);
            ui.add_space(10.0);
            behavior_toggles_and_sliders(ui, pal, next_cfg);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            compact_right_slot_grid(ui, pal, next_cfg);
        },
    );
}

fn placement_and_time_row(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    let avail = ui.available_width();
    if avail >= 360.0 {
        let gap = 10.0;
        let left_w = ((avail - gap) * 0.56).floor();
        let right_w = (avail - gap - left_w).max(120.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_max_width(left_w);
                if let Some(v) = pill_selector_group(
                    ui,
                    pal,
                    "Screen Position",
                    next_cfg.placement,
                    &[
                        (Placement::Center, "Top Center"),
                        (Placement::Left, "Top Left"),
                        (Placement::Right, "Top Right"),
                    ],
                ) {
                    next_cfg.placement = v;
                }
            });
            ui.allocate_ui_with_layout(vec2(right_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_max_width(right_w);
                if let Some(v) = pill_selector_group(
                    ui,
                    pal,
                    "Time Format",
                    next_cfg.time_format,
                    &[
                        (TimeFormat::Hour12, "12-Hour (AM/PM)"),
                        (TimeFormat::Hour24, "24-Hour"),
                    ],
                ) {
                    next_cfg.time_format = v;
                }
            });
        });
    } else {
        if let Some(v) = pill_selector_group(
            ui,
            pal,
            "Screen Position",
            next_cfg.placement,
            &[
                (Placement::Center, "Top Center"),
                (Placement::Left, "Top Left"),
                (Placement::Right, "Top Right"),
            ],
        ) {
            next_cfg.placement = v;
        }
        ui.add_space(8.0);
        if let Some(v) = pill_selector_group(
            ui,
            pal,
            "Time Format",
            next_cfg.time_format,
            &[
                (TimeFormat::Hour12, "12-Hour (AM/PM)"),
                (TimeFormat::Hour24, "24-Hour"),
            ],
        ) {
            next_cfg.time_format = v;
        }
    }
}

fn glass_style_preview_grid(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    ui.label(
        RichText::new("3D Glass & Frost Style")
            .size(12.5)
            .strong()
            .color(pal.text),
    );
    ui.add(
        egui::Label::new(
            RichText::new("Choose optical backdrop refraction and shell depth.")
                .size(11.5)
                .color(pal.muted),
        )
        .wrap(),
    );
    ui.add_space(6.0);

    const MODES: [(GlassMode, &str, &str); 4] = [
        (GlassMode::Normal, "Normal", "14px crisp glass"),
        (GlassMode::Realistic, "Realistic", "22px deep tint"),
        (GlassMode::Blurred, "Blurred", "34px dense frost"),
        (GlassMode::Opaque, "Opaque", "Solid 3D, zero blur"),
    ];

    let avail = ui.available_width();
    let cols: usize = if avail >= 400.0 { 4 } else { 2 };
    let gap = 8.0;
    let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(100.0);
    let card_h = 92.0;

    for row in MODES.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &(mode, title, desc) in row {
                let selected = next_cfg.glass_mode == mode;
                if glass_style_tile(ui, pal, mode, title, desc, selected, card_w, card_h) {
                    next_cfg.glass_mode = mode;
                }
            }
        });
        ui.add_space(gap);
    }
}

fn glass_style_tile(
    ui: &mut Ui,
    pal: &Palette,
    mode: GlassMode,
    title: &str,
    desc: &str,
    selected: bool,
    width: f32,
    height: f32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let p = ui.painter();
    let fill = if selected {
        pal.accent_soft
    } else if resp.hovered() {
        theme::mix(pal.chip_bg, pal.accent, 0.08)
    } else {
        pal.chip_bg
    };
    let stroke = if selected {
        Stroke::new(1.6, pal.accent)
    } else if resp.hovered() {
        Stroke::new(1.0, theme::with_alpha(pal.accent, 0.6))
    } else {
        Stroke::new(1.0, pal.chip_border)
    };
    p.rect(
        rect,
        CornerRadius::same(10),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    // Desktop gradient thumbnail (height: 34px)
    let thumb = Rect::from_min_size(rect.min + vec2(7.0, 7.0), vec2((width - 14.0).max(50.0), 34.0));
    let (bg_a, bg_b) = match mode {
        GlassMode::Normal => (
            Color32::from_rgb(30, 64, 175),
            Color32::from_rgb(14, 165, 233),
        ),
        GlassMode::Realistic => (
            Color32::from_rgb(67, 56, 202),
            Color32::from_rgb(56, 189, 248),
        ),
        GlassMode::Blurred => (
            Color32::from_rgb(88, 28, 135),
            Color32::from_rgb(14, 116, 144),
        ),
        GlassMode::Opaque => (
            Color32::from_rgb(30, 41, 59),
            Color32::from_rgb(15, 23, 42),
        ),
    };
    p.rect_filled(thumb, CornerRadius::same(7), bg_a);
    let right_half = Rect::from_min_max(pos2(thumb.center().x, thumb.min.y), thumb.max);
    p.rect_filled(
        right_half,
        CornerRadius {
            nw: 0,
            sw: 0,
            ne: 7,
            se: 7,
        },
        theme::with_alpha(bg_b, 0.72),
    );
    p.circle_filled(
        pos2(thumb.min.x + thumb.width() * 0.28, thumb.center().y + 5.0),
        9.0,
        Color32::from_rgba_unmultiplied(244, 114, 182, 85),
    );
    p.circle_filled(
        pos2(thumb.max.x - thumb.width() * 0.24, thumb.center().y - 4.0),
        8.0,
        Color32::from_rgba_unmultiplied(56, 189, 248, 95),
    );

    // Floating pill capsule inside thumbnail
    let cap_w = (thumb.width() * 0.68).clamp(36.0, 74.0);
    let cap_rect = Rect::from_center_size(thumb.center(), vec2(cap_w, 16.0));
    let (cap_fill, cap_rim) = match mode {
        GlassMode::Normal => (
            Color32::from_rgba_unmultiplied(15, 23, 42, 165),
            Color32::from_rgba_unmultiplied(255, 255, 255, 115),
        ),
        GlassMode::Realistic => (
            Color32::from_rgba_unmultiplied(17, 24, 39, 200),
            Color32::from_rgba_unmultiplied(125, 211, 252, 165),
        ),
        GlassMode::Blurred => (
            Color32::from_rgba_unmultiplied(30, 41, 59, 224),
            Color32::from_rgba_unmultiplied(226, 232, 240, 155),
        ),
        GlassMode::Opaque => (
            Color32::from_rgb(10, 14, 23),
            Color32::from_rgba_unmultiplied(255, 255, 255, 85),
        ),
    };
    p.rect(
        cap_rect,
        CornerRadius::same(8),
        cap_fill,
        Stroke::new(1.0, cap_rim),
        egui::StrokeKind::Inside,
    );
    p.circle_filled(
        pos2(cap_rect.min.x + 6.5, cap_rect.center().y),
        2.2,
        Color32::from_rgb(56, 189, 248),
    );
    p.rect_filled(
        Rect::from_min_size(
            pos2(cap_rect.min.x + 12.0, cap_rect.center().y - 1.5),
            vec2((cap_w - 19.0).max(10.0), 3.0),
        ),
        CornerRadius::same(2),
        Color32::from_rgba_unmultiplied(255, 255, 255, 210),
    );

    // Title + glowing cyan dot if selected
    let text_x = rect.min.x + 8.0;
    let title_y = thumb.max.y + 8.0;
    let mut title_x = text_x;
    if selected {
        let dot_c = pos2(text_x + 4.0, title_y + 7.0);
        let cyan = Color32::from_rgb(56, 189, 248);
        p.circle_filled(dot_c, 5.0, theme::with_alpha(cyan, 0.28));
        p.circle_filled(dot_c, 2.8, cyan);
        title_x += 12.0;
    }
    p.text(
        pos2(title_x, title_y),
        egui::Align2::LEFT_TOP,
        title,
        Theme::font_semibold(12.0),
        if selected { pal.accent_text } else { pal.text },
    );
    p.text(
        pos2(text_x, title_y + 17.0),
        egui::Align2::LEFT_TOP,
        desc,
        FontId::proportional(10.5),
        pal.muted,
    );

    resp.clicked()
}

fn performance_tip_banner(ui: &mut Ui, pal: &Palette, os_transparency: bool) {
    Frame::new()
        .fill(pal.accent_soft)
        .stroke(Stroke::new(1.0, theme::with_alpha(pal.accent, 0.42)))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Performance Tip: Opaque uses solid 3D shading with lowest GPU/RAM overhead; Normal, Realistic, and Blurred layer Windows backdrop blur.",
                    )
                    .size(11.3)
                    .color(pal.text),
                )
                .wrap(),
            );
        });

    if !os_transparency {
        ui.add_space(6.0);
        Frame::new()
            .fill(pal.warning_soft)
            .stroke(Stroke::new(1.0, theme::with_alpha(pal.warning, 0.55)))
            .corner_radius(CornerRadius::same(9))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Windows transparency effects are disabled — ORBIT is using tint-only fallback.",
                        )
                        .size(11.3)
                        .color(pal.warning),
                    )
                    .wrap(),
                );
            });
    }
}

fn live_split_desktop_preview(
    ui: &mut Ui,
    pal: &Palette,
    system_light: bool,
    cfg: &Config,
) {
    ui.label(
        RichText::new("Live Island Preview")
            .size(12.5)
            .strong()
            .color(pal.text),
    );
    ui.add_space(4.0);

    let avail = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(avail, 98.0), Sense::hover());
    let p = ui.painter();

    let left_rect = Rect::from_min_max(rect.min, pos2(rect.center().x, rect.max.y));
    let right_rect = Rect::from_min_max(pos2(rect.center().x, rect.min.y), rect.max);

    // Light desktop half
    p.rect_filled(
        left_rect,
        CornerRadius {
            nw: 14,
            sw: 14,
            ne: 0,
            se: 0,
        },
        Color32::from_rgb(0xDC, 0xE9, 0xF9),
    );
    p.circle_filled(
        pos2(left_rect.min.x + left_rect.width() * 0.35, left_rect.max.y - 18.0),
        34.0,
        Color32::from_rgba_unmultiplied(56, 189, 248, 48),
    );

    // Dark desktop half
    p.rect_filled(
        right_rect,
        CornerRadius {
            nw: 0,
            sw: 0,
            ne: 14,
            se: 14,
        },
        Color32::from_rgb(0x0E, 0x15, 0x29),
    );
    p.circle_filled(
        pos2(right_rect.max.x - right_rect.width() * 0.32, right_rect.max.y - 16.0),
        36.0,
        Color32::from_rgba_unmultiplied(139, 92, 246, 52),
    );

    // Center seam + outer card rim
    p.vline(
        rect.center().x,
        (rect.min.y + 6.0)..=(rect.max.y - 6.0),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(148, 163, 184, 85)),
    );
    p.rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, pal.card_border),
        egui::StrokeKind::Inside,
    );

    // Top badges: LIGHT DESKTOP | DARK DESKTOP
    let light_tag = Rect::from_min_size(rect.min + vec2(10.0, 9.0), vec2(92.0, 18.0));
    p.rect(
        light_tag,
        CornerRadius::same(9),
        Color32::from_rgba_unmultiplied(255, 255, 255, 215),
        Stroke::new(0.8, Color32::from_rgba_unmultiplied(15, 23, 42, 36)),
        egui::StrokeKind::Inside,
    );
    p.text(
        light_tag.center(),
        egui::Align2::CENTER_CENTER,
        "LIGHT DESKTOP",
        Theme::font_semibold(9.0),
        Color32::from_rgb(0x0F, 0x17, 0x2A),
    );

    let dark_tag = Rect::from_min_size(
        pos2(rect.max.x - 102.0, rect.min.y + 9.0),
        vec2(92.0, 18.0),
    );
    p.rect(
        dark_tag,
        CornerRadius::same(9),
        Color32::from_rgba_unmultiplied(15, 23, 42, 220),
        Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 48)),
        egui::StrokeKind::Inside,
    );
    p.text(
        dark_tag.center(),
        egui::Align2::CENTER_CENTER,
        "DARK DESKTOP",
        Theme::font_semibold(9.0),
        Color32::WHITE,
    );

    // Centered live 3D ORBIT capsule using active Theme
    let active_theme = Theme::for_config(cfg.theme, cfg.color_theme, system_light);
    let pill_w = 186.0_f32.min((avail - 24.0).max(140.0));
    let pill_rect = Rect::from_center_size(rect.center() + vec2(0.0, 10.0), vec2(pill_w, 36.0));

    // Soft shadow beneath pill
    p.rect_filled(
        pill_rect.translate(vec2(0.0, 3.5)).expand(1.5),
        CornerRadius::same(19),
        Color32::from_rgba_unmultiplied(0, 0, 0, 88),
    );

    let pill_fill = if cfg.glass_mode == GlassMode::Opaque {
        active_theme.opaque_surface
    } else {
        theme::mix(active_theme.pill_top, active_theme.pill_bottom, 0.45)
    };
    let pill_fill = Color32::from_rgb(pill_fill.r(), pill_fill.g(), pill_fill.b());
    p.rect(
        pill_rect,
        CornerRadius::same(18),
        pill_fill,
        Stroke::new(1.2, active_theme.pill_rim),
        egui::StrokeKind::Inside,
    );
    p.hline(
        (pill_rect.min.x + 16.0)..=(pill_rect.max.x - 16.0),
        pill_rect.min.y + 1.5,
        Stroke::new(1.0, active_theme.pill_highlight),
    );

    let py = pill_rect.center().y;
    p.circle_filled(
        pos2(pill_rect.min.x + 15.0, py),
        4.0,
        active_theme.accent,
    );

    match cfg.time_format {
        TimeFormat::Hour12 => {
            p.text(
                pos2(pill_rect.min.x + 26.0, py),
                egui::Align2::LEFT_CENTER,
                "10:08",
                Theme::font_bold(14.5),
                active_theme.pill_text,
            );
            p.text(
                pos2(pill_rect.min.x + 69.0, py + 1.0),
                egui::Align2::LEFT_CENTER,
                "AM",
                Theme::font_semibold(10.0),
                active_theme.accent,
            );
        }
        TimeFormat::Hour24 => {
            p.text(
                pos2(pill_rect.min.x + 28.0, py),
                egui::Align2::LEFT_CENTER,
                "10:08",
                Theme::font_bold(14.5),
                active_theme.pill_text,
            );
        }
    }

    let preview_chip = Rect::from_min_size(
        pos2(pill_rect.max.x - 66.0, py - 11.0),
        vec2(56.0, 22.0),
    );
    p.rect(
        preview_chip,
        CornerRadius::same(11),
        active_theme.pill_btn_bg,
        Stroke::new(1.0, active_theme.pill_btn_border),
        egui::StrokeKind::Inside,
    );
    p.text(
        preview_chip.center(),
        egui::Align2::CENTER_CENTER,
        "Preview",
        Theme::font_semibold(10.5),
        active_theme.pill_text,
    );
}

fn motion_profile_grid(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    if let Some(v) = option_card_grid(
        ui,
        pal,
        "Motion & Physics Profile",
        "Tune spring stiffness, overshoot, or reduced motion.",
        next_cfg.motion_preset,
        &[
            (MotionPreset::LiquidSpring, "Liquid Spring", "Fluid spring"),
            (MotionPreset::Snappy, "Snappy", "Fast & crisp"),
            (MotionPreset::Bouncy, "Bouncy", "Playful bounce"),
            (MotionPreset::Smooth, "Smooth", "Silky ease"),
            (MotionPreset::Reduced, "Reduced", "Minimal motion"),
        ],
    ) {
        next_cfg.motion_preset = v;
    }
}

fn behavior_toggles_and_sliders(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    // Show Weather in Compact Pill + inline °C / °F toggle pills
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let text_w = (ui.available_width() - 82.0).max(120.0);
            ui.set_max_width(text_w);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new("Show Weather in Compact Pill")
                        .size(12.5)
                        .strong()
                        .color(pal.text),
                );
                for (unit, label) in [
                    (WeatherUnit::Celsius, "°C"),
                    (WeatherUnit::Fahrenheit, "°F"),
                ] {
                    let selected = next_cfg.weather_unit == unit;
                    let fill = if selected {
                        pal.accent_soft
                    } else {
                        pal.chip_bg
                    };
                    let stroke = if selected {
                        Stroke::new(1.2, pal.accent)
                    } else {
                        Stroke::new(1.0, pal.chip_border)
                    };
                    let fg = if selected { pal.accent_text } else { pal.muted };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).size(11.0).strong().color(fg))
                                .fill(fill)
                                .stroke(stroke)
                                .corner_radius(CornerRadius::same(7))
                                .min_size(vec2(30.0, 20.0)),
                        )
                        .clicked()
                    {
                        next_cfg.weather_unit = unit;
                    }
                }
            });
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Display local temperature in the compact pill when idle.",
                    )
                    .size(11.5)
                    .color(pal.muted),
                )
                .wrap(),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle_switch(ui, pal, &mut next_cfg.weather_enabled);
        });
    });

    ui.add_space(6.0);
    toggle_row(
        ui,
        pal,
        "Always on Top",
        "Keep ORBIT anchored above normal desktop windows.",
        &mut next_cfg.on_top,
    );

    ui.add_space(6.0);
    let mut click_only = next_cfg.expand_trigger == ExpandTrigger::Click;
    if toggle_row(
        ui,
        pal,
        "Expand on Click Only",
        "Require clicking the compact pill instead of hovering to open the panel.",
        &mut click_only,
    ) {
        next_cfg.expand_trigger = if click_only {
            ExpandTrigger::Click
        } else {
            ExpandTrigger::Hover
        };
    }

    ui.add_space(6.0);
    toggle_row(
        ui,
        pal,
        "Auto-Hide in Fullscreen Video",
        "Automatically hide ORBIT while a fullscreen game or video is active.",
        &mut next_cfg.pause_in_fullscreen,
    );

    ui.add_space(6.0);
    toggle_row(
        ui,
        pal,
        "Launch at Windows Sign-In",
        "Register ORBIT in Windows HKCU Run so it starts when you sign in.",
        &mut next_cfg.launch_at_startup,
    );

    ui.add_space(10.0);
    slider_row(
        ui,
        pal,
        "Hover Open Delay",
        "Time cursor must rest over the compact pill before expanding.",
        &mut next_cfg.hover_delay_ms,
        0..=500,
        "ms",
    );

    ui.add_space(8.0);
    slider_row(
        ui,
        pal,
        "Collapse Grace Period",
        "Delay before collapsing after the pointer leaves the expanded panel.",
        &mut next_cfg.collapse_grace_ms,
        100..=800,
        "ms",
    );
}

fn compact_right_slot_grid(ui: &mut Ui, pal: &Palette, next_cfg: &mut Config) {
    ui.label(
        RichText::new("Compact Right Slot Content")
            .size(12.5)
            .strong()
            .color(pal.text),
    );
    ui.add(
        egui::Label::new(
            RichText::new("Choose what appears on the right side of the idle pill.")
                .size(11.5)
                .color(pal.muted),
        )
        .wrap(),
    );
    ui.add_space(6.0);

    const SLOTS: [(CompactRightSlot, &str, &str); 7] = [
        (CompactRightSlot::Weather, "Weather", "Temp & sky"),
        (CompactRightSlot::Battery, "Battery", "Charge %"),
        (CompactRightSlot::OrbitLogo, "3D ORBIT", "Orbital mark"),
        (CompactRightSlot::WindowsLogo, "Windows", "Win 11 icon"),
        (CompactRightSlot::AppleLogo, "Apple", "Classic mark"),
        (CompactRightSlot::LinuxLogo, "Linux", "Shell mark"),
        (CompactRightSlot::SmartAuto, "Smart Auto", "Context aware"),
    ];

    let avail = ui.available_width();
    let cols: usize = if avail >= 360.0 {
        4
    } else if avail >= 210.0 {
        2
    } else {
        1
    };
    let gap = 6.0;
    let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(96.0);
    let card_h = 56.0;

    for row in SLOTS.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &(slot, title, sub) in row {
                let selected = next_cfg.compact_right_slot == slot;
                if compact_slot_card(ui, pal, slot, title, sub, selected, card_w, card_h) {
                    next_cfg.compact_right_slot = slot;
                }
            }
        });
        ui.add_space(gap);
    }
}

fn compact_slot_card(
    ui: &mut Ui,
    pal: &Palette,
    slot: CompactRightSlot,
    title: &str,
    sub: &str,
    selected: bool,
    width: f32,
    height: f32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let p = ui.painter();
    let fill = if selected {
        pal.accent_soft
    } else if resp.hovered() {
        theme::mix(pal.chip_bg, pal.accent, 0.08)
    } else {
        pal.chip_bg
    };
    let stroke = if selected {
        Stroke::new(1.5, pal.accent)
    } else if resp.hovered() {
        Stroke::new(1.0, theme::with_alpha(pal.accent, 0.55))
    } else {
        Stroke::new(1.0, pal.chip_border)
    };
    p.rect(
        rect,
        CornerRadius::same(9),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    let icon_center = pos2(rect.min.x + 17.0, rect.min.y + 18.0);
    let icon_col = if selected { pal.accent } else { pal.muted };
    paint_slot_icon(p, icon_center, slot, icon_col);

    if selected {
        p.circle_filled(pos2(rect.max.x - 9.0, rect.min.y + 10.0), 3.0, pal.accent);
    }

    p.text(
        pos2(rect.min.x + 30.0, rect.min.y + 10.0),
        egui::Align2::LEFT_TOP,
        title,
        Theme::font_semibold(11.5),
        if selected { pal.accent_text } else { pal.text },
    );
    p.text(
        pos2(rect.min.x + 10.0, rect.min.y + 34.0),
        egui::Align2::LEFT_TOP,
        sub,
        FontId::proportional(10.2),
        pal.muted,
    );

    resp.clicked()
}

fn paint_slot_icon(p: &egui::Painter, c: Pos2, slot: CompactRightSlot, col: Color32) {
    match slot {
        CompactRightSlot::Weather => {
            p.circle_filled(c + vec2(-2.0, -2.0), 3.5, Color32::from_rgb(251, 191, 36));
            p.rect_filled(
                Rect::from_center_size(c + vec2(2.0, 2.5), vec2(9.0, 5.0)),
                CornerRadius::same(3),
                col,
            );
        }
        CompactRightSlot::Battery => {
            let body = Rect::from_center_size(c, vec2(12.0, 7.0));
            p.rect_stroke(
                body,
                CornerRadius::same(2),
                Stroke::new(1.2, col),
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                Rect::from_min_size(body.min + vec2(2.0, 2.0), vec2(6.5, 3.0)),
                CornerRadius::same(1),
                Color32::from_rgb(52, 211, 153),
            );
            p.rect_filled(
                Rect::from_min_size(pos2(body.max.x, c.y - 1.5), vec2(1.5, 3.0)),
                CornerRadius::same(1),
                col,
            );
        }
        CompactRightSlot::OrbitLogo => {
            p.circle_stroke(c, 5.5, Stroke::new(1.3, col));
            p.circle_filled(c, 2.2, col);
        }
        CompactRightSlot::WindowsLogo => {
            for (dx, dy) in [(-3.0, -3.0), (1.0, -3.0), (-3.0, 1.0), (1.0, 1.0)] {
                p.rect_filled(
                    Rect::from_min_size(c + vec2(dx, dy), vec2(3.2, 3.2)),
                    CornerRadius::ZERO,
                    col,
                );
            }
        }
        CompactRightSlot::AppleLogo => {
            p.circle_filled(c + vec2(0.0, 1.2), 4.8, col);
            p.line_segment(
                [c + vec2(0.0, -3.8), c + vec2(2.4, -6.2)],
                Stroke::new(1.5, col),
            );
        }
        CompactRightSlot::LinuxLogo => {
            let box_r = Rect::from_center_size(c, vec2(11.0, 9.0));
            p.rect_stroke(
                box_r,
                CornerRadius::same(2),
                Stroke::new(1.2, col),
                egui::StrokeKind::Inside,
            );
            p.line_segment(
                [c + vec2(-3.0, -1.5), c + vec2(-1.0, 0.5)],
                Stroke::new(1.2, col),
            );
            p.line_segment(
                [c + vec2(-1.0, 0.5), c + vec2(-3.0, 2.5)],
                Stroke::new(1.2, col),
            );
            p.line_segment(
                [c + vec2(0.5, 2.2), c + vec2(3.2, 2.2)],
                Stroke::new(1.2, col),
            );
        }
        CompactRightSlot::SmartAuto => {
            p.line_segment([c + vec2(0.0, -5.5), c + vec2(0.0, 5.5)], Stroke::new(1.4, col));
            p.line_segment([c + vec2(-5.5, 0.0), c + vec2(5.5, 0.0)], Stroke::new(1.4, col));
            p.circle_filled(c, 2.0, col);
        }
    }
}

fn notifications_privacy_card(
    ui: &mut Ui,
    pal: &Palette,
    next_cfg: &mut Config,
    state: &mut SettingsUiState,
    actions: &mut Vec<SettingsAction>,
) {
    section_card_with_badge(
        ui,
        pal,
        "NOTIFICATIONS & PRIVACY",
        icons::BELL,
        Some("Live Feed"),
        |ui| {
            // Windows Listener status badge + Grant Access
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Windows Listener:")
                        .size(12.5)
                        .strong()
                        .color(pal.text),
                );
                access_badge(ui, pal, &state.notif_access_status);

                if state.notif_access_status != "allowed" {
                    let req_btn = egui::Button::new(
                        RichText::new(if state.requesting_notif_access {
                            "Requesting..."
                        } else {
                            "Grant Access"
                        })
                        .size(11.5)
                        .strong()
                        .color(pal.accent_text),
                    )
                    .fill(pal.accent_soft)
                    .stroke(Stroke::new(1.0, pal.accent))
                    .corner_radius(CornerRadius::same(8));
                    if ui
                        .add_enabled(!state.requesting_notif_access, req_btn)
                        .clicked()
                    {
                        state.requesting_notif_access = true;
                        actions.push(SettingsAction::RequestNotificationAccess);
                    }
                }
            });

            ui.add_space(10.0);
            toggle_row(
                ui,
                pal,
                "Enable Island Notifications",
                "Display floating toast bubbles for incoming Windows notifications.",
                &mut next_cfg.notifications_enabled,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "Privacy Mode",
                "Replace notification message text with a generic hidden placeholder.",
                &mut next_cfg.privacy_mode,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "Preview Privacy",
                "Also redact sender identity until you open the notification.",
                &mut next_cfg.preview_privacy,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "Camera, Mic & Location Privacy Light",
                "Show live active camera, microphone, and location indicator dots on the pill.",
                &mut next_cfg.show_privacy_indicators,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "System Utility Alerts (Power, Wi-Fi, BT, Location, Timer)",
                "Surface brief pill alerts when power, wireless, location, or timer states change.",
                &mut next_cfg.utility_alerts_enabled,
            );
            ui.add_space(6.0);
            toggle_row(
                ui,
                pal,
                "Hide in Screen Recording & Share",
                "Apply Windows display affinity so screenshots and recordings skip ORBIT.",
                &mut next_cfg.hide_from_screen_capture,
            );

            ui.add_space(10.0);
            slider_row(
                ui,
                pal,
                "Compact Preview Duration",
                "Seconds before an unhovered notification bubble drains and hides.",
                &mut next_cfg.notification_preview_secs,
                2..=15,
                "sec",
            );

            ui.add_space(6.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Clear Notification Queue")
                            .size(12.0)
                            .color(pal.text),
                    )
                    .fill(pal.chip_bg)
                    .stroke(Stroke::new(1.0, pal.chip_border))
                    .corner_radius(CornerRadius::same(8)),
                )
                .clicked()
            {
                actions.push(SettingsAction::ClearNotifications);
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(
                RichText::new("Blocked Apps")
                    .size(12.5)
                    .strong()
                    .color(pal.text),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Notifications matching these app names or executables are suppressed.",
                    )
                    .size(11.5)
                    .color(pal.muted),
                )
                .wrap(),
            );
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                let desired_w = (ui.available_width() - 68.0).clamp(140.0, 240.0);
                let edit = TextEdit::singleline(&mut state.blocked_app_input)
                    .hint_text("e.g. discord.exe, Slack")
                    .desired_width(desired_w);
                let resp = ui.add(edit);
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let add_clicked = ui
                    .add(
                        egui::Button::new(RichText::new("Add").size(12.0).color(pal.text))
                            .fill(pal.chip_bg)
                            .stroke(Stroke::new(1.0, pal.chip_border))
                            .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked();
                if enter || add_clicked {
                    let trimmed = state.blocked_app_input.trim();
                    if !trimmed.is_empty()
                        && !next_cfg
                            .blocked_apps
                            .iter()
                            .any(|b| b.eq_ignore_ascii_case(trimmed))
                    {
                        next_cfg.blocked_apps.push(trimmed.to_string());
                    }
                    state.blocked_app_input.clear();
                }
            });

            if !next_cfg.blocked_apps.is_empty() {
                ui.add_space(6.0);
                let mut remove_idx: Option<usize> = None;
                ui.horizontal_wrapped(|ui| {
                    for (idx, app) in next_cfg.blocked_apps.iter().enumerate() {
                        Frame::new()
                            .fill(pal.chip_bg)
                            .stroke(Stroke::new(1.0, pal.chip_border))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.label(RichText::new(app).size(12.0).color(pal.text));
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(icons::X).size(11.0).color(pal.muted),
                                            )
                                            .frame(false),
                                        )
                                        .on_hover_text("Unblock app")
                                        .clicked()
                                    {
                                        remove_idx = Some(idx);
                                    }
                                });
                            });
                    }
                });
                if let Some(idx) = remove_idx {
                    next_cfg.blocked_apps.remove(idx);
                }
            }
        },
    );
}

fn memory_usage_card(
    ui: &mut Ui,
    pal: &Palette,
    state: &mut SettingsUiState,
    actions: &mut Vec<SettingsAction>,
) {
    section_card_with_badge(
        ui,
        pal,
        "MEMORY USED BY ORBIT",
        icons::SQUARES_FOUR,
        Some("Diagnostics"),
        |ui| {
            let mem_text = match state.orbit_mem_mb {
                Some(mb) => format!("{mb:.1} MB Working Set"),
                None => "Measuring...".to_string(),
            };
            let (status_label, status_col) = match state.orbit_mem_mb {
                Some(mb) if mb < 120.0 => ("Optimal", pal.success),
                Some(mb) if mb < 220.0 => ("Normal", pal.accent_text),
                Some(_) => ("Elevated", pal.warning),
                None => ("Sampling", pal.muted),
            };

            Frame::new()
                .fill(pal.chip_bg)
                .stroke(Stroke::new(1.0, pal.chip_border))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(mem_text)
                                .size(15.0)
                                .strong()
                                .color(pal.text),
                        );
                        Frame::new()
                            .fill(theme::with_alpha(status_col, 0.16))
                            .corner_radius(CornerRadius::same(99))
                            .inner_margin(Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(status_label)
                                        .size(11.0)
                                        .strong()
                                        .color(status_col),
                                );
                            });
                    });
                });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let refresh_lbl = if state.is_refreshing_mem {
                    "Refreshing..."
                } else {
                    "Refresh"
                };
                if ui
                    .add_enabled(
                        !state.is_refreshing_mem,
                        egui::Button::new(RichText::new(refresh_lbl).size(12.0).color(pal.text))
                            .fill(pal.chip_bg)
                            .stroke(Stroke::new(1.0, pal.chip_border))
                            .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    state.is_refreshing_mem = true;
                    actions.push(SettingsAction::RefreshMemory);
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Trim RAM")
                                .size(12.0)
                                .strong()
                                .color(pal.accent_text),
                        )
                        .fill(pal.accent_soft)
                        .stroke(Stroke::new(1.0, pal.accent))
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    actions.push(SettingsAction::TrimMemoryNow);
                }
            });

            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Shows ORBIT's active Windows working set. Trim RAM compacts heap pages and releases idle backdrop buffers without closing the island.",
                    )
                    .size(11.5)
                    .color(pal.muted),
                )
                .wrap(),
            );
        },
    );
}

fn storage_shelf_about_card(
    ui: &mut Ui,
    pal: &Palette,
    next_cfg: &mut Config,
    state: &mut SettingsUiState,
    actions: &mut Vec<SettingsAction>,
) {
    section_card_with_badge(
        ui,
        pal,
        "STORAGE, SHELF & ABOUT",
        icons::DATABASE,
        Some("Local Only"),
        |ui| {
            let current_copy = next_cfg.shelf_storage_mode == "copy";
            if let Some(copy_mode) = inline_segmented_row(
                ui,
                pal,
                "Shelf Storage Mode",
                "Store lightweight path links or copy dropped files into ORBIT storage.",
                current_copy,
                &[(false, "Path-only"), (true, "Copy-file")],
            ) {
                next_cfg.shelf_storage_mode = if copy_mode {
                    "copy".to_string()
                } else {
                    "reference".to_string()
                };
            }

            ui.add_space(10.0);
            Frame::new()
                .fill(pal.chip_bg)
                .stroke(Stroke::new(1.0, pal.chip_border))
                .corner_radius(CornerRadius::same(9))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "%APPDATA%\\ORBIT stores config.toml, SQLite calendar tasks, and Shelf items. Notifications remain strictly in RAM.",
                            )
                            .size(11.5)
                            .color(pal.muted),
                        )
                        .wrap(),
                    );
                });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Open Config Folder")
                                .size(12.0)
                                .color(pal.text),
                        )
                        .fill(pal.chip_bg)
                        .stroke(Stroke::new(1.0, pal.chip_border))
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    actions.push(SettingsAction::RevealConfigFolder);
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Reset Defaults").size(12.0).color(pal.text),
                        )
                        .fill(pal.chip_bg)
                        .stroke(Stroke::new(1.0, pal.chip_border))
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    state.confirm_clear_data = false;
                    actions.push(SettingsAction::ResetDefaults);
                }

                if !state.confirm_clear_data {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Clear Local Data")
                                    .size(12.0)
                                    .color(pal.danger),
                            )
                            .fill(pal.danger_soft)
                            .stroke(Stroke::new(1.0, theme::with_alpha(pal.danger, 0.5)))
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        state.confirm_clear_data = true;
                    }
                } else {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Confirm Clear?")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(pal.danger)
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        state.confirm_clear_data = false;
                        actions.push(SettingsAction::ClearLocalData);
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Cancel").size(12.0).color(pal.text))
                                .fill(pal.chip_bg)
                                .stroke(Stroke::new(1.0, pal.chip_border))
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        state.confirm_clear_data = false;
                    }
                }
            });

            ui.add_space(8.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(if state.show_privacy_terms {
                            "Hide About & Privacy Policy"
                        } else {
                            "About & Privacy Policy"
                        })
                        .size(12.0)
                        .strong()
                        .color(pal.accent_text),
                    )
                    .frame(false),
                )
                .clicked()
            {
                state.show_privacy_terms = !state.show_privacy_terms;
            }

            if state.show_privacy_terms {
                ui.add_space(4.0);
                Frame::new()
                    .fill(pal.chip_bg)
                    .stroke(Stroke::new(1.0, pal.chip_border))
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                RichText::new(concat!(
                                    "ORBIT v",
                                    env!("CARGO_PKG_VERSION"),
                                    " — Zero telemetry. No cloud accounts. Notification bodies, media titles, and camera/mic status are kept strictly in RAM and never written to disk."
                                ))
                                .size(11.5)
                                .color(pal.muted),
                            )
                            .wrap(),
                        );
                    });
            }
        },
    );
}

fn apply_local_visuals(ui: &mut Ui, pal: &Palette) {
    let mut visuals = if pal.is_light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.panel_fill = pal.canvas_bg;
    visuals.window_fill = pal.card_bg;
    visuals.extreme_bg_color = pal.input_bg;
    visuals.faint_bg_color = pal.chip_bg;
    visuals.override_text_color = Some(pal.text);
    visuals.selection.bg_fill = pal.accent;
    visuals.selection.stroke = Stroke::new(1.0, pal.text);

    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text);
    visuals.widgets.noninteractive.bg_fill = pal.card_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.card_border);

    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text);
    visuals.widgets.inactive.bg_fill = pal.chip_bg;
    visuals.widgets.inactive.weak_bg_fill = pal.chip_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.chip_border);

    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, pal.text);
    visuals.widgets.hovered.bg_fill = theme::mix(pal.chip_bg, pal.accent, 0.14);
    visuals.widgets.hovered.weak_bg_fill = theme::mix(pal.chip_bg, pal.accent, 0.14);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.accent);

    visuals.widgets.active.fg_stroke = Stroke::new(1.0, pal.text);
    visuals.widgets.active.bg_fill = pal.accent_soft;
    visuals.widgets.active.weak_bg_fill = pal.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.25, pal.accent);

    let style = ui.style_mut();
    style.visuals = visuals;
    style.spacing.item_spacing = vec2(8.0, 6.0);
}

fn header_bar(
    ui: &mut Ui,
    pal: &Palette,
    next_cfg: &mut Config,
    logo_tex: Option<egui::TextureId>,
    actions: &mut Vec<SettingsAction>,
) {
    let avail = ui.available_width();
    let compact = avail < 640.0;

    let frame_resp = Frame::new()
        .fill(pal.card_bg)
        .stroke(Stroke::new(1.0, pal.card_border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                paint_orbit_logo(ui, pal, logo_tex);

                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("ORBIT Settings")
                                .size(16.5)
                                .strong()
                                .color(pal.text),
                        );
                        Frame::new()
                            .fill(pal.accent_soft)
                            .stroke(Stroke::new(1.0, theme::with_alpha(pal.accent, 0.48)))
                            .corner_radius(CornerRadius::same(99))
                            .inner_margin(Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                        .size(10.5)
                                        .strong()
                                        .color(pal.accent_text),
                                );
                            });
                    });
                    ui.add_space(2.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "3D Orbital Shell, Motion Profiles & System Diagnostics",
                            )
                            .size(11.5)
                            .color(pal.muted),
                        )
                        .wrap(),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!("{} Close", icons::X))
                                    .size(12.0)
                                    .strong()
                                    .color(pal.text),
                            )
                            .fill(pal.chip_bg)
                            .stroke(Stroke::new(1.0, pal.chip_border))
                            .corner_radius(CornerRadius::same(9)),
                        )
                        .clicked()
                    {
                        actions.push(SettingsAction::Close);
                    }

                    if !compact {
                        ui.add_space(6.0);
                        appearance_selector(ui, pal, &mut next_cfg.settings_appearance);
                    }
                });
            });

            if compact {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Appearance:")
                            .size(11.5)
                            .strong()
                            .color(pal.muted),
                    );
                    appearance_selector(ui, pal, &mut next_cfg.settings_appearance);
                });
            }
        });

    let r = frame_resp.response.rect;
    ui.painter().hline(
        (r.min.x + 14.0)..=(r.max.x - 14.0),
        r.min.y + 1.0,
        Stroke::new(1.0, pal.card_highlight),
    );
}

fn paint_orbit_logo(ui: &mut Ui, pal: &Palette, logo_tex: Option<egui::TextureId>) {
    let (logo_r, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
    if let Some(tex) = logo_tex {
        crate::paint::rounded_texture(
            ui.painter(),
            logo_r,
            12.0,
            tex,
            1.0,
            pal.card_border,
        );
    } else {
        let p = ui.painter();
        let center = logo_r.center();
        p.circle(
            center,
            18.0,
            pal.accent_soft,
            Stroke::new(1.25, theme::with_alpha(pal.accent, 0.78)),
        );
        p.circle_stroke(
            center,
            11.5,
            Stroke::new(1.1, theme::with_alpha(pal.accent, 0.55)),
        );
        p.circle_filled(center, 4.5, pal.accent);
    }
}

fn appearance_selector(ui: &mut Ui, pal: &Palette, current: &mut SettingsAppearance) {
    Frame::new()
        .fill(pal.chip_bg)
        .stroke(Stroke::new(1.0, pal.chip_border))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for (val, label) in [
                    (SettingsAppearance::System, "System"),
                    (SettingsAppearance::Dark, "Dark"),
                    (SettingsAppearance::Light, "Light"),
                    (SettingsAppearance::Orbit, "ORBIT"),
                ] {
                    let selected = *current == val;
                    let fill = if selected {
                        pal.accent_soft
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if selected {
                        Stroke::new(1.0, pal.accent)
                    } else {
                        Stroke::NONE
                    };
                    let txt_col = if selected { pal.accent_text } else { pal.muted };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(label).size(11.3).strong().color(txt_col),
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::same(7))
                            .min_size(vec2(0.0, 22.0)),
                        )
                        .clicked()
                    {
                        *current = val;
                    }
                }
            });
        });
}

fn category_filter_bar(ui: &mut Ui, pal: &Palette, active: &mut SettingsCategory) {
    Frame::new()
        .fill(pal.card_bg)
        .stroke(Stroke::new(1.0, pal.card_border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for (cat, label) in SettingsCategory::ALL_TABS {
                    let selected = *active == cat;
                    let fill = if selected {
                        pal.accent_soft
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_col = if selected { pal.accent_text } else { pal.muted };
                    let text = if selected {
                        RichText::new(label).size(12.0).strong().color(text_col)
                    } else {
                        RichText::new(label).size(12.0).color(text_col)
                    };
                    let btn = egui::Button::new(text)
                        .fill(fill)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(99))
                        .min_size(vec2(0.0, 26.0));
                    if ui.add(btn).clicked() {
                        *active = cat;
                    }
                }
            });
        });
}

fn status_banner(ui: &mut Ui, pal: &Palette, msg: &str, is_err: bool) {
    let (bg, fg) = if is_err {
        (pal.danger_soft, pal.danger)
    } else {
        (pal.success_soft, pal.success)
    };
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, fg.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(msg).size(12.0).strong().color(fg)).wrap());
        });
}

fn section_card_with_badge(
    ui: &mut Ui,
    pal: &Palette,
    title: &str,
    icon_glyph: &str,
    badge: Option<&str>,
    add_contents: impl FnOnce(&mut Ui),
) {
    let resp = Frame::new()
        .fill(pal.card_bg)
        .stroke(Stroke::new(1.0, pal.card_border))
        .corner_radius(CornerRadius::same(13))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(icon_glyph)
                        .font(crate::pill::icon_font(15.0, false))
                        .color(pal.accent),
                );
                ui.label(RichText::new(title).size(13.0).strong().color(pal.text));
                if let Some(badge_text) = badge {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Frame::new()
                            .fill(pal.accent_soft)
                            .stroke(Stroke::new(1.0, theme::with_alpha(pal.accent, 0.35)))
                            .corner_radius(CornerRadius::same(99))
                            .inner_margin(Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(badge_text)
                                        .size(10.5)
                                        .strong()
                                        .color(pal.accent_text),
                                );
                            });
                    });
                }
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(7.0);
            add_contents(ui);
        });

    let r = resp.response.rect;
    ui.painter().hline(
        (r.min.x + 14.0)..=(r.max.x - 14.0),
        r.min.y + 1.0,
        Stroke::new(0.9, pal.card_highlight),
    );
}

fn option_card_grid<T: Copy + PartialEq>(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    hint: &str,
    current: T,
    options: &[(T, &str, &str)],
) -> Option<T> {
    let mut picked = None;
    ui.label(RichText::new(label).size(12.5).strong().color(pal.text));
    ui.add(egui::Label::new(RichText::new(hint).size(11.5).color(pal.muted)).wrap());
    ui.add_space(6.0);

    let avail = ui.available_width();
    let cols: usize = if avail >= 420.0 {
        options.len().min(3)
    } else if avail >= 220.0 {
        2
    } else {
        1
    };
    let gap = 6.0;
    let card_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).max(96.0);
    let card_h = 48.0;

    for row in options.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &(val, title, sub) in row {
                let selected = val == current;
                let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::click());
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                let fill = if selected {
                    pal.accent_soft
                } else if resp.hovered() {
                    theme::mix(pal.chip_bg, pal.accent, 0.08)
                } else {
                    pal.chip_bg
                };
                let stroke = if selected {
                    Stroke::new(1.5, pal.accent)
                } else if resp.hovered() {
                    Stroke::new(1.0, theme::with_alpha(pal.accent, 0.55))
                } else {
                    Stroke::new(1.0, pal.chip_border)
                };
                let p = ui.painter();
                p.rect(
                    rect,
                    CornerRadius::same(9),
                    fill,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                p.text(
                    pos2(rect.min.x + 10.0, rect.min.y + 8.0),
                    egui::Align2::LEFT_TOP,
                    title,
                    Theme::font_semibold(12.0),
                    if selected { pal.accent_text } else { pal.text },
                );
                p.text(
                    pos2(rect.min.x + 10.0, rect.min.y + 26.0),
                    egui::Align2::LEFT_TOP,
                    sub,
                    FontId::proportional(10.5),
                    pal.muted,
                );
                if resp.clicked() && !selected {
                    picked = Some(val);
                }
            }
        });
        ui.add_space(gap);
    }

    picked
}

fn pill_selector_group<T: Copy + PartialEq>(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut picked = None;
    ui.label(RichText::new(label).size(12.2).strong().color(pal.text));
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
        for &(val, text) in options {
            let selected = val == current;
            let fill = if selected {
                pal.accent_soft
            } else {
                pal.chip_bg
            };
            let stroke = if selected {
                Stroke::new(1.25, pal.accent)
            } else {
                Stroke::new(1.0, pal.chip_border)
            };
            let txt_col = if selected { pal.accent_text } else { pal.text };
            let btn = egui::Button::new(RichText::new(text).size(11.5).strong().color(txt_col))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(0.0, 26.0));
            if ui.add(btn).clicked() && !selected {
                picked = Some(val);
            }
        }
    });
    picked
}

fn inline_segmented_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    hint: &str,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut picked = None;
    ui.label(RichText::new(label).size(12.5).strong().color(pal.text));
    ui.add(egui::Label::new(RichText::new(hint).size(11.5).color(pal.muted)).wrap());
    ui.add_space(5.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for &(val, text) in options {
            let selected = val == current;
            let fill = if selected {
                pal.accent_soft
            } else {
                pal.chip_bg
            };
            let stroke = if selected {
                Stroke::new(1.25, pal.accent)
            } else {
                Stroke::new(1.0, pal.chip_border)
            };
            let txt_col = if selected { pal.accent_text } else { pal.text };
            let btn = egui::Button::new(RichText::new(text).size(12.0).strong().color(txt_col))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(66.0, 26.0));
            if ui.add(btn).clicked() && !selected {
                picked = Some(val);
            }
        }
    });
    picked
}

fn toggle_row(ui: &mut Ui, pal: &Palette, label: &str, hint: &str, value: &mut bool) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let text_w = (ui.available_width() - 82.0).max(120.0);
            ui.set_max_width(text_w);
            ui.label(RichText::new(label).size(12.5).strong().color(pal.text));
            ui.add(egui::Label::new(RichText::new(hint).size(11.5).color(pal.muted)).wrap());
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle_switch(ui, pal, value);
        });
    });
    *value != before
}

fn toggle_switch(ui: &mut Ui, pal: &Palette, on: &mut bool) {
    let switch_size = vec2(40.0, 22.0);
    let (bounds, resp) = ui.allocate_exact_size(vec2(72.0, 24.0), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if resp.clicked() {
        *on = !*on;
    }
    let anim_t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = if *on {
        pal.accent
    } else if pal.is_light {
        Color32::from_rgb(148, 163, 184)
    } else {
        Color32::from_rgb(51, 65, 85)
    };
    let rect = Rect::from_min_size(pos2(bounds.min.x, bounds.center().y - 11.0), switch_size);
    ui.painter().rect_filled(rect, 11.0, bg);
    let knob_x = egui::lerp((rect.min.x + 11.0)..=(rect.max.x - 11.0), anim_t);
    ui.painter()
        .circle_filled(pos2(knob_x, rect.center().y), 8.0, Color32::WHITE);
    let state_color = if *on { pal.accent_text } else { pal.muted };
    ui.painter().text(
        pos2(rect.max.x + 6.0, bounds.center().y),
        egui::Align2::LEFT_CENTER,
        if *on { "On" } else { "Off" },
        Theme::font_semibold(10.8),
        state_color,
    );
}

fn slider_row(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    hint: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    suffix: &str,
) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).strong().color(pal.text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{value} {suffix}"))
                    .size(12.0)
                    .strong()
                    .color(pal.accent_text),
            );
        });
    });
    ui.add(egui::Label::new(RichText::new(hint).size(11.5).color(pal.muted)).wrap());
    ui.add_space(2.0);
    ui.add(Slider::new(value, range).show_value(false));
    *value != before
}

fn access_badge(ui: &mut Ui, pal: &Palette, status: &str) {
    let (label, color) = match status {
        "allowed" => ("Allowed", pal.success),
        "denied" => ("Denied", pal.danger),
        "unsupported" => ("Unsupported", pal.muted),
        "checking..." => ("Checking...", pal.muted),
        _ => ("Unspecified", pal.warning),
    };
    Frame::new()
        .fill(theme::with_alpha(color, 0.18))
        .stroke(Stroke::new(1.0, theme::with_alpha(color, 0.45)))
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.5).strong().color(color));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::FALLBACK_ACCENT;

    #[test]
    fn settings_state_defaults_and_theme_mode_resolution() {
        let state = SettingsUiState::default();
        assert!(state.blocked_app_input.is_empty());
        assert!(!state.confirm_clear_data);
        assert_eq!(state.notif_access_status, "checking...");
        assert_eq!(state.active_category, SettingsCategory::All);
        assert!(state.orbit_mem_mb.is_none());
        assert!(!state.is_refreshing_mem);

        assert!(Theme::is_light_effective(ThemeMode::Light, false));
        assert!(!Theme::is_light_effective(ThemeMode::Dark, true));
        assert!(Theme::is_light_effective(ThemeMode::Auto, true));
        assert!(!Theme::is_light_effective(ThemeMode::Auto, false));

        let light_theme = Theme::for_mode(ThemeMode::Auto, true, FALLBACK_ACCENT);
        let dark_theme = Theme::for_mode(ThemeMode::Auto, false, FALLBACK_ACCENT);
        assert_ne!(light_theme.pill_top, dark_theme.pill_top);
        assert_ne!(light_theme.pill_text, dark_theme.pill_text);

        let dark_cfg = Config {
            color_theme: ColorTheme::Cyberpunk,
            theme: ThemeMode::Dark,
            settings_appearance: SettingsAppearance::Orbit,
            ..Config::default()
        };
        let light_cfg = Config {
            color_theme: ColorTheme::CeramicPearl,
            theme: ThemeMode::Light,
            settings_appearance: SettingsAppearance::Orbit,
            ..Config::default()
        };
        assert_ne!(
            canvas_color_for(&dark_cfg, false),
            canvas_color_for(&light_cfg, false)
        );

        let studio_dark = Config {
            settings_appearance: SettingsAppearance::Dark,
            ..Config::default()
        };
        let studio_light = Config {
            settings_appearance: SettingsAppearance::Light,
            ..Config::default()
        };
        let studio_sys = Config {
            settings_appearance: SettingsAppearance::System,
            ..Config::default()
        };
        assert_eq!(
            canvas_color_for(&studio_dark, true),
            Color32::from_rgb(0x09, 0x0D, 0x16)
        );
        assert_eq!(
            canvas_color_for(&studio_light, false),
            Color32::from_rgb(0xF1, 0xF5, 0xF9)
        );
        assert_eq!(
            canvas_color_for(&studio_sys, true),
            canvas_color_for(&studio_light, true)
        );
        assert_eq!(
            canvas_color_for(&studio_sys, false),
            canvas_color_for(&studio_dark, false)
        );
    }

    #[test]
    fn settings_renders_and_drops_cleanly_at_multiple_widths() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(
            &ctx,
            crate::theme::FontAssets {
                regular: None,
                semibold: None,
                bold: None,
            },
        );
        let cfg = Config::default();
        for width in [460.0_f32, 640.0, 960.0] {
            let mut state = SettingsUiState {
                orbit_mem_mb: Some(98.4),
                ..SettingsUiState::default()
            };
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(width, 680.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let actions = show_settings(ui, &cfg, false, true, None, &mut state);
                assert!(actions.is_empty());
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn settings_category_filtering_renders_without_panics_across_widths() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(
            &ctx,
            crate::theme::FontAssets {
                regular: None,
                semibold: None,
                bold: None,
            },
        );
        let cfg = Config {
            color_theme: ColorTheme::LinearIndigo,
            motion_preset: MotionPreset::Bouncy,
            ..Config::default()
        };
        let categories = [
            SettingsCategory::All,
            SettingsCategory::Themes,
            SettingsCategory::MotionSurface,
            SettingsCategory::BehaviorPill,
            SettingsCategory::PrivacyNotifications,
            SettingsCategory::Data,
        ];
        for width in [460.0_f32, 640.0, 960.0] {
            for category in categories {
                let mut state = SettingsUiState {
                    active_category: category,
                    ..SettingsUiState::default()
                };
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        vec2(width, 680.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let actions = show_settings(ui, &cfg, false, false, None, &mut state);
                    assert!(actions.is_empty());
                });
                out.textures_delta.clear();
            }
        }
    }

    #[test]
    fn settings_category_filter_click_updates_active_category() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(
            &ctx,
            crate::theme::FontAssets {
                regular: None,
                semibold: None,
                bold: None,
            },
        );
        let cfg = Config::default();
        let mut state = SettingsUiState::default();
        let mut themes_chip_center = None;

        // First frame: measure position of "Themes" chip
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(640.0, 680.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            let _ = show_settings(ui, &cfg, false, true, None, &mut state);
        });
        for clipped in &out.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                if t.galley.text() == "Themes" {
                    themes_chip_center = Some(t.pos + t.galley.size() * 0.5);
                    break;
                }
            }
        }
        out.textures_delta.clear();

        let click_pos = themes_chip_center.expect("Themes filter chip rendered");
        let raw_down = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(640.0, 680.0),
            )),
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut out2 = ctx.run_ui(raw_down, |ui| {
            let _ = show_settings(ui, &cfg, false, true, None, &mut state);
        });
        out2.textures_delta.clear();

        let raw_up = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(640.0, 680.0),
            )),
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut out3 = ctx.run_ui(raw_up, |ui| {
            let actions = show_settings(ui, &cfg, false, true, None, &mut state);
            assert!(actions.is_empty());
        });
        out3.textures_delta.clear();

        assert_eq!(state.active_category, SettingsCategory::Themes);
    }

    #[test]
    fn settings_theme_swatch_click_updates_theme_and_mode() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(
            &ctx,
            crate::theme::FontAssets {
                regular: None,
                semibold: None,
                bold: None,
            },
        );

        let click_label = |cfg: &Config, state: &mut SettingsUiState, title: &str| -> Vec<SettingsAction> {
            let screen = vec2(960.0, 900.0);
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
                ..Default::default()
            };
            let mut target_pos = None;
            let mut out = ctx.run_ui(raw, |ui| {
                let _ = show_settings(ui, cfg, false, true, None, state);
            });
            for clipped in &out.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    if t.galley.text() == title {
                        target_pos = Some(t.pos + t.galley.size() * 0.5);
                        break;
                    }
                }
            }
            out.textures_delta.clear();
            let pos = target_pos.unwrap_or_else(|| panic!("label '{title}' not rendered"));

            let raw_down = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            };
            let mut out2 = ctx.run_ui(raw_down, |ui| {
                let _ = show_settings(ui, cfg, false, true, None, state);
            });
            out2.textures_delta.clear();

            let mut emitted = Vec::new();
            let raw_up = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen)),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            };
            let mut out3 = ctx.run_ui(raw_up, |ui| {
                emitted = show_settings(ui, cfg, false, true, None, state);
            });
            out3.textures_delta.clear();
            emitted
        };

        // 1. Clicking a dark preset while in Light mode switches to Dark mode immediately
        let mut state = SettingsUiState {
            active_category: SettingsCategory::Themes,
            ..SettingsUiState::default()
        };
        let light_cfg = Config {
            theme: ThemeMode::Light,
            color_theme: ColorTheme::CeramicPearl,
            ..Config::default()
        };
        let actions = click_label(&light_cfg, &mut state, "Cyberpunk");
        assert_eq!(
            actions,
            vec![SettingsAction::ConfigChanged(Config {
                theme: ThemeMode::Dark,
                color_theme: ColorTheme::Cyberpunk,
                ..light_cfg
            })]
        );

        // 2. Clicking Ceramic Pearl activates Light mode + CeramicPearl
        let dark_cfg = Config {
            theme: ThemeMode::Dark,
            color_theme: ColorTheme::Obsidian,
            ..Config::default()
        };
        let actions2 = click_label(&dark_cfg, &mut state, "Ceramic Pearl");
        assert_eq!(
            actions2,
            vec![SettingsAction::ConfigChanged(Config {
                theme: ThemeMode::Light,
                color_theme: ColorTheme::CeramicPearl,
                ..dark_cfg
            })]
        );
    }
}
