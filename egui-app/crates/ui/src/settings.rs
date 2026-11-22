//! Gate 6 Settings UI: pure immediate-mode Settings panels rendered inside the
//! on-demand second viewport (`orbit-settings`). No Win32 calls or disk I/O live here.

use crate::icons;
use crate::theme::Theme;
use egui::{
    Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Slider,
    Stroke, TextEdit, Ui, Visuals, vec2,
};
use orbit_core::config::{Config, ExpandTrigger, GlassMode, Placement, ThemeMode, WeatherUnit};

/// Transient UI-only state owned by the Settings window while open and dropped on close.
#[derive(Clone, Debug)]
pub struct SettingsUiState {
    pub blocked_app_input: String,
    pub confirm_clear_data: bool,
    pub status_banner: Option<(String, bool)>,
    pub notif_access_status: String,
    pub requesting_notif_access: bool,
    pub show_privacy_terms: bool,
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
    Close,
}

#[derive(Clone, Copy)]
struct Palette {
    is_light: bool,
    canvas_bg: Color32,
    card_bg: Color32,
    card_border: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    accent_soft: Color32,
    chip_bg: Color32,
    chip_border: Color32,
    danger: Color32,
    danger_soft: Color32,
    success: Color32,
    success_soft: Color32,
    warning: Color32,
}

impl Palette {
    fn from_mode(theme_mode: ThemeMode, system_light: bool) -> Self {
        let is_light = Theme::is_light_effective(theme_mode, system_light);
        if is_light {
            Self {
                is_light: true,
                canvas_bg: Color32::from_rgb(241, 245, 249),
                card_bg: Color32::from_rgb(255, 255, 255),
                card_border: Color32::from_rgb(220, 226, 235),
                text: Color32::from_rgb(15, 23, 42),
                muted: Color32::from_rgb(82, 95, 114),
                accent: Color32::from_rgb(24, 114, 238),
                accent_soft: Color32::from_rgba_unmultiplied(24, 114, 238, 28),
                chip_bg: Color32::from_rgb(241, 245, 249),
                chip_border: Color32::from_rgb(203, 213, 225),
                danger: Color32::from_rgb(220, 38, 38),
                danger_soft: Color32::from_rgba_unmultiplied(220, 38, 38, 24),
                success: Color32::from_rgb(5, 150, 105),
                success_soft: Color32::from_rgba_unmultiplied(5, 150, 105, 24),
                warning: Color32::from_rgb(217, 119, 6),
            }
        } else {
            Self {
                is_light: false,
                canvas_bg: Color32::from_rgb(12, 15, 22),
                card_bg: Color32::from_rgb(20, 25, 36),
                card_border: Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                text: Color32::from_rgb(241, 245, 249),
                muted: Color32::from_rgb(152, 163, 184),
                accent: Color32::from_rgb(56, 152, 255),
                accent_soft: Color32::from_rgba_unmultiplied(56, 152, 255, 42),
                chip_bg: Color32::from_rgb(28, 35, 50),
                chip_border: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
                danger: Color32::from_rgb(248, 113, 113),
                danger_soft: Color32::from_rgba_unmultiplied(248, 113, 113, 30),
                success: Color32::from_rgb(52, 211, 153),
                success_soft: Color32::from_rgba_unmultiplied(52, 211, 153, 30),
                warning: Color32::from_rgb(251, 191, 36),
            }
        }
    }
}

pub fn canvas_color(theme_mode: ThemeMode, system_light: bool) -> Color32 {
    Palette::from_mode(theme_mode, system_light).canvas_bg
}

/// Renders the complete ORBIT Settings interface inside a native viewport's `CentralPanel`.
pub fn show_settings(
    ui: &mut Ui,
    cfg: &Config,
    system_light: bool,
    os_transparency: bool,
    logo_tex: Option<egui::TextureId>,
    state: &mut SettingsUiState,
) -> Vec<SettingsAction> {
    let pal = Palette::from_mode(cfg.theme, system_light);
    apply_local_visuals(ui, &pal);

    let mut next_cfg = cfg.clone();
    let mut actions = Vec::new();

    // Pinned top header
    header_bar(ui, &pal, logo_tex, state, &mut actions);
    ui.add_space(10.0);

    // Optional feedback banner
    if let Some((msg, is_err)) = state.status_banner.clone() {
        status_banner(ui, &pal, &msg, is_err);
        ui.add_space(10.0);
    }

    ScrollArea::vertical()
        .id_salt("orbit-settings-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // 1. Appearance & Placement
            section_card(ui, &pal, "Appearance & Placement", icons::PALETTE, |ui| {
                if let Some(v) = segmented_row(
                    ui,
                    &pal,
                    "Theme",
                    "Follow Windows light/dark or lock a fixed palette.",
                    next_cfg.theme,
                    &[
                        (ThemeMode::Auto, "Auto"),
                        (ThemeMode::Dark, "Dark"),
                        (ThemeMode::Light, "Light"),
                    ],
                ) {
                    next_cfg.theme = v;
                }

                ui.add_space(8.0);
                if let Some(v) = segmented_row(
                    ui,
                    &pal,
                    "Island Placement",
                    "Horizontal dock edge on the primary display.",
                    next_cfg.placement,
                    &[
                        (Placement::Left, "Left"),
                        (Placement::Center, "Center"),
                        (Placement::Right, "Right"),
                    ],
                ) {
                    next_cfg.placement = v;
                }

                ui.add_space(8.0);
                if let Some(v) = segmented_row(
                    ui,
                    &pal,
                    "Surface Style",
                    "Live Windows.UI.Composition blur behind ORBIT (Opaque disables blur).",
                    next_cfg.glass_mode,
                    &[
                        (GlassMode::Normal, "Normal"),
                        (GlassMode::Realistic, "Realistic"),
                        (GlassMode::Blurred, "Blurred"),
                        (GlassMode::Opaque, "Opaque"),
                    ],
                ) {
                    next_cfg.glass_mode = v;
                }
                if !os_transparency {
                    ui.add_space(4.0);
                    Frame::new()
                        .fill(pal.warning.gamma_multiply(0.16))
                        .stroke(Stroke::new(1.0, pal.warning.gamma_multiply(0.45)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "Windows transparency effects are turned off — using tint-only fallback.",
                                    )
                                    .size(11.5)
                                    .color(pal.warning),
                                )
                                .wrap(),
                            );
                        });
                }

                ui.add_space(8.0);
                toggle_row(
                    ui,
                    &pal,
                    "Always on Top",
                    "Keep ORBIT anchored above normal desktop windows.",
                    &mut next_cfg.on_top,
                );

                ui.add_space(6.0);
                toggle_row(
                    ui,
                    &pal,
                    "Hide from Screen Capture",
                    "Apply Windows display affinity so screenshots and recordings skip ORBIT.",
                    &mut next_cfg.hide_from_screen_capture,
                );
            });

            ui.add_space(14.0);

            // 2. Behavior & Startup
            section_card(ui, &pal, "Behavior & Startup", icons::SLIDERS_HORIZONTAL, |ui| {
                if let Some(v) = segmented_row(
                    ui,
                    &pal,
                    "Expand Trigger",
                    "Open the expanded panel by hovering the pill or only on click.",
                    next_cfg.expand_trigger,
                    &[
                        (ExpandTrigger::Hover, "Hover"),
                        (ExpandTrigger::Click, "Click Only"),
                    ],
                ) {
                    next_cfg.expand_trigger = v;
                }

                ui.add_space(8.0);
                slider_row(
                    ui,
                    &pal,
                    "Hover Open Delay",
                    "Time cursor must rest over the compact pill before expanding.",
                    &mut next_cfg.hover_delay_ms,
                    0..=500,
                    "ms",
                );

                ui.add_space(8.0);
                slider_row(
                    ui,
                    &pal,
                    "Collapse Grace Period",
                    "Delay before collapsing after the pointer leaves the expanded panel.",
                    &mut next_cfg.collapse_grace_ms,
                    100..=800,
                    "ms",
                );

                ui.add_space(8.0);
                toggle_row(
                    ui,
                    &pal,
                    "Pause in Fullscreen",
                    "Automatically hide ORBIT while a fullscreen game or video is active.",
                    &mut next_cfg.pause_in_fullscreen,
                );

                ui.add_space(6.0);
                toggle_row(
                    ui,
                    &pal,
                    "Launch at Windows Startup",
                    "Register ORBIT in Windows HKCU Run so it starts when you sign in.",
                    &mut next_cfg.launch_at_startup,
                );
            });

            ui.add_space(14.0);

            // 3. Weather & Shelf
            section_card(ui, &pal, "Weather & File Shelf", icons::CLOUD_SUN, |ui| {
                toggle_row(
                    ui,
                    &pal,
                    "Enable Weather Pill Slot",
                    "Show local temperature in the compact pill when no focus timer is running.",
                    &mut next_cfg.weather_enabled,
                );

                ui.add_space(8.0);
                if let Some(v) = segmented_row(
                    ui,
                    &pal,
                    "Temperature Unit",
                    "Display compact weather in Celsius or Fahrenheit.",
                    next_cfg.weather_unit,
                    &[
                        (WeatherUnit::Celsius, "°C  Celsius"),
                        (WeatherUnit::Fahrenheit, "°F  Fahrenheit"),
                    ],
                ) {
                    next_cfg.weather_unit = v;
                }

                ui.add_space(8.0);
                let current_copy = next_cfg.shelf_storage_mode == "copy";
                if let Some(copy_mode) = segmented_row(
                    ui,
                    &pal,
                    "Shelf Storage Mode",
                    "Store lightweight path references or copy dropped files into ORBIT storage.",
                    current_copy,
                    &[(false, "Path-only"), (true, "Copy-file")],
                ) {
                    next_cfg.shelf_storage_mode = if copy_mode {
                        "copy".to_string()
                    } else {
                        "reference".to_string()
                    };
                }
            });

            ui.add_space(14.0);

            // 4. Notifications & Privacy
            section_card(ui, &pal, "Notifications & Privacy", icons::BELL, |ui| {
                toggle_row(
                    ui,
                    &pal,
                    "Enable Notification Bubbles",
                    "Display floating toast previews for incoming Windows notifications.",
                    &mut next_cfg.notifications_enabled,
                );

                ui.add_space(8.0);
                slider_row(
                    ui,
                    &pal,
                    "Bubble Preview Duration",
                    "Seconds before an unhovered notification bubble drains and hides.",
                    &mut next_cfg.notification_preview_secs,
                    2..=15,
                    "s",
                );

                ui.add_space(8.0);
                toggle_row(
                    ui,
                    &pal,
                    "Privacy Mode (Mask Content)",
                    "Replace notification message text with a generic hidden placeholder.",
                    &mut next_cfg.privacy_mode,
                );

                ui.add_space(6.0);
                toggle_row(
                    ui,
                    &pal,
                    "Preview Privacy",
                    "Also redact sender identity until you open the notification.",
                    &mut next_cfg.preview_privacy,
                );

                ui.add_space(6.0);
                toggle_row(
                    ui,
                    &pal,
                    "Camera & Microphone Indicators",
                    "Show active camera/mic status dots and app warnings on the pill.",
                    &mut next_cfg.show_privacy_indicators,
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Notification listener access row
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Windows Listener Access:")
                            .size(12.5)
                            .strong()
                            .color(pal.text),
                    );
                    access_badge(ui, &pal, &state.notif_access_status);

                    if state.notif_access_status != "allowed" {
                        let req_btn = egui::Button::new(if state.requesting_notif_access {
                            "Requesting..."
                        } else {
                            "Request Access"
                        })
                        .corner_radius(CornerRadius::same(8));
                        if ui
                            .add_enabled(!state.requesting_notif_access, req_btn)
                            .clicked()
                        {
                            state.requesting_notif_access = true;
                            actions.push(SettingsAction::RequestNotificationAccess);
                        }
                    }

                    if ui
                        .add(
                            egui::Button::new("Clear Notifications")
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        actions.push(SettingsAction::ClearNotifications);
                    }
                });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Blocked Apps (Suppress Notifications)")
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
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    let edit = TextEdit::singleline(&mut state.blocked_app_input)
                        .hint_text("e.g. discord.exe, Slack")
                        .desired_width(220.0);
                    let resp = ui.add(edit);
                    let enter = resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let add_clicked = ui
                        .add(egui::Button::new("Add").corner_radius(CornerRadius::same(8)))
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
                                        ui.label(
                                            RichText::new(app).size(12.0).color(pal.text),
                                        );
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(icons::X)
                                                        .size(11.0)
                                                        .color(pal.muted),
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
            });

            ui.add_space(14.0);

            // 5. Data & Diagnostics
            section_card(ui, &pal, "Data & Diagnostics", icons::DATABASE, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "ORBIT stores config.toml, local SQLite calendar tasks, and Shelf items in %APPDATA%\\ORBIT. Notifications stay in memory only.",
                        )
                        .size(11.5)
                        .color(pal.muted),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(
                            egui::Button::new("Open Config Folder")
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        actions.push(SettingsAction::RevealConfigFolder);
                    }

                    if ui
                        .add(
                            egui::Button::new("Reset Defaults")
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
                                    RichText::new("Clear Local Data").color(pal.danger),
                                )
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
                                    RichText::new("Confirm Clear?").color(Color32::WHITE),
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
                                egui::Button::new("Cancel")
                                    .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                        {
                            state.confirm_clear_data = false;
                        }
                    }
                });

                ui.add_space(6.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if state.show_privacy_terms {
                                "Hide Privacy Note"
                            } else {
                                "View Privacy Note"
                            })
                            .size(11.5)
                            .color(pal.accent),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    state.show_privacy_terms = !state.show_privacy_terms;
                }
                if state.show_privacy_terms {
                    Frame::new()
                        .fill(pal.chip_bg)
                        .stroke(Stroke::new(1.0, pal.chip_border))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "Zero telemetry. No cloud accounts. Notification bodies, media titles, and camera/mic status are kept strictly in RAM and never written to disk.",
                                    )
                                    .size(11.5)
                                    .color(pal.muted),
                                )
                                .wrap(),
                            );
                        });
                }
            });

            ui.add_space(10.0);
        });

    if next_cfg != *cfg {
        actions.push(SettingsAction::ConfigChanged(next_cfg));
    }

    actions
}

fn apply_local_visuals(ui: &mut Ui, pal: &Palette) {
    let mut visuals = if pal.is_light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.panel_fill = pal.canvas_bg;
    visuals.window_fill = pal.card_bg;
    visuals.selection.bg_fill = pal.accent;
    let style = ui.style_mut();
    style.visuals = visuals;
    style.spacing.item_spacing = vec2(8.0, 6.0);
}

fn header_bar(
    ui: &mut Ui,
    pal: &Palette,
    logo_tex: Option<egui::TextureId>,
    _state: &mut SettingsUiState,
    actions: &mut Vec<SettingsAction>,
) {
    Frame::new()
        .fill(pal.card_bg)
        .stroke(Stroke::new(1.0, pal.card_border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(tex) = logo_tex {
                    let (logo_r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                    crate::paint::rounded_texture(
                        ui.painter(),
                        logo_r,
                        8.0,
                        tex,
                        1.0,
                        pal.card_border,
                    );
                }
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("ORBIT Settings")
                                .size(16.0)
                                .strong()
                                .color(pal.text),
                        );
                        Frame::new()
                            .fill(pal.accent_soft)
                            .corner_radius(CornerRadius::same(99))
                            .inner_margin(Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                        .size(10.5)
                                        .strong()
                                        .color(pal.accent),
                                );
                            });
                    });
                    ui.label(
                        RichText::new("Changes apply immediately and are saved to config.toml")
                            .size(11.5)
                            .color(pal.muted),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new("Close").corner_radius(CornerRadius::same(8)))
                        .clicked()
                    {
                        actions.push(SettingsAction::Close);
                    }
                });
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
        .stroke(Stroke::new(1.0, fg.gamma_multiply(0.45)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(msg).size(12.0).color(fg)).wrap());
        });
}

fn section_card(
    ui: &mut Ui,
    pal: &Palette,
    title: &str,
    icon_glyph: &str,
    add_contents: impl FnOnce(&mut Ui),
) {
    Frame::new()
        .fill(pal.card_bg)
        .stroke(Stroke::new(1.0, pal.card_border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(icon_glyph)
                        .font(crate::pill::icon_font(14.5, false))
                        .color(pal.accent),
                );
                ui.label(
                    RichText::new(title)
                        .size(13.0)
                        .strong()
                        .color(pal.text),
                );
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(6.0);
            add_contents(ui);
        });
}

fn segmented_row<T: Copy + PartialEq>(
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
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for &(val, text) in options {
            let selected = val == current;
            let fill = if selected { pal.accent_soft } else { pal.chip_bg };
            let stroke = if selected {
                Stroke::new(1.25, pal.accent)
            } else {
                Stroke::new(1.0, pal.chip_border)
            };
            let txt_col = if selected { pal.accent } else { pal.text };
            let btn = egui::Button::new(RichText::new(text).size(12.0).color(txt_col))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(68.0, 26.0));
            if ui.add(btn).clicked() && !selected {
                picked = Some(val);
            }
        }
    });
    picked
}

fn toggle_row(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    hint: &str,
    value: &mut bool,
) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let text_w = (ui.available_width() - 56.0).max(120.0);
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
    let size = vec2(40.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let anim_t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = if *on {
        pal.accent
    } else if pal.is_light {
        Color32::from_rgb(203, 213, 225)
    } else {
        Color32::from_rgb(51, 65, 85)
    };
    ui.painter().rect_filled(rect, 11.0, bg);
    let knob_x = egui::lerp((rect.min.x + 11.0)..=(rect.max.x - 11.0), anim_t);
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), 8.0, Color32::WHITE);
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
                    .color(pal.accent),
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
        .fill(color.gamma_multiply(0.18))
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

        assert!(Theme::is_light_effective(ThemeMode::Light, false));
        assert!(!Theme::is_light_effective(ThemeMode::Dark, true));
        assert!(Theme::is_light_effective(ThemeMode::Auto, true));
        assert!(!Theme::is_light_effective(ThemeMode::Auto, false));

        let light_theme = Theme::for_mode(ThemeMode::Auto, true, FALLBACK_ACCENT);
        let dark_theme = Theme::for_mode(ThemeMode::Auto, false, FALLBACK_ACCENT);
        assert_ne!(light_theme.pill_top, dark_theme.pill_top);
        assert_ne!(light_theme.pill_text, dark_theme.pill_text);
    }

    #[test]
    fn settings_renders_and_drops_cleanly_at_multiple_widths() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(
            &ctx,
            crate::theme::FontAssets {
                regular: None,
                semibold: None,
            },
        );
        let cfg = Config::default();
        for width in [460.0_f32, 640.0, 960.0] {
            let mut state = SettingsUiState::default();
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
}
