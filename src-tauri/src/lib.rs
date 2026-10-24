pub mod commands;
pub mod services;

use commands::{
    activate_app_source, add_reminder, add_shelf_item, clear_all_notifications, clear_imported_calendar,
    clear_local_data, clear_shelf, close_settings_window, delete_reminder, dismiss_notification,
    free_up_memory, get_config, get_hardware_metrics, get_media_sources, get_media_state,
    get_memory_metrics, get_notification_access_status, get_notification_state, get_privacy_state,
    get_reminders, get_shelf_items, hide_island_window, media_seek, media_send_command,
    open_settings_window, open_shelf_file, pick_and_import_calendar, pick_shelf_file,
    pin_media_source, remove_shelf_item, request_notification_access, reveal_diagnostic_log,
    save_config, set_island_state, show_context_menu, show_shelf_file_in_folder, toggle_reminder,
    trigger_privacy_test, update_island_region, MediaState,
};
use services::config::load_config;
use services::notifications::NotificationManager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

#[derive(Clone)]
pub struct TrayState(pub std::sync::Arc<TrayIcon>);

#[derive(Default)]
pub struct IslandVisibilityState {
    pub user_manually_hidden: std::sync::atomic::AtomicBool,
    pub auto_suppressed_fullscreen: std::sync::atomic::AtomicBool,
    pub shortcuts_enabled: std::sync::atomic::AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_SHOW};

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: *const std::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> *mut std::ffi::c_void;
        }

        let mutex_name: Vec<u16> = "Local\\ORBIT_Island_SingleInstance_Mutex\0"
            .encode_utf16()
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
        if !handle.is_null() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let win_title: Vec<u16> = "ORBIT\0".encode_utf16().collect();
            let existing_hwnd = unsafe { FindWindowW(std::ptr::null(), win_title.as_ptr()) };
            if !existing_hwnd.is_null() {
                unsafe {
                    ShowWindow(existing_hwnd, SW_SHOW);
                }
            }
            return;
        }
    }

    services::logger::init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "ctx_pin_expanded" => {
                    let _ = app.emit("toggle-pin-expanded", ());
                }
                "ctx_pause_fullscreen" => {
                    let mut cfg = load_config(app);
                    cfg.pause_in_fullscreen = !cfg.pause_in_fullscreen;
                    match services::config::save_config(app, &cfg) {
                        Ok(()) => {
                            let _ = app.emit("config-changed", &cfg);
                        }
                        Err(error) => log::error!("Failed to save pause_in_fullscreen from context menu: {error}"),
                    }
                }
                "ctx_expand_click" => {
                    let mut cfg = load_config(app);
                    cfg.expand_trigger = if cfg.expand_trigger == "click" {
                        "hover".to_string()
                    } else {
                        "click".to_string()
                    };
                    match services::config::save_config(app, &cfg) {
                        Ok(()) => {
                            let _ = app.emit("config-changed", &cfg);
                        }
                        Err(error) => log::error!("Failed to save expand_trigger from context menu: {error}"),
                    }
                }
                "ctx_stealth_capture" => {
                    let mut cfg = load_config(app);
                    cfg.hide_from_screen_capture = !cfg.hide_from_screen_capture;
                    if let Some(main_win) = app.get_webview_window("main") {
                        services::window::set_window_capture_stealth(&main_win, cfg.hide_from_screen_capture);
                    }
                    match services::config::save_config(app, &cfg) {
                        Ok(()) => {
                            let _ = app.emit("config-changed", &cfg);
                            let _ = app.emit("stealth-mode-toggled", cfg.hide_from_screen_capture);
                        }
                        Err(error) => log::error!("Failed to save hide_from_screen_capture from context menu: {error}"),
                    }
                }
                "ctx_hide" => {
                    if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                        vis.user_manually_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    let _ = app.emit("request-hide-island", ());
                }
                "ctx_dismiss" => {
                    let _ = app.emit("collapse-island", ());
                }
                "ctx_test_privacy" => {
                    let _ = commands::trigger_privacy_test(app.clone());
                }
                "ctx_timer_5" => {
                    let _ = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 300 }));
                }
                "ctx_timer_15" => {
                    let _ = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 900 }));
                }
                "ctx_timer_25" => {
                    let _ = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 1500 }));
                }
                "ctx_timer_stop" => {
                    let _ = app.emit("timer-command", serde_json::json!({ "action": "stop" }));
                }
                "ctx_settings" | "settings" => {
                    let _ = open_settings_window(app.clone());
                }
                "toggle" => {
                    if let Some(main_win) = app.get_webview_window("main") {
                        if let Ok(is_visible) = main_win.is_visible() {
                            if is_visible {
                                if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                                    vis.user_manually_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
                                }
                                let _ = app.emit("request-hide-island", ());
                            } else {
                                if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                                    vis.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                                }
                                let _ = main_win.show();
                                let _ = main_win.set_focus();
                                let _ = app.emit("island-shown", ());
                            }
                        }
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                "theme_system"
                | "theme_obsidian"
                | "theme_dark"
                | "theme_dynamic"
                | "theme_light"
                | "theme_black"
                | "theme_espresso"
                | "theme_evergreen"
                | "theme_nord" => {
                    let theme_id = match id {
                        "theme_system" => "system",
                        "theme_obsidian" => "obsidian",
                        "theme_dynamic" => "dynamic",
                        "theme_dark" => "dark",
                        "theme_light" => "light",
                        "theme_black" => "black",
                        "theme_espresso" => "espresso",
                        "theme_evergreen" => "evergreen",
                        "theme_nord" => "nord",
                        _ => "obsidian",
                    };
                    let mut cfg = load_config(app);
                    cfg.theme = theme_id.to_string();
                    match services::config::save_config(app, &cfg) {
                        Ok(()) => {
                            let _ = app.emit("config-changed", &cfg);
                        }
                        Err(error) => log::error!("Failed to save theme from context menu: {error}"),
                    }
                }
                _ => {}
            }
        })
        .on_window_event(|window, event| {
            services::logger::log_event(
                "DEBUG",
                "window_event",
                &format!("label: {}, event: {:?}", window.label(), event),
            );
            if window.label() == "main" {
                if let tauri::WindowEvent::ScaleFactorChanged { .. } = event {
                    let app = window.app_handle();
                    let cfg = load_config(app);
                    if let Some(main_win) = app.get_webview_window("main") {
                        let _ = services::window::ensure_window_on_screen(&main_win, &cfg.position);
                    }
                }
            }
        })
        .setup(|app| {
            // Initialize and persist default configuration if not already present
            let initial_config = load_config(&app.handle());

            // Position the compact island on the primary monitor according to config.position.
            if let Some(main_win) = app.get_webview_window("main") {
                if let Err(error) = services::window::init_island_window(&main_win, &initial_config.position) {
                    log::error!("Failed to initialize island window geometry: {error}");
                }

                // Apply always_on_top and screen-capture stealth from config AFTER init_island_window
                let _ = main_win.set_always_on_top(initial_config.always_on_top);
                services::window::set_window_topmost(&main_win, initial_config.always_on_top);
                services::window::set_window_capture_stealth(&main_win, initial_config.hide_from_screen_capture);
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
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                            vis.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        if let Some(main_win) = app.get_webview_window("main") {
                            let _ = main_win.show();
                            let _ = main_win.set_focus();
                            let _ = app.emit("island-shown", ());
                        }
                    }
                })
                .build(app)?;

            // Retain tray icon handle in app state to guarantee lifetime
            app.manage(TrayState(std::sync::Arc::new(tray)));

            // Manage island visibility and auto-suppression state (Task M4.1)
            let visibility_state = std::sync::Arc::new(IslandVisibilityState::default());
            visibility_state
                .shortcuts_enabled
                .store(initial_config.shortcuts_enabled, std::sync::atomic::Ordering::SeqCst);
            app.manage(visibility_state.clone());

            // Spawn global keyboard shortcuts manager thread (Phase 9)
            #[cfg(target_os = "windows")]
            {
                let app_handle_hk = app.handle().clone();
                let vis_hk = visibility_state.clone();
                std::thread::Builder::new()
                    .name("orbit-hotkey-manager".into())
                    .spawn(move || {
                        use windows_sys::Win32::Foundation::HWND;
                        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                            RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT,
                        };
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY,
                        };

                        let mut registered = false;
                        // ID 1 = Alt+O (Expand/Collapse), ID 2 = Alt+H (Hide/Show), ID 3 = Alt+S (Stealth Capture), ID 4 = Alt+M (Play/Pause Media)
                        let hotkeys: [(i32, u32); 4] = [
                            (1, b'O' as u32),
                            (2, b'H' as u32),
                            (3, b'S' as u32),
                            (4, b'M' as u32),
                        ];

                        loop {
                            let should_enable = vis_hk
                                .shortcuts_enabled
                                .load(std::sync::atomic::Ordering::Relaxed);

                            if should_enable && !registered {
                                unsafe {
                                    for &(id, vk) in &hotkeys {
                                        let _ = RegisterHotKey(
                                            0 as HWND,
                                            id,
                                            MOD_ALT | MOD_NOREPEAT,
                                            vk,
                                        );
                                    }
                                }
                                registered = true;
                            } else if !should_enable && registered {
                                unsafe {
                                    for &(id, _) in &hotkeys {
                                        let _ = UnregisterHotKey(0 as HWND, id);
                                    }
                                    let mut dummy: MSG = std::mem::zeroed();
                                    while PeekMessageW(&mut dummy, 0 as HWND, WM_HOTKEY, WM_HOTKEY, PM_REMOVE) != 0 {}
                                }
                                registered = false;
                            }

                            if registered {
                                unsafe {
                                    let mut msg: MSG = std::mem::zeroed();
                                    while PeekMessageW(
                                        &mut msg,
                                        0 as HWND,
                                        WM_HOTKEY,
                                        WM_HOTKEY,
                                        PM_REMOVE,
                                    ) != 0
                                    {
                                        services::logger::log_event(
                                            "DEBUG",
                                            "hotkey",
                                            &format!("Hotkey msg.wParam: {}", msg.wParam),
                                        );
                                        match msg.wParam as i32 {
                                            1 => {
                                                // Alt+O: Expand / Collapse Island
                                                if let Some(main_win) = app_handle_hk.get_webview_window("main") {
                                                    if !main_win.is_visible().unwrap_or(true) {
                                                        vis_hk.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                                                        let _ = main_win.show();
                                                        let _ = app_handle_hk.emit("island-shown", ());
                                                    }
                                                }
                                                let _ = app_handle_hk.emit("hotkey-toggle-expand", ());
                                            }
                                            2 => {
                                                // Alt+H: Hide / Show Island
                                                if let Some(main_win) = app_handle_hk.get_webview_window("main") {
                                                    if let Ok(is_visible) = main_win.is_visible() {
                                                        if is_visible {
                                                            vis_hk.user_manually_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
                                                            let _ = app_handle_hk.emit("request-hide-island", ());
                                                        } else {
                                                            vis_hk.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                                                            let _ = main_win.show();
                                                            let _ = main_win.set_focus();
                                                            let _ = app_handle_hk.emit("island-shown", ());
                                                        }
                                                    }
                                                }
                                            }
                                            3 => {
                                                // Alt+S: Toggle Screen-Share / Recording Stealth
                                                let mut cfg = load_config(&app_handle_hk);
                                                cfg.hide_from_screen_capture = !cfg.hide_from_screen_capture;
                                                if let Some(main_win) = app_handle_hk.get_webview_window("main") {
                                                    services::window::set_window_capture_stealth(
                                                        &main_win,
                                                        cfg.hide_from_screen_capture,
                                                    );
                                                }
                                                if services::config::save_config(&app_handle_hk, &cfg).is_ok() {
                                                    let _ = app_handle_hk.emit("config-changed", &cfg);
                                                    let _ = app_handle_hk.emit("stealth-mode-toggled", cfg.hide_from_screen_capture);
                                                }
                                            }
                                            4 => {
                                                // Alt+M: Play / Pause Media
                                                let _ = app_handle_hk.emit("hotkey-media-play-pause", ());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }

                            std::thread::sleep(std::time::Duration::from_millis(45));
                        }
                    })
                    .expect("failed to spawn hotkey manager thread");
            }

            // Manage media state
            let media_state = MediaState::default();
            app.manage(media_state.clone());

            // Manage SQLite Database for reminders and file shelf
            let db_manager = std::sync::Arc::new(
                services::db::DatabaseManager::for_app(&app.handle())
                    .expect("Failed to initialize SQLite database"),
            );
            app.manage(db_manager);

            // Manage and start hardware monitor
            let hw_manager = std::sync::Arc::new(services::hardware::HardwareManager::new());
            hw_manager.start(app.handle().clone());
            app.manage(hw_manager);

            // Manage and start notification background worker
            let notif_manager = std::sync::Arc::new(NotificationManager::new());
            notif_manager.start(app.handle().clone());
            app.manage(notif_manager);

            // Spawn fullscreen auto-suppression background monitor (Task M4.1)
            let app_handle_fs = app.handle().clone();
            let vis_fs = visibility_state.clone();
            std::thread::Builder::new()
                .name("orbit-fullscreen-monitor".into())
                .spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(600));
                        if let Some(main_win) = app_handle_fs.get_webview_window("main") {
                            if let Ok(hwnd) = main_win.hwnd() {
                                let is_fs = services::window::is_foreground_fullscreen(hwnd.0 as isize);
                                let auto_suppressed = vis_fs.auto_suppressed_fullscreen.load(std::sync::atomic::Ordering::Relaxed);
                                let manually_hidden = vis_fs.user_manually_hidden.load(std::sync::atomic::Ordering::Relaxed);
                                let cfg = load_config(&app_handle_fs);
                                let should_pause = cfg.pause_in_fullscreen;

                                if is_fs && should_pause && !auto_suppressed && !manually_hidden {
                                    services::logger::log_event("DEBUG", "fullscreen", "Auto-suppressing window due to fullscreen");
                                    vis_fs.auto_suppressed_fullscreen.store(true, std::sync::atomic::Ordering::SeqCst);
                                    let _ = app_handle_fs.emit("fullscreen-state-changed", true);
                                    if let Ok(is_vis) = main_win.is_visible() {
                                        if is_vis {
                                            let _ = main_win.hide();
                                        }
                                    }
                                } else if (!is_fs || !should_pause) && auto_suppressed && !manually_hidden {
                                    vis_fs.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                                    let _ = app_handle_fs.emit("fullscreen-state-changed", false);
                                    let _ = main_win.set_always_on_top(cfg.always_on_top);
                                    services::window::set_window_topmost(&main_win, cfg.always_on_top);
                                    let _ = main_win.show();
                                    let _ = app_handle_fs.emit("island-shown", ());
                                } else if !is_fs && !auto_suppressed && !manually_hidden {
                                    if cfg.always_on_top {
                                        services::window::set_window_topmost(&main_win, true);
                                    }
                                }
                            }
                        }
                    }
                })
                .expect("failed to spawn fullscreen monitor thread");

            // Spawn lightweight background thread to poll media session changes
            let app_handle_media = app.handle().clone();
            let monitor_media_state = media_state.clone();
            std::thread::Builder::new()
                .name("orbit-media-monitor".into())
                .spawn(move || {
                    let _com = match services::media::ComGuard::new() {
                        Ok(g) => g,
                        Err(err) => {
                            log::error!("Failed to initialize COM for media monitor thread: {err}");
                            return;
                        }
                    };
                    let mut last_source_id = String::new();
                    let mut last_title = String::new();
                    let mut last_status = String::new();
                    let mut last_duration = 0.0f64;
                    let mut last_can_seek = false;
                    let mut last_pos = 0.0f64;
                    loop {
                        let pin = monitor_media_state.pinned_source.lock().ok().and_then(|p| p.clone());
                        let session = services::media::get_current_media_session(pin.as_deref());
                        let is_playing = session.as_ref().map(|s| s.playback_status.as_str() == "playing").unwrap_or(false);

                        let current_source_id = session.as_ref().map(|s| s.source_id.as_str()).unwrap_or("").to_string();
                        let current_title = session.as_ref().map(|s| s.track.title.as_str()).unwrap_or("").to_string();
                        let current_status = session.as_ref().map(|s| s.playback_status.as_str()).unwrap_or("").to_string();
                        let current_duration = session.as_ref().and_then(|s| s.timeline.as_ref()).map(|t| t.duration_secs).unwrap_or(0.0);
                        let current_can_seek = session.as_ref().map(|s| s.controls.can_seek).unwrap_or(false);
                        let current_pos = session.as_ref().and_then(|s| s.timeline.as_ref()).map(|t| t.position_secs).unwrap_or(0.0);

                        let source_changed = current_source_id != last_source_id;
                        let title_changed = current_title != last_title;
                        let status_changed = current_status != last_status;
                        let duration_changed = (current_duration - last_duration).abs() >= 0.5;
                        let can_seek_changed = current_can_seek != last_can_seek;
                        let pos_changed = (current_pos - last_pos).abs() >= 0.8;

                        if source_changed
                            || title_changed
                            || status_changed
                            || duration_changed
                            || can_seek_changed
                            || (is_playing && pos_changed)
                        {
                            last_source_id = current_source_id;
                            last_title = current_title;
                            last_status = current_status;
                            last_duration = current_duration;
                            last_can_seek = current_can_seek;
                            last_pos = current_pos;
                            let _ = app_handle_media.emit("media-state-changed", &session);
                        }

                        let sleep_duration = if is_playing {
                            std::time::Duration::from_millis(1000)
                        } else {
                            std::time::Duration::from_millis(2000)
                        };
                        std::thread::sleep(sleep_duration);
                    }
                })
                .expect("failed to spawn media monitor thread");

            // Spawn real-time WASAPI audio peak/spectrum monitor for frequency-reactive music waveforms
            let app_handle_spectrum = app.handle().clone();
            std::thread::Builder::new()
                .name("orbit-audio-spectrum".into())
                .spawn(move || {
                    let _com = match services::media::ComGuard::new() {
                        Ok(g) => g,
                        Err(err) => {
                            log::error!("Failed to initialize COM for audio spectrum thread: {err}");
                            return;
                        }
                    };
                    let mut prev_peak = 0.0f32;
                    let mut smooth_low = 0.0f32;
                    let mut phase = 0.0f32;
                    let mut was_active = false;

                    loop {
                        let spectrum = services::media::sample_audio_spectrum(
                            &mut prev_peak,
                            &mut smooth_low,
                            &mut phase,
                        );
                        let is_active = spectrum.peak > 0.004;
                        if is_active || was_active {
                            was_active = is_active;
                            let _ = app_handle_spectrum.emit("media-audio-spectrum", &spectrum);
                        }
                        let delay = if is_active { 95 } else { 220 };
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                    }
                })
                .expect("failed to spawn audio spectrum thread");

            // Initialize native Media Foundation sensor activity monitor for camera hardware streaming
            services::privacy::init_camera_monitor();

            // Spawn real-time background thread to continuously inspect camera & mic privacy indicators
            let app_handle_privacy = app.handle().clone();
            std::thread::Builder::new()
                .name("orbit-privacy-monitor".into())
                .spawn(move || {
                    let _com = match services::media::ComGuard::new() {
                        Ok(g) => g,
                        Err(err) => {
                            log::error!("Failed to initialize COM for privacy monitor thread: {err}");
                            return;
                        }
                    };
                    let mut last_cam = false;
                    let mut last_mic = false;
                    let mut last_cam_app: Option<String> = None;
                    let mut last_mic_app: Option<String> = None;

                    loop {
                        let state = services::privacy::inspect_privacy_indicators();
                        let changed = state.camera_active != last_cam
                            || state.microphone_active != last_mic
                            || state.camera_app != last_cam_app
                            || state.microphone_app != last_mic_app;

                        if changed {
                            last_cam = state.camera_active;
                            last_mic = state.microphone_active;
                            last_cam_app = state.camera_app.clone();
                            last_mic_app = state.microphone_app.clone();
                            let _ = app_handle_privacy.emit("privacy-state-changed", &state);
                        }

                        let delay = if state.camera_active || state.microphone_active {
                            350 // poll faster during active calls/recording
                        } else {
                            750
                        };
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                    }
                })
                .expect("failed to spawn privacy monitor thread");

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
            hide_island_window,
            get_memory_metrics,
            set_island_state,
            show_context_menu,
            get_media_state,
            get_media_sources,
            pin_media_source,
            media_send_command,
            media_seek,
            get_notification_state,
            get_notification_access_status,
            request_notification_access,
            dismiss_notification,
            clear_all_notifications,
            get_privacy_state,
            trigger_privacy_test,
            update_island_region,
            get_reminders,
            add_reminder,
            toggle_reminder,
            delete_reminder,
            pick_and_import_calendar,
            clear_imported_calendar,
            get_hardware_metrics,
            free_up_memory,
            get_shelf_items,
            add_shelf_item,
            remove_shelf_item,
            open_shelf_file,
            show_shelf_file_in_folder,
            pick_shelf_file,
            clear_shelf,
            clear_local_data,
            activate_app_source,
            reveal_diagnostic_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    services::logger::log_event("INFO", "system", "ORBIT run loop ended normally");
}
