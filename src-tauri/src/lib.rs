pub mod commands;
pub mod services;

use commands::{
    center_island_window, close_settings_window, get_config, get_memory_metrics,
    open_settings_window, save_config, set_island_state, show_context_menu,
};
use services::config::load_config;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, LogicalPosition, Manager, Position};

#[derive(Clone)]
pub struct TrayState(pub std::sync::Arc<TrayIcon>);

#[cfg(target_os = "windows")]
fn apply_capsule_region(window: &tauri::WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        use windows_sys::Win32::Foundation::{BOOL, HWND};
        use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, HRGN};

        unsafe extern "system" {
            fn SetWindowRgn(hWnd: HWND, hRgn: HRGN, bRedraw: BOOL) -> i32;
        }

        let scale = window.scale_factor().unwrap_or(1.0);
        let phys_w = (200.0 * scale).round() as i32;
        let phys_h = (56.0 * scale).round() as i32;
        let corner_radius = phys_h; // Full capsule corner diameter

        unsafe {
            let rgn = CreateRoundRectRgn(0, 0, phys_w + 1, phys_h + 1, corner_radius, corner_radius);
            if !rgn.is_null() {
                SetWindowRgn(hwnd.0 as HWND, rgn, 1);
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "ctx_hide" => {
                    if let Some(main_win) = app.get_webview_window("main") {
                        let _ = main_win.hide();
                    }
                }
                "ctx_dismiss" => {
                    let _ = set_island_state(app.clone(), "compact".to_string());
                    let _ = app.emit("collapse-island", ());
                }
                "ctx_settings" | "settings" => {
                    let _ = open_settings_window(app.clone());
                }
                "toggle" => {
                    if let Some(main_win) = app.get_webview_window("main") {
                        if let Ok(is_visible) = main_win.is_visible() {
                            if is_visible {
                                let _ = main_win.hide();
                            } else {
                                let _ = main_win.show();
                                let _ = main_win.set_focus();
                            }
                        }
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                "theme_dynamic" | "theme_dark" | "theme_light" | "theme_black" => {
                    let theme_id = match id {
                        "theme_dynamic" => "dynamic",
                        "theme_dark" => "dark",
                        "theme_light" => "light",
                        "theme_black" => "black",
                        _ => "dark",
                    };
                    let mut cfg = load_config(app);
                    cfg.theme = theme_id.to_string();
                    let _ = services::config::save_config(app, &cfg);
                    let _ = app.emit("config-changed", &cfg);
                }
                _ => {}
            }
        })
        .setup(|app| {
            // Initialize and persist default configuration if not already present
            let initial_config = load_config(&app.handle());

            // Position main island at top center on primary monitor (400x56 pill footprint)
            if let Some(main_win) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = main_win.primary_monitor() {
                    let screen_size = monitor.size();
                    let scale = monitor.scale_factor();
                    let logical_width = screen_size.width as f64 / scale;
                    let win_width = 200.0;
                    let x = (logical_width - win_width) / 2.0;
                    let y = 12.0;
                    let _ = main_win.set_position(Position::Logical(LogicalPosition { x, y }));
                }

                // Apply always_on_top from config
                let _ = main_win.set_always_on_top(initial_config.always_on_top);

                // Clip native window to exact capsule pill geometry on Windows
                #[cfg(target_os = "windows")]
                apply_capsule_region(&main_win);
            }

            // Build Tray Menu
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let toggle_item =
                MenuItem::with_id(app, "toggle", "Toggle Island", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit ORBIT", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &toggle_item, &quit_item])?;

            let tray = TrayIconBuilder::with_id("orbit-tray")
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        let _ = open_settings_window(app.clone());
                    }
                    "toggle" => {
                        if let Some(main_win) = app.get_webview_window("main") {
                            if let Ok(is_visible) = main_win.is_visible() {
                                if is_visible {
                                    let _ = main_win.hide();
                                } else {
                                    let _ = main_win.show();
                                    let _ = main_win.set_focus();
                                }
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(main_win) = app.get_webview_window("main") {
                            let _ = main_win.show();
                            let _ = main_win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Retain tray icon handle in app state to guarantee lifetime
            app.manage(TrayState(std::sync::Arc::new(tray)));

            // Intercept settings window close requested event to hide instead of destroy
            if let Some(settings_win) = app.get_webview_window("settings") {
                let win_clone = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            open_settings_window,
            close_settings_window,
            get_memory_metrics,
            center_island_window,
            set_island_state,
            show_context_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
