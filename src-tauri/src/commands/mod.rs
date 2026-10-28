use std::sync::Arc;
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

    if let Some(tray) = app.try_state::<crate::TrayState>() {
        if let Err(error) = tray.0.set_visible(config.show_system_tray_icon) {
            log::error!("Failed to apply system tray visibility setting: {error}");
        }
    }

    if let Some(vis) = app.try_state::<std::sync::Arc<crate::IslandVisibilityState>>() {
        vis.shortcuts_enabled
            .store(config.shortcuts_enabled, std::sync::atomic::Ordering::SeqCst);
    }

    let _ = app.emit("config-changed", &config);

    Ok(config)
}

#[tauri::command]
pub fn hide_island_window(app: AppHandle) -> Result<(), String> {
    if let Some(vis) = app.try_state::<std::sync::Arc<crate::IslandVisibilityState>>() {
        vis.user_manually_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| "Main island window not found".to_string())?;
    let _ = crate::services::window::apply_island_region(&main_win, "compact");
    main_win.hide().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    let settings_win = app
        .get_webview_window("settings")
        .ok_or_else(|| "Settings window not found".to_string())?;
    settings_win.show().map_err(|e| e.to_string())?;
    settings_win.unminimize().map_err(|e| e.to_string())?;
    settings_win.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_settings_window(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("settings")
        .ok_or_else(|| "Settings window not found".to_string())?
        .hide()
        .map_err(|e| e.to_string())
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
pub fn pick_and_import_calendar(
    app: AppHandle,
    source_tag: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<usize, String> {
    #[cfg(target_os = "windows")]
    {
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
        let output = std::process::Command::new("powershell")
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
    source_filter: Option<String>,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<usize, String> {
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

// File Shelf (Max 10 held references, SQLite persistence)
#[tauri::command]
pub fn get_shelf_items(
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<Vec<crate::services::db::ShelfItem>, String> {
    db.get_shelf_items()
}

#[tauri::command]
pub fn add_shelf_item(
    app: AppHandle,
    file_path: String,
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<crate::services::db::ShelfItem, String> {
    let cfg = load_config(&app);
    db.add_shelf_item_with_mode(&file_path, &cfg.shelf_storage_mode)
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
pub fn pick_shelf_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
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
        let output = std::process::Command::new("powershell")
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
pub fn clear_local_data(
    db: tauri::State<'_, Arc<crate::services::db::DatabaseManager>>,
) -> Result<(), String> {
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
        let _ = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path_str))
            .spawn();
    }
    Ok(path_str)
}

