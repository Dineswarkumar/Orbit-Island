use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::services::config::{load_config, save_config as persist_config, AppConfig};
use crate::services::media::{
    get_available_media_sources, get_current_media_session, run_blocking_media_task, seek_media,
    send_media_command, MediaSessionSnapshot, MediaSourceInfo,
};
use crate::services::notifications::{
    get_access_status, request_access_async, NotificationManager, NotificationSnapshot,
};
use crate::services::privacy::{inspect_privacy_indicators, PrivacyState};
use crate::services::telemetry::{measure_memory, MemoryReport};
use tauri::menu::{CheckMenuItem, ContextMenu, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    if let Some(store) = app.try_state::<std::sync::Arc<crate::ConfigStore>>() {
        return Ok(store.get());
    }
    Ok(load_config(&app))
}

#[tauri::command]
pub fn show_context_menu(app: AppHandle, window: tauri::Window) -> Result<bool, String> {
    let cfg = load_config(&app);
    let active_theme = cfg.theme.as_str();

    let pin_item =
        MenuItem::with_id(&app, "ctx_pin_expanded", "Pin Expanded Island", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let hide_item =
        MenuItem::with_id(&app, "ctx_hide", "Hide Island", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let dismiss_item =
        MenuItem::with_id(&app, "ctx_dismiss", "Dismiss", true, None::<&str>)
            .map_err(|e| e.to_string())?;

    let pause_fs_item = CheckMenuItem::with_id(
        &app,
        "ctx_pause_fullscreen",
        "Auto-Hide in Fullscreen Video",
        true,
        cfg.pause_in_fullscreen,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let expand_click_item = CheckMenuItem::with_id(
        &app,
        "ctx_expand_click",
        "Expand on Click Only",
        true,
        cfg.expand_trigger == "click",
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let stealth_capture_item = CheckMenuItem::with_id(
        &app,
        "ctx_stealth_capture",
        "Hide in Screen Recording / Share",
        true,
        cfg.hide_from_screen_capture,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let test_privacy_item = MenuItem::with_id(
        &app,
        "ctx_test_privacy",
        "Test Privacy Light (4s)",
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;

    let theme_specs = [
        ("theme_system", "System Auto", "system"),
        ("theme_obsidian", "Midnight Sapphire", "obsidian"),
        ("theme_dynamic", "Nebula Violet", "dynamic"),
        ("theme_dark", "VisionOS Spatial Titanium", "dark"),
        ("theme_black", "OLED Cyber Pitch", "black"),
        ("theme_espresso", "Sunset Rose & Amber", "espresso"),
        ("theme_evergreen", "Aurora Botanical Emerald", "evergreen"),
        ("theme_nord", "Arc Ocean & Solar Gold", "nord"),
        ("theme_light", "Apple Ceramic Pearl", "light"),
    ];
    let theme_items = theme_specs
        .iter()
        .map(|(id, label, theme)| {
            CheckMenuItem::with_id(
                &app,
                *id,
                *label,
                true,
                active_theme == *theme,
                None::<&str>,
            )
            .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let theme_item_refs = theme_items
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<_>)
        .collect::<Vec<_>>();

    let theme_submenu = Submenu::with_items(
        &app,
        "Theme",
        true,
        &theme_item_refs,
    )
    .map_err(|e| e.to_string())?;

    let timer_5 = MenuItem::with_id(&app, "ctx_timer_5", "Start 5m Timer", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let timer_15 = MenuItem::with_id(&app, "ctx_timer_15", "Start 15m Timer", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let timer_25 = MenuItem::with_id(&app, "ctx_timer_25", "Start 25m Focus", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let timer_stop = MenuItem::with_id(&app, "ctx_timer_stop", "Cancel Timer", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let timer_submenu = Submenu::with_items(
        &app,
        "Timer",
        true,
        &[&timer_5, &timer_15, &timer_25, &timer_stop],
    )
    .map_err(|e| e.to_string())?;

    let sep1 = PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?;
    let settings_item =
        MenuItem::with_id(&app, "ctx_settings", "ORBIT Settings...", true, None::<&str>)
            .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        &app,
        &[
            &pin_item,
            &hide_item,
            &dismiss_item,
            &sep1,
            &pause_fs_item,
            &expand_click_item,
            &stealth_capture_item,
            &test_privacy_item,
            &timer_submenu,
            &theme_submenu,
            &sep2,
            &settings_item,
        ],
    )
    .map_err(|e| e.to_string())?;

    menu.popup(window.clone()).map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{HWND, POINT};
        use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

        if let Ok(hwnd) = window.hwnd() {
            let mut pt = POINT { x: 0, y: 0 };
            let is_inside = unsafe {
                if GetCursorPos(&mut pt) != 0 && ScreenToClient(hwnd.0 as HWND, &mut pt) != 0 {
                    if let Ok(Some(monitor)) = window.primary_monitor() {
                        let scale = monitor.scale_factor();
                        let logical_x = pt.x as f64 / scale;
                        let logical_y = pt.y as f64 / scale;
                        logical_x >= 30.0 && logical_x <= 370.0 && logical_y >= 2.0 && logical_y <= 158.0
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            return Ok(is_inside);
        }
    }

    Ok(false)
}

pub fn verify_settings_window(label: &str) -> Result<(), String> {
    if label != "settings" {
        Err("Access denied: command only permitted from settings window".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    let previous = load_config(&app);
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Main island window not found".to_string())?;
    main_win
        .set_always_on_top(config.always_on_top)
        .map_err(|e| e.to_string())?;
    crate::services::window::set_window_topmost(&main_win, config.always_on_top);
    crate::services::window::set_window_capture_stealth(&main_win, config.hide_from_screen_capture);
    if config.position != previous.position {
        let _ = crate::services::window::position_island_window(&main_win, &config.position);
    }
    if let Err(error) = persist_config(&app, &config) {
        if let Err(rollback_error) = main_win.set_always_on_top(previous.always_on_top) {
            log::error!("Failed to roll back always-on-top after config save failed: {rollback_error}");
        }
        crate::services::window::set_window_topmost(&main_win, previous.always_on_top);
        crate::services::window::set_window_capture_stealth(&main_win, previous.hide_from_screen_capture);
        return Err(error);
    }

    let island_visible = main_win.is_visible().unwrap_or(false);
    if let Err(error) = crate::sync_tray_visibility(&app, island_visible) {
        log::error!("Failed to apply system tray visibility setting: {error}");
    }

    if let Some(vis) = app.try_state::<std::sync::Arc<crate::IslandVisibilityState>>() {
        vis.shortcuts_enabled
            .store(config.shortcuts_enabled, std::sync::atomic::Ordering::SeqCst);
    }

    if let Some(store) = app.try_state::<std::sync::Arc<crate::ConfigStore>>() {
        store.update(config.clone());
    }

    let _ = app.emit("config-changed", &config);

    Ok(config)
}

#[tauri::command]
pub fn hide_island_window(app: AppHandle) -> Result<(), String> {
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Main island window not found".to_string())?;
    // Do not let the user hide the island while the only restore path is also hidden.
    if let Err(error) = crate::sync_tray_visibility(&app, false) {
        return Err(error);
    }
    if let Some(vis) = app.try_state::<std::sync::Arc<crate::IslandVisibilityState>>() {
        vis.user_manually_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let _ = crate::services::window::apply_island_region(&main_win, "compact");
    if let Err(error) = main_win.hide() {
        if let Some(vis) = app.try_state::<std::sync::Arc<crate::IslandVisibilityState>>() {
            vis.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
        }
        let island_visible = main_win.is_visible().unwrap_or(true);
        let _ = crate::sync_tray_visibility(&app, island_visible);
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(settings_win) = app.get_webview_window("settings") {
        settings_win.show().map_err(|e| e.to_string())?;
        settings_win.unminimize().map_err(|e| e.to_string())?;
        settings_win.set_focus().map_err(|e| e.to_string())
    } else {
        let settings_win = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("ORBIT Settings")
        .inner_size(640.0, 520.0)
        .resizable(true)
        .decorations(true)
        .always_on_top(false)
        .skip_taskbar(false)
        .build()
        .map_err(|e| e.to_string())?;

        let win_clone = settings_win.clone();
        settings_win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win_clone.hide();
            }
        });

        settings_win.show().map_err(|e| e.to_string())?;
        settings_win.set_focus().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn close_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(settings_win) = app.get_webview_window("settings") {
        settings_win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_memory_metrics() -> Result<MemoryReport, String> {
    measure_memory()
}

#[tauri::command]
pub fn set_island_state(
    app: AppHandle,
    state: String,
    width: Option<f64>,
) -> Result<(), String> {
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Main island window not found".to_string())?;

    crate::services::window::apply_island_region_sized(&main_win, &state, width)?;

    // Smart Z-Order: When Always on Top is OFF in config (and not suppressed by fullscreen video),
    // temporarily promote the island to top for transient notifications, reminders, or active expansion,
    // then drop back behind normal windows as soon as it returns to compact idle.
    let is_fs_suppressed = app
        .try_state::<std::sync::Arc<crate::IslandVisibilityState>>()
        .map(|v| v.auto_suppressed_fullscreen.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false);

    if !is_fs_suppressed {
        let cfg = load_config(&app);
        if !cfg.always_on_top {
            let should_temp_top = state == "notification"
                || state == "notification_hover"
                || state == "expanded"
                || state == "drag_targets"
                || state == "compact_alert";
            let _ = main_win.set_always_on_top(should_temp_top);
            crate::services::window::force_window_topmost(&main_win, should_temp_top);
        } else {
            crate::services::window::set_window_topmost(&main_win, true);
        }
    }

    Ok(())
}

/// Resizes only the native hit region of a compact-family state to match the live pill width.
/// Intentionally does NOT touch z-order, so temporary alert promotion is never disturbed.
#[tauri::command]
pub fn update_island_region(app: AppHandle, state: String, width: f64) -> Result<(), String> {
    if !matches!(state.as_str(), "compact" | "compact_alert" | "compact_dual") {
        return Ok(());
    }
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Main island window not found".to_string())?;
    crate::services::window::apply_island_region_sized(&main_win, &state, Some(width))
}

#[derive(Default, Clone)]
pub struct MediaState {
    pub pinned_source: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

#[tauri::command]
pub async fn get_media_state(
    state: tauri::State<'_, MediaState>,
    pinned_source: Option<String>,
) -> Result<Option<MediaSessionSnapshot>, String> {
    let pin = pinned_source.or_else(|| state.pinned_source.lock().ok().and_then(|p| p.clone()));
    run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        Ok(get_current_media_session(pin.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn get_media_sources(
    state: tauri::State<'_, MediaState>,
    pinned_source: Option<String>,
) -> Result<Vec<MediaSourceInfo>, String> {
    let pin = pinned_source.or_else(|| state.pinned_source.lock().ok().and_then(|p| p.clone()));
    run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        Ok(get_available_media_sources(pin.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn pin_media_source(
    app: AppHandle,
    state: tauri::State<'_, MediaState>,
    source_id: Option<String>,
) -> Result<Option<MediaSessionSnapshot>, String> {
    if let Ok(mut pinned) = state.pinned_source.lock() {
        *pinned = source_id.clone();
    }
    let sid = source_id.clone();
    let updated = run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        Ok(get_current_media_session(sid.as_deref()))
    })
    .await
    .unwrap_or(None);

    let _ = app.emit("media-state-changed", &updated);
    Ok(updated)
}

#[tauri::command]
pub async fn media_send_command(
    app: AppHandle,
    state: tauri::State<'_, MediaState>,
    command: String,
    pinned_source: Option<String>,
) -> Result<Option<MediaSessionSnapshot>, String> {
    let pin = pinned_source.or_else(|| state.pinned_source.lock().ok().and_then(|p| p.clone()));
    let cmd = command.clone();
    let cmd_pin = pin.clone();

    // Execute the media command off the main thread with a 2500ms timeout
    run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        send_media_command(&cmd, cmd_pin.as_deref())
    })
    .await?;

    // Allow the external media player a brief moment to update its state before taking snapshot
    let fetch_pin = pin.clone();
    let updated = run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        Ok(get_current_media_session(fetch_pin.as_deref()))
    })
    .await
    .unwrap_or(None);

    let _ = app.emit("media-state-changed", &updated);
    Ok(updated)
}

#[tauri::command]
pub async fn media_seek(
    app: AppHandle,
    state: tauri::State<'_, MediaState>,
    position_secs: f64,
    pinned_source: Option<String>,
) -> Result<Option<MediaSessionSnapshot>, String> {
    let pin = pinned_source.or_else(|| state.pinned_source.lock().ok().and_then(|p| p.clone()));
    let cmd_pin = pin.clone();

    // Execute the seek command off the main thread with a 2500ms timeout
    run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        seek_media(position_secs, cmd_pin.as_deref())
    })
    .await?;

    let fetch_pin = pin.clone();
    let updated = run_blocking_media_task(std::time::Duration::from_millis(2500), move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(get_current_media_session(fetch_pin.as_deref()))
    })
    .await
    .unwrap_or(None);

    let _ = app.emit("media-state-changed", &updated);
    Ok(updated)
}

#[tauri::command]
pub fn get_notification_state(
    app: AppHandle,
    notif_manager: tauri::State<'_, Arc<NotificationManager>>,
) -> Result<NotificationSnapshot, String> {
    let config = load_config(&app);
    let snapshot = notif_manager.get_current_snapshot();
    Ok(crate::services::notifications::apply_filtering_and_privacy(snapshot, &config))
}

#[tauri::command]
pub fn get_notification_access_status() -> Result<String, String> {
    Ok(get_access_status())
}

#[tauri::command]
pub async fn request_notification_access() -> Result<String, String> {
    request_access_async().await
}

#[tauri::command]
pub fn dismiss_notification(
    id: u32,
    notif_manager: tauri::State<'_, Arc<NotificationManager>>,
) -> Result<(), String> {
    notif_manager.dismiss_notification(id);
    Ok(())
}

#[tauri::command]
pub fn clear_all_notifications(
    notif_manager: tauri::State<'_, Arc<NotificationManager>>,
) -> Result<(), String> {
    notif_manager.clear_all();
    Ok(())
}

#[tauri::command]
pub fn get_privacy_state(app: AppHandle) -> Result<PrivacyState, String> {
    let config = load_config(&app);
    if !config.show_privacy_indicators {
        return Ok(PrivacyState {
            camera_active: false,
            camera_app: None,
            microphone_active: false,
            microphone_app: None,
            is_reliable: true,
        });
    }
    Ok(inspect_privacy_indicators())
}

#[tauri::command]
pub fn trigger_privacy_test(app: AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    std::thread::spawn(move || {
        // Step 1: Camera active (Green) - 1.3s
        let _ = app_handle.emit(
            "privacy-state-changed",
            PrivacyState {
                camera_active: true,
                camera_app: Some("Camera (Test)".to_string()),
                microphone_active: false,
                microphone_app: None,
                is_reliable: true,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(1300));

        // Step 2: Microphone active (Yellow) - 1.3s
        let _ = app_handle.emit(
            "privacy-state-changed",
            PrivacyState {
                camera_active: false,
                camera_app: None,
                microphone_active: true,
                microphone_app: Some("Microphone (Test)".to_string()),
                is_reliable: true,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(1300));

        // Step 3: Both active (Cyan) - 1.4s
        let _ = app_handle.emit(
            "privacy-state-changed",
            PrivacyState {
                camera_active: true,
                camera_app: Some("Dual Cam & Mic (Test)".to_string()),
                microphone_active: true,
                microphone_app: Some("Dual Cam & Mic (Test)".to_string()),
                is_reliable: true,
            },
        );
        std::thread::sleep(std::time::Duration::from_millis(1400));

        // Step 4: Revert to actual hardware telemetry
        let real_state = inspect_privacy_indicators();
        let _ = app_handle.emit("privacy-test-finished", real_state);
    });
    Ok(())
}

// Reminders & Productivity Hub (SQLite persistence)
#[tauri::command]
pub fn get_reminders(
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<Vec<crate::services::db::ReminderItem>, String> {
    db.get_reminders()
}

#[tauri::command]
pub fn add_reminder(
    app: AppHandle,
    title: String,
    due_timestamp_ms: u64,
    category: String,
    source: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<crate::services::db::ReminderItem, String> {
    let item = db.add_reminder(&title, due_timestamp_ms, &category, &source)?;
    if let Ok(updated) = db.get_reminders() {
        let _ = app.emit("reminders-updated", &updated);
    }
    Ok(item)
}

#[tauri::command]
pub fn toggle_reminder(
    app: AppHandle,
    id: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<bool, String> {
    let res = db.toggle_reminder(&id)?;
    if let Ok(updated) = db.get_reminders() {
        let _ = app.emit("reminders-updated", &updated);
    }
    Ok(res)
}

#[tauri::command]
pub fn delete_reminder(
    app: AppHandle,
    id: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    db.delete_reminder(&id)?;
    if let Ok(updated) = db.get_reminders() {
        let _ = app.emit("reminders-updated", &updated);
    }
    Ok(())
}

#[tauri::command]
pub async fn pick_and_import_calendar(
    app: AppHandle,
    source_tag: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<usize, String> {
    #[cfg(target_os = "windows")]
    {
        let db = db.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let script = r#"
                Add-Type -AssemblyName System.Windows.Forms
                $dialog = New-Object System.Windows.Forms.OpenFileDialog
                $dialog.Title = 'Import Academic or Office Calendar (.ics, .csv, .json)'
                $dialog.Filter = 'Calendar & Schedule Files (*.ics;*.ical;*.csv;*.json)|*.ics;*.ical;*.csv;*.json|All Files (*.*)|*.*'
                $dialog.Multiselect = $false
                if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
                    Write-Output $dialog.FileName
                }
            "#;
            let powershell_exe = crate::services::db::get_powershell_path();
            let output = std::process::Command::new(powershell_exe)
                .args(["-NoProfile", "-NonInteractive", "-Command", script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Failed to open calendar file picker: {e}"))?;
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                return Ok(0);
            }
            let count = db.import_calendar_file(&path, &source_tag)?;
            if let Ok(updated) = db.get_reminders() {
                let _ = app.emit("reminders-updated", &updated);
            }
            Ok(count)
        })
        .await
        .map_err(|e| format!("Calendar import worker stopped unexpectedly: {e}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, source_tag, db);
        Ok(0)
    }
}

#[tauri::command]
pub fn clear_imported_calendar(
    app: AppHandle,
    window: tauri::Window,
    source_filter: Option<String>,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<usize, String> {
    verify_settings_window(window.label())?;
    let deleted = db.clear_imported_reminders(source_filter.as_deref())?;
    if let Ok(updated) = db.get_reminders() {
        let _ = app.emit("reminders-updated", &updated);
    }
    Ok(deleted)
}

// Hardware & System Monitor
#[tauri::command]
pub fn get_hardware_metrics(
    hw: tauri::State<'_, Arc<crate::services::hardware::HardwareManager>>,
) -> Result<crate::services::hardware::HardwareMetrics, String> {
    Ok(hw.get_metrics())
}

#[tauri::command]
pub async fn free_up_memory(
    app: AppHandle,
    hw: tauri::State<'_, Arc<crate::services::hardware::HardwareManager>>,
) -> Result<crate::services::hardware::MemoryCleanResult, String> {
    let hw_clone = hw.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        hw_clone.free_up_memory()
    })
    .await
    .map_err(|e| format!("Failed to execute free_up_memory task: {e}"))?;

    let metrics = hw.get_metrics();
    let _ = app.emit("hardware-metrics-changed", &metrics);

    Ok(result)
}

// File Shelf (unbounded references, SQLite persistence)
#[tauri::command]
pub fn get_shelf_items(
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<Vec<crate::services::db::ShelfItem>, String> {
    db.get_shelf_items()
}

#[tauri::command]
pub async fn add_shelf_item(
    app: AppHandle,
    file_path: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<crate::services::db::ShelfItem, String> {
    let cfg = load_config(&app);
    let storage_mode = cfg.shelf_storage_mode;
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.add_shelf_item_with_mode(&file_path, &storage_mode)
    })
    .await
    .map_err(|error| format!("Shelf worker stopped unexpectedly: {error}"))?
}

#[tauri::command]
pub async fn handle_file_drop(
    app: AppHandle,
    paths: Vec<String>,
    action: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<Vec<crate::services::db::DropFileResult>, String> {
    let storage_mode = load_config(&app).shelf_storage_mode;
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db.handle_dropped_files(&paths, &action, &storage_mode)
    })
    .await
    .map_err(|error| format!("File drop worker stopped unexpectedly: {error}"))
}

#[tauri::command]
pub async fn share_files(window: tauri::Window, paths: Vec<String>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let hwnd = window
            .hwnd()
            .map_err(|error| format!("Could not connect sharing to the ORBIT window: {error}"))?
            .0 as isize;
        let validated_paths = paths
            .iter()
            .map(|raw_path| {
                let path = crate::services::db::validate_local_safe_path(raw_path)?;
                if !path.is_file() {
                    return Err("Only regular files can be shared.".to_string());
                }
                Ok(path.to_string_lossy().into_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;

        tauri::async_runtime::spawn_blocking(move || {
            let files = crate::services::windows_share::prepare_share_files(&validated_paths)?;
            let send_files = crate::services::windows_share::SendStorageFiles(files);
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            window
                .run_on_main_thread(move || {
                    let files = send_files.take();
                    let res = crate::services::windows_share::show_share_ui_for_window_on_main_thread(hwnd, files);
                    let _ = tx.send(res);
                })
                .map_err(|e| format!("Failed to dispatch share UI to main thread: {e}"))?;
            rx.recv().map_err(|e| format!("Share UI response channel closed: {e}"))?
        })
        .await
        .map_err(|error| format!("Windows Share worker stopped unexpectedly: {error}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, paths);
        Err("Windows app sharing is only available on Windows.".into())
    }
}

#[tauri::command]
pub fn remove_shelf_item(
    id: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    db.remove_shelf_item(&id)
}

#[tauri::command]
pub fn open_shelf_file(
    id: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    db.open_shelf_file(&id)
}

#[tauri::command]
pub fn show_shelf_file_in_folder(
    id: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    db.show_shelf_file_in_folder(&id)
}

#[tauri::command]
pub async fn pick_shelf_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let script = r#"
                Add-Type -AssemblyName System.Windows.Forms
                $dialog = New-Object System.Windows.Forms.OpenFileDialog
                $dialog.Title = 'Select File for ORBIT Shelf'
                $dialog.Filter = 'All Files (*.*)|*.*'
                $dialog.Multiselect = $false
                if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
                    Write-Output $dialog.FileName
                }
            "#;
            let powershell_exe = crate::services::db::get_powershell_path();
            let output = std::process::Command::new(powershell_exe)
                .args(["-NoProfile", "-NonInteractive", "-Command", script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Failed to open file picker: {e}"))?;
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                Ok(None)
            } else {
                Ok(Some(path))
            }
        })
        .await
        .map_err(|e| format!("File picker worker stopped unexpectedly: {e}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

#[tauri::command]
pub fn clear_shelf(
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    db.clear_shelf()
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") || url.chars().any(char::is_control) {
        return Err("Only secure https links can be opened from a pinned destination.".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let result = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL as i32) };
        if (result as isize) <= 32 {
            return Err(format!("Windows could not open that link (code: {result:?})."));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
        Err("Pinned web destinations are currently supported on Windows only.".to_string())
    }
}

#[tauri::command]
pub fn clear_local_data(
    window: tauri::Window,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
    verify_settings_window(window.label())?;
    db.clear_local_data()
}

#[tauri::command]
pub fn activate_app_source(
    app_id: String,
    app_name: String,
    title_hint: Option<String>,
) -> Result<bool, String> {
    crate::services::window::activate_app_window(&app_id, &app_name, title_hint.as_deref())
}

#[tauri::command]
pub fn reveal_diagnostic_log() -> Result<String, String> {
    let log_path = crate::services::logger::get_default_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if !log_path.exists() {
        crate::services::logger::log_event("INFO", "diagnostics", "Diagnostic log exported/revealed by user");
    }
    let path_str = log_path.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let explorer_exe = crate::services::db::get_explorer_path();
        let _ = std::process::Command::new(explorer_exe)
            .arg(format!("/select,{}", path_str))
            .spawn();
    }
    Ok(path_str)
}

#[tauri::command]
pub async fn get_live_backdrop(
    app: AppHandle,
) -> Result<Option<crate::services::window::LiveBackdropSnapshot>, String> {
    let target_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Island main window not found".to_string())?;
    // The sampler waits a few milliseconds for DWM, so it must never run on the UI thread.
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::window::capture_live_backdrop(&target_win)
    })
    .await
    .map_err(|e| format!("Live backdrop task failed: {e}"))?
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherGlance {
    pub temp_c: i32,
    pub temp_f: i32,
    pub condition_code: String,
    pub label: String,
}

struct WeatherCache {
    data: WeatherGlance,
    fetched_at: Instant,
}

static WEATHER_CACHE: Mutex<Option<WeatherCache>> = Mutex::new(None);

pub fn parse_wttr_json(json_str: &str) -> Result<WeatherGlance, String> {
    let val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid weather JSON: {e}"))?;

    let current = val
        .get("current_condition")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| "Missing current_condition in weather data".to_string())?;

    let temp_c = current
        .get("temp_C")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| "Missing or invalid temp_C".to_string())?;

    let temp_f = current
        .get("temp_F")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or_else(|| (temp_c * 9 / 5) + 32);

    let code = current
        .get("weatherCode")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(113);

    let condition_code = match code {
        200 | 386 | 389 | 392 | 395 => "storm",
        176 | 293 | 296 | 299 | 302 | 305 | 308 | 311 | 353 | 356 | 359 => "rain",
        179 | 182 | 227 | 230 | 323 | 326 | 329 | 332 | 335 | 338 | 368 | 371 => "snow",
        116 => "partly_cloudy",
        119 | 122 | 143 | 248 => "cloudy",
        _ => "clear",
    }
    .to_string();

    let label = current
        .get("weatherDesc")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|item| item.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Clear".to_string());

    Ok(WeatherGlance {
        temp_c,
        temp_f,
        condition_code,
        label,
    })
}

fn fetch_weather_sync() -> Result<WeatherGlance, String> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let curl_exe = crate::services::db::get_curl_path();
    let mut cmd = std::process::Command::new(curl_exe);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.args(["-s", "--max-time", "6", "https://wttr.in/?format=j1"]);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute curl: {e}"))?;

    if output.status.success() {
        if let Ok(body) = String::from_utf8(output.stdout) {
            if let Ok(parsed) = parse_wttr_json(&body) {
                return Ok(parsed);
            }
        }
    }

    Ok(WeatherGlance {
        temp_c: 24,
        temp_f: 75,
        condition_code: "partly_cloudy".to_string(),
        label: "Partly Cloudy".to_string(),
    })
}

#[tauri::command]
pub async fn get_weather() -> Result<WeatherGlance, String> {
    // Return cached weather data if fetched within the last 10 minutes (600s)
    if let Ok(cache) = WEATHER_CACHE.lock() {
        if let Some(ref c) = *cache {
            if c.fetched_at.elapsed() < std::time::Duration::from_secs(600) {
                return Ok(c.data.clone());
            }
        }
    }

    // Proxy request through backend to bypass browser CSP and avoid CORS blocks
    let result = tauri::async_runtime::spawn_blocking(fetch_weather_sync)
        .await
        .map_err(|e| format!("Weather task failed to join: {e}"))??;

    if let Ok(mut cache) = WEATHER_CACHE.lock() {
        *cache = Some(WeatherCache {
            data: result.clone(),
            fetched_at: Instant::now(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wttr_json_valid_data() {
        let sample_json = r#"{
            "current_condition": [
                {
                    "temp_C": "21",
                    "temp_F": "70",
                    "weatherCode": "116",
                    "weatherDesc": [
                        { "value": "Partly cloudy" }
                    ]
                }
            ]
        }"#;

        let glance = parse_wttr_json(sample_json).expect("Should parse valid wttr JSON");
        assert_eq!(glance.temp_c, 21);
        assert_eq!(glance.temp_f, 70);
        assert_eq!(glance.condition_code, "partly_cloudy");
        assert_eq!(glance.label, "Partly cloudy");
    }

    #[test]
    fn test_parse_wttr_json_rain_condition() {
        let sample_json = r#"{
            "current_condition": [
                {
                    "temp_C": "18",
                    "temp_F": "64",
                    "weatherCode": "296",
                    "weatherDesc": [
                        { "value": "Light rain" }
                    ]
                }
            ]
        }"#;

        let glance = parse_wttr_json(sample_json).expect("Should parse rain wttr JSON");
        assert_eq!(glance.temp_c, 18);
        assert_eq!(glance.temp_f, 64);
        assert_eq!(glance.condition_code, "rain");
        assert_eq!(glance.label, "Light rain");
    }

    #[test]
    fn test_parse_wttr_json_invalid_structure() {
        let bad_json = r#"{"unexpected": 42}"#;
        assert!(parse_wttr_json(bad_json).is_err());
    }

    #[test]
    fn test_verify_settings_window_acl() {
        assert!(super::verify_settings_window("settings").is_ok());

        let denied_main = super::verify_settings_window("main");
        assert!(denied_main.is_err());
        assert_eq!(
            denied_main.unwrap_err(),
            "Access denied: command only permitted from settings window"
        );

        let denied_unknown = super::verify_settings_window("unknown_window");
        assert!(denied_unknown.is_err());

        let denied_empty = super::verify_settings_window("");
        assert!(denied_empty.is_err());
    }

    #[test]
    fn test_qualified_system_binary_paths() {
        let pwsh = crate::services::db::get_powershell_path();
        assert!(pwsh.to_string_lossy().to_lowercase().contains("powershell"));

        let explorer = crate::services::db::get_explorer_path();
        assert!(explorer.to_string_lossy().to_lowercase().contains("explorer"));

        let curl = crate::services::db::get_curl_path();
        assert!(curl.to_string_lossy().to_lowercase().contains("curl"));
    }
}
