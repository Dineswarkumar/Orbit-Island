use crate::services::config::{load_config, save_config as persist_config, AppConfig};
use crate::services::telemetry::{measure_memory, MemoryReport};
use tauri::menu::{CheckMenuItem, ContextMenu, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    Ok(load_config(&app))
}

#[tauri::command]
pub fn show_context_menu(app: AppHandle, window: tauri::Window) -> Result<(), String> {
    let cfg = load_config(&app);
    let active_theme = cfg.theme.as_str();

    let hide_item =
        MenuItem::with_id(&app, "ctx_hide", "Hide Island", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let dismiss_item =
        MenuItem::with_id(&app, "ctx_dismiss", "Dismiss", true, None::<&str>)
            .map_err(|e| e.to_string())?;

    // Theme selection submenu with active theme checkmark (Dynamic, Dark, Light, Black)
    let is_dynamic = active_theme == "dynamic";
    let is_dark = active_theme == "dark" || active_theme == "obsidian";
    let is_light = active_theme == "light";
    let is_black = active_theme == "black";

    let theme_dynamic = CheckMenuItem::with_id(
        &app,
        "theme_dynamic",
        "Dynamic",
        true,
        is_dynamic,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let theme_dark = CheckMenuItem::with_id(
        &app,
        "theme_dark",
        "Dark",
        true,
        is_dark,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let theme_light = CheckMenuItem::with_id(
        &app,
        "theme_light",
        "Light",
        true,
        is_light,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let theme_black = CheckMenuItem::with_id(
        &app,
        "theme_black",
        "Black",
        true,
        is_black,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let theme_submenu = Submenu::with_items(
        &app,
        "Theme",
        true,
        &[&theme_dynamic, &theme_dark, &theme_light, &theme_black],
    )
    .map_err(|e| e.to_string())?;

    let sep = PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?;
    let settings_item =
        MenuItem::with_id(&app, "ctx_settings", "ORBIT Settings...", true, None::<&str>)
            .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        &app,
        &[&hide_item, &dismiss_item, &theme_submenu, &sep, &settings_item],
    )
    .map_err(|e| e.to_string())?;

    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    persist_config(&app, &config)?;

    // Dynamically apply Foundation runtime settings
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.set_always_on_top(config.always_on_top);
    }

    // Broadcast config update to all windows (Island and Settings)
    let _ = app.emit("config-changed", &config);

    Ok(config)
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(settings_win) = app.get_webview_window("settings") {
        let _ = settings_win.show();
        let _ = settings_win.unminimize();
        let _ = settings_win.set_focus();
        Ok(())
    } else {
        Err("Settings window not found".to_string())
    }
}

#[tauri::command]
pub fn close_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(settings_win) = app.get_webview_window("settings") {
        let _ = settings_win.hide();
        Ok(())
    } else {
        Err("Settings window not found".to_string())
    }
}

#[tauri::command]
pub fn get_memory_metrics() -> Result<MemoryReport, String> {
    measure_memory()
}

#[tauri::command]
pub fn center_island_window(app: AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = main_win.primary_monitor() {
            let screen_size = monitor.size();
            let screen_scale = monitor.scale_factor();
            let logical_screen_width = screen_size.width as f64 / screen_scale;
            let win_width = 200.0;
            let x = logical_width_screen(logical_screen_width, win_width);
            let y = 12.0;
            let _ = main_win.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_island_state(app: AppHandle, state: String) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        let (width, height) = match state.as_str() {
            "expanded" => (400.0, 184.0),
            _ => (200.0, 56.0),
        };

        if let Ok(Some(monitor)) = main_win.primary_monitor() {
            let screen_size = monitor.size();
            let screen_scale = monitor.scale_factor();
            let logical_screen_width = screen_size.width as f64 / screen_scale;
            let x = logical_width_screen(logical_screen_width, width);
            let y = 12.0;

            let _ = main_win.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
            let _ = main_win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width,
                height,
            }));

            #[cfg(target_os = "windows")]
            {
                if let Ok(hwnd) = main_win.hwnd() {
                    use windows_sys::Win32::Foundation::{BOOL, HWND};
                    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, HRGN};

                    unsafe extern "system" {
                        fn SetWindowRgn(hWnd: HWND, hRgn: HRGN, bRedraw: BOOL) -> i32;
                    }

                    let phys_w = (width * screen_scale).round() as i32;
                    let phys_h = (height * screen_scale).round() as i32;
                    // Both compact (56px) and expanded (28px corner radius) use a 56px corner diameter
                    let corner_diameter = (56.0 * screen_scale).round() as i32;

                    unsafe {
                        let rgn = CreateRoundRectRgn(0, 0, phys_w + 1, phys_h + 1, corner_diameter, corner_diameter);
                        if !rgn.is_null() {
                            SetWindowRgn(hwnd.0 as HWND, rgn, 0); // 0 prevents unpainted flash
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn logical_width_screen(screen_width: f64, win_width: f64) -> f64 {
    (screen_width - win_width) / 2.0
}
