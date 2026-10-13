use crate::services::config::{load_config, save_config as persist_config, AppConfig};
use crate::services::telemetry::{measure_memory, MemoryReport};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    Ok(load_config(&app))
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    persist_config(&app, &config)?;
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
            let win_width = 700.0;
            let x = (logical_screen_width - win_width) / 2.0;
            let y = 12.0; // Moderate spacing from the top edge as per PRD
            let _ = main_win.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
    }
    Ok(())
}
