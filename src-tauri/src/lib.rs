pub mod commands;
pub mod services;

use commands::{
    activate_app_source, add_reminder, add_shelf_item, clear_all_notifications, clear_imported_calendar,
    clear_local_data, clear_shelf, close_settings_window, delete_reminder, dismiss_notification,
    free_up_memory, get_config, get_hardware_metrics, get_live_backdrop, get_media_sources, get_media_state,
    get_memory_metrics, get_notification_access_status, get_notification_state, get_privacy_state,
    get_reminders, get_shelf_items, get_weather, handle_file_drop, hide_island_window, media_seek, media_send_command,
    open_external_url, open_settings_window, open_shelf_file, pick_and_import_calendar, pick_shelf_file,
    pin_media_source, remove_shelf_item, request_notification_access, reveal_diagnostic_log,
    save_config, set_island_state, share_files, show_context_menu, show_shelf_file_in_folder, toggle_reminder,
    trigger_privacy_test, update_island_region, MediaState,
};
use services::config::load_config;
use services::notifications::NotificationManager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, Listener};

#[derive(Clone)]
pub struct TrayState(pub std::sync::Arc<TrayIcon>);

#[derive(Default)]
pub struct IslandVisibilityState {
    pub user_manually_hidden: std::sync::atomic::AtomicBool,
    pub auto_suppressed_fullscreen: std::sync::atomic::AtomicBool,
    pub shortcuts_enabled: std::sync::atomic::AtomicBool,
}

pub struct ConfigStore {
    inner: std::sync::RwLock<services::config::AppConfig>,
}

impl ConfigStore {
    pub fn new(initial: services::config::AppConfig) -> Self {
        Self {
            inner: std::sync::RwLock::new(initial),
        }
    }

    pub fn get(&self) -> services::config::AppConfig {
        self.inner.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn update(&self, new_config: services::config::AppConfig) {
        let mut guard = self.inner.write().unwrap_or_else(|p| p.into_inner());
        *guard = new_config;
    }
}

/// Keep at least one visible way to restore the island. If the island is hidden,
/// the tray icon overrides the user's normal tray preference until it is shown again.
pub fn sync_tray_visibility(app: &tauri::AppHandle, island_visible: bool) -> Result<(), String> {
    let config = app
        .try_state::<std::sync::Arc<ConfigStore>>()
        .map(|store| store.get())
        .unwrap_or_else(|| services::config::load_config(app));
    let should_show_tray = tray_visibility_for_state(config.show_system_tray_icon, island_visible);
    if let Some(tray) = app.try_state::<TrayState>() {
        tray.0
            .set_visible(should_show_tray)
            .map_err(|error| format!("Could not update the ORBIT tray icon: {error}"))
    } else if should_show_tray {
        Err("ORBIT tray icon is not available to restore the hidden island.".into())
    } else {
        Ok(())
    }
}

fn tray_visibility_for_state(preference_enabled: bool, island_visible: bool) -> bool {
    preference_enabled || !island_visible
}

#[cfg(target_os = "windows")]
pub const WM_ORBIT_UPDATE_HOTKEYS: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[derive(Default, Clone)]
pub struct HotkeyState {
    pub thread_id: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

/// Spawns a supervised background thread that catches panics and restarts the worker loop
/// with exponential backoff (from 500ms up to 30s).
pub fn spawn_supervised_loop<F>(name: &'static str, mut loop_fn: F)
where
    F: FnMut() + Send + 'static,
{
    let handle = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let mut backoff = std::time::Duration::from_millis(500);
            let max_backoff = std::time::Duration::from_secs(30);

            loop {
                let start_time = std::time::Instant::now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&mut loop_fn));

                match result {
                    Ok(()) => {
                        log::info!("Supervised worker thread '{name}' terminated normally.");
                        break;
                    }
                    Err(panic_err) => {
                        let panic_msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                            *s
                        } else if let Some(s) = panic_err.downcast_ref::<String>() {
                            s.as_str()
                        } else {
                            "non-string panic payload"
                        };

                        log::error!(
                            "Supervised worker thread '{name}' crashed with panic: {panic_msg}. Restarting in {backoff:?}..."
                        );

                        if start_time.elapsed() > std::time::Duration::from_secs(60) {
                            backoff = std::time::Duration::from_millis(500);
                        }

                        std::thread::sleep(backoff);
                        backoff = (backoff * 2).min(max_backoff);
                    }
                }
            }
        });

    if let Err(err) = handle {
        log::error!("Failed to spawn supervised worker thread '{name}': {err}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    services::logger::init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                vis.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            if let Some(window) = app.get_webview_window("main") {
                if window.show().is_ok() {
                    let _ = window.set_focus();
                    let _ = sync_tray_visibility(app, true);
                    let _ = app.emit("island-shown", ());
                }
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "ctx_pin_expanded" => {
                    if let Err(error) = app.emit("toggle-pin-expanded", ()) {
                        log::error!("Could not apply Pin Expanded Island: {error}");
                    }
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
                    if let Err(error) = app.emit("request-hide-island", ()) {
                        log::error!("Could not apply Hide Island: {error}");
                    }
                }
                "ctx_dismiss" => {
                    if let Err(error) = app.emit("collapse-island", ()) {
                        log::error!("Could not apply Dismiss: {error}");
                    }
                }
                "ctx_test_privacy" => {
                    if let Err(error) = commands::trigger_privacy_test(app.clone()) {
                        log::error!("Could not run Privacy Light test: {error}");
                    }
                }
                "ctx_timer_5" => {
                    if let Err(error) = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 300 })) {
                        log::error!("Could not start the 5 minute timer: {error}");
                    }
                }
                "ctx_timer_15" => {
                    if let Err(error) = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 900 })) {
                        log::error!("Could not start the 15 minute timer: {error}");
                    }
                }
                "ctx_timer_25" => {
                    if let Err(error) = app.emit("timer-command", serde_json::json!({ "action": "start", "durationSecs": 1500 })) {
                        log::error!("Could not start the 25 minute timer: {error}");
                    }
                }
                "ctx_timer_stop" => {
                    if let Err(error) = app.emit("timer-command", serde_json::json!({ "action": "stop" })) {
                        log::error!("Could not stop the timer: {error}");
                    }
                }
                "ctx_settings" | "settings" => {
                    if let Err(error) = open_settings_window(app.clone()) {
                        log::error!("Could not open ORBIT Settings from the menu: {error}");
                    }
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
                                if main_win.show().is_ok() {
                                    let _ = main_win.set_focus();
                                    let _ = sync_tray_visibility(app, true);
                                    let _ = app.emit("island-shown", ());
                                }
                            }
                        }
                    }
                }
                "hide_tray_icon" => {
                    let mut cfg = load_config(app);
                    cfg.show_system_tray_icon = false;
                    match services::config::save_config(app, &cfg) {
                        Ok(()) => {
                            if let Some(main_win) = app.get_webview_window("main") {
                                if !main_win.is_visible().unwrap_or(false) {
                                    if let Some(vis) = app.try_state::<std::sync::Arc<IslandVisibilityState>>() {
                                        vis.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                                        vis.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                                    }
                                    if main_win.show().is_ok() {
                                        let _ = main_win.set_focus();
                                        let _ = app.emit("island-shown", ());
                                    }
                                }
                                if let Err(error) = sync_tray_visibility(app, main_win.is_visible().unwrap_or(false)) {
                                    log::error!("Failed to apply tray preference: {error}");
                                }
                            }
                            let _ = app.emit("config-changed", &cfg);
                        }
                        Err(error) => log::error!("Failed to save system tray visibility setting: {error}"),
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
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            } else if window.label() == "main" {
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
            // Initialize in-memory configuration cache from disk once on application boot
            let initial_config = load_config(&app.handle());
            services::config::sync_startup_registry(initial_config.launch_at_startup);
            let config_store = std::sync::Arc::new(ConfigStore::new(initial_config.clone()));
            app.manage(config_store.clone());

            let hotkey_thread_state = HotkeyState::default();
            app.manage(hotkey_thread_state.clone());

            let cs_listener = config_store.clone();
            #[cfg(target_os = "windows")]
            let hk_listener = hotkey_thread_state.clone();
            app.listen("config-changed", move |event| {
                if let Ok(cfg) = serde_json::from_str::<services::config::AppConfig>(event.payload()) {
                    cs_listener.update(cfg.clone());
                    #[cfg(target_os = "windows")]
                    {
                        let tid = hk_listener.thread_id.load(std::sync::atomic::Ordering::SeqCst);
                        if tid != 0 {
                            unsafe {
                                windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                                    tid,
                                    WM_ORBIT_UPDATE_HOTKEYS,
                                    if cfg.shortcuts_enabled { 1 } else { 0 },
                                    0,
                                );
                            }
                        }
                    }
                }
            });

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
            let hide_tray_item = MenuItem::with_id(
                app,
                "hide_tray_icon",
                "Hide System Tray Icon",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit ORBIT", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &toggle_item, &hide_tray_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::with_id("orbit-tray");
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }
            let tray = tray_builder
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
                            vis.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        if let Some(main_win) = app.get_webview_window("main") {
                            if main_win.show().is_ok() {
                                let _ = main_win.set_focus();
                                if let Err(error) = sync_tray_visibility(&app, true) {
                                    log::error!("Could not restore tray preference after showing island: {error}");
                                }
                                let _ = app.emit("island-shown", ());
                            }
                        }
                    }
                })
                .build(app)?;

            if let Err(error) = tray.set_visible(initial_config.show_system_tray_icon) {
                log::warn!("Failed to apply system tray visibility setting: {error}");
            }

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
                let config_store_hk = config_store.clone();
                let hk_thread_id = hotkey_thread_state.thread_id.clone();

                spawn_supervised_loop("orbit-hotkey-manager", move || {
                        use windows_sys::Win32::Foundation::HWND;
                        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                            RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT,
                        };
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            GetMessageW, PeekMessageW, MSG, PM_NOREMOVE, WM_HOTKEY, WM_QUIT,
                        };

                        let thread_id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
                        hk_thread_id.store(thread_id, std::sync::atomic::Ordering::SeqCst);

                        // Ensure thread message queue exists before any PostThreadMessageW calls
                        let mut init_msg: MSG = unsafe { std::mem::zeroed() };
                        unsafe {
                            PeekMessageW(&mut init_msg, 0 as HWND, 0, 0, PM_NOREMOVE);
                        }

                        let mut registered = false;
                        // ID 1 = Alt+O (Expand/Collapse), ID 2 = Alt+H (Hide/Show), ID 3 = Alt+S (Stealth Capture), ID 4 = Alt+M (Play/Pause Media)
                        let hotkeys: [(i32, u32); 4] = [
                            (1, b'O' as u32),
                            (2, b'H' as u32),
                            (3, b'S' as u32),
                            (4, b'M' as u32),
                        ];

                        let initial_enable = vis_hk
                            .shortcuts_enabled
                            .load(std::sync::atomic::Ordering::Relaxed);
                        if initial_enable {
                            unsafe {
                                for &(id, vk) in &hotkeys {
                                    let res = RegisterHotKey(0 as HWND, id, MOD_ALT | MOD_NOREPEAT, vk);
                                    if res == 0 {
                                        log::warn!("RegisterHotKey failed for id {id} vk {vk}");
                                    }
                                }
                            }
                            registered = true;
                        }

                        let mut msg: MSG = unsafe { std::mem::zeroed() };
                        while unsafe { GetMessageW(&mut msg, 0 as HWND, 0, 0) } > 0 {
                            match msg.message {
                                WM_HOTKEY => {
                                    let id = msg.wParam as i32;
                                    services::logger::log_event(
                                        "DEBUG",
                                        "hotkey",
                                        &format!("Hotkey msg.wParam: {}", id),
                                    );
                                    match id {
                                        1 => {
                                            // Alt+O: Expand / Collapse Island
                                            if let Some(main_win) = app_handle_hk.get_webview_window("main") {
                                                if !main_win.is_visible().unwrap_or(true) {
                                                    vis_hk.user_manually_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                                                    vis_hk.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                                                    if main_win.show().is_ok() {
                                                        let _ = sync_tray_visibility(&app_handle_hk, true);
                                                        let _ = app_handle_hk.emit("island-shown", ());
                                                    }
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
                                                        vis_hk.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                                                        if main_win.show().is_ok() {
                                                            let _ = main_win.set_focus();
                                                            let _ = sync_tray_visibility(&app_handle_hk, true);
                                                            let _ = app_handle_hk.emit("island-shown", ());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        3 => {
                                            // Alt+S: Toggle Screen-Share / Recording Stealth
                                            let mut cfg = config_store_hk.get();
                                            cfg.hide_from_screen_capture = !cfg.hide_from_screen_capture;
                                            if let Some(main_win) = app_handle_hk.get_webview_window("main") {
                                                services::window::set_window_capture_stealth(
                                                    &main_win,
                                                    cfg.hide_from_screen_capture,
                                                );
                                            }
                                            config_store_hk.update(cfg.clone());
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
                                m if m == WM_ORBIT_UPDATE_HOTKEYS => {
                                    let should_enable = msg.wParam != 0;
                                    if should_enable && !registered {
                                        unsafe {
                                            for &(id, vk) in &hotkeys {
                                                let res = RegisterHotKey(0 as HWND, id, MOD_ALT | MOD_NOREPEAT, vk);
                                                if res == 0 {
                                                    log::warn!("RegisterHotKey failed for id {id} vk {vk}");
                                                }
                                            }
                                        }
                                        registered = true;
                                    } else if !should_enable && registered {
                                        unsafe {
                                            for &(id, _) in &hotkeys {
                                                let _ = UnregisterHotKey(0 as HWND, id);
                                            }
                                        }
                                        registered = false;
                                    }
                                }
                                WM_QUIT => {
                                    if registered {
                                        unsafe {
                                            for &(id, _) in &hotkeys {
                                                let _ = UnregisterHotKey(0 as HWND, id);
                                            }
                                        }
                                    }
                                    break;
                                }
                                _ => {}
                            }
                        }
                });
            }

            // Manage media state
            let media_state = MediaState::default();
            app.manage(media_state.clone());

            // Manage SQLite Database for reminders and file shelf
            let db_manager = std::sync::Arc::new(
                services::db::DatabaseManager::for_app(&app.handle()).unwrap_or_else(|err| {
                    log::error!("DatabaseManager::for_app failed: {err}; creating in-memory fallback instance");
                    services::db::DatabaseManager::in_memory().expect("in-memory db must succeed")
                }),
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
            let config_store_fs = config_store.clone();
            spawn_supervised_loop("orbit-fullscreen-monitor", move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    if let Some(main_win) = app_handle_fs.get_webview_window("main") {
                        if let Ok(hwnd) = main_win.hwnd() {
                            let is_fs = services::window::is_foreground_fullscreen(hwnd.0 as isize);
                            let auto_suppressed = vis_fs.auto_suppressed_fullscreen.load(std::sync::atomic::Ordering::Relaxed);
                            let manually_hidden = vis_fs.user_manually_hidden.load(std::sync::atomic::Ordering::Relaxed);
                            let cfg = config_store_fs.get();
                            let should_pause = cfg.pause_in_fullscreen;

                            if is_fs && should_pause && !auto_suppressed && !manually_hidden {
                                services::logger::log_event("DEBUG", "fullscreen", "Auto-suppressing window due to fullscreen");
                                vis_fs.auto_suppressed_fullscreen.store(true, std::sync::atomic::Ordering::SeqCst);
                                let _ = app_handle_fs.emit("fullscreen-state-changed", true);
                                if let Ok(is_vis) = main_win.is_visible() {
                                    if is_vis {
                                        if main_win.hide().is_ok() {
                                            if let Err(error) = sync_tray_visibility(&app_handle_fs, false) {
                                                log::error!("Could not show tray icon while island is hidden: {error}");
                                            }
                                        }
                                    }
                                }
                            } else if (!is_fs || !should_pause) && auto_suppressed && !manually_hidden {
                                vis_fs.auto_suppressed_fullscreen.store(false, std::sync::atomic::Ordering::SeqCst);
                                let _ = app_handle_fs.emit("fullscreen-state-changed", false);
                                let _ = main_win.set_always_on_top(cfg.always_on_top);
                                services::window::set_window_topmost(&main_win, cfg.always_on_top);
                                if main_win.show().is_ok() {
                                    if let Err(error) = sync_tray_visibility(&app_handle_fs, true) {
                                        log::error!("Could not restore tray preference after fullscreen: {error}");
                                    }
                                    let _ = app_handle_fs.emit("island-shown", ());
                                }
                            } else if !is_fs && !auto_suppressed && !manually_hidden {
                                if cfg.always_on_top {
                                    services::window::set_window_topmost(&main_win, true);
                                }
                            }
                        }
                    }
                }
            });

            // Spawn lightweight background thread to poll media session changes
            let app_handle_media = app.handle().clone();
            let monitor_media_state = media_state.clone();
            spawn_supervised_loop("orbit-media-monitor", move || {
                let _com = match services::media::ComGuard::new() {
                    Ok(g) => g,
                    Err(err) => {
                        log::error!("Failed to initialize COM for media monitor thread: {err}");
                        std::thread::sleep(std::time::Duration::from_secs(2));
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
            });

            // Spawn real-time WASAPI audio peak/spectrum monitor for frequency-reactive music waveforms
            let app_handle_spectrum = app.handle().clone();
            spawn_supervised_loop("orbit-audio-spectrum", move || {
                use std::collections::VecDeque;
                use std::time::Instant;

                let _com = match services::media::ComGuard::new() {
                    Ok(g) => g,
                    Err(err) => {
                        log::error!("Failed to initialize COM for audio spectrum thread: {err}");
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        return;
                    }
                };
                let mut prev_peak = 0.0f32;
                let mut smooth_low = 0.0f32;
                let mut phase = 0.0f32;
                let mut was_active = false;
                let mut loopback: Option<(
                    services::loopback::LoopbackCapture,
                    services::spectrum_dsp::SpectrumAnalyzer,
                )> = None;
                let mut ring = VecDeque::with_capacity(4096);
                let mut retry_loopback_at = Instant::now();
                let mut fft_mode = false;
                let mut mode_known = false;

                loop {
                    if loopback.is_none() && Instant::now() >= retry_loopback_at {
                        match services::loopback::LoopbackCapture::new() {
                            Ok(capture) => {
                                let analyzer = services::spectrum_dsp::SpectrumAnalyzer::new(
                                    capture.sample_rate(),
                                );
                                loopback = Some((capture, analyzer));
                                ring.clear();
                                if !mode_known || !fft_mode {
                                    log::info!("spectrum mode: fft-loopback");
                                    fft_mode = true;
                                }
                                mode_known = true;
                            }
                            Err(err) => {
                                if !mode_known || fft_mode {
                                    log::info!("spectrum mode: meter-fallback ({err})");
                                }
                                fft_mode = false;
                                mode_known = true;
                                retry_loopback_at = Instant::now()
                                    + std::time::Duration::from_secs(5);
                            }
                        }
                    }

                    let spectrum = if let Some((capture, analyzer)) = loopback.as_mut() {
                        match capture.read_into(&mut ring) {
                            Ok(()) => {
                                let samples = ring.make_contiguous();
                                let start = samples.len().saturating_sub(2048);
                                let recent = &samples[start..];
                                let rms = if recent.is_empty() {
                                    0.0
                                } else {
                                    (recent.iter().map(|sample| sample * sample).sum::<f32>()
                                        / recent.len() as f32)
                                        .sqrt()
                                };
                                let (wave_bars, compact_bars) = analyzer.analyze(samples);
                                if rms < 1e-4 {
                                    services::media::AudioSpectrumState::default()
                                } else {
                                    services::media::AudioSpectrumState {
                                        peak: wave_bars.iter().copied().fold(0.0, f32::max),
                                        compact_bars,
                                        wave_bars,
                                    }
                                }
                            }
                            Err(err) => {
                                log::warn!("WASAPI loopback capture stopped: {err}");
                                loopback = None;
                                ring.clear();
                                retry_loopback_at = Instant::now()
                                    + std::time::Duration::from_secs(1);
                                log::info!("spectrum mode: meter-fallback");
                                fft_mode = false;
                                mode_known = true;
                                services::media::sample_audio_spectrum_meter(
                                    &mut prev_peak,
                                    &mut smooth_low,
                                    &mut phase,
                                )
                            }
                        }
                    } else {
                        services::media::sample_audio_spectrum_meter(
                            &mut prev_peak,
                            &mut smooth_low,
                            &mut phase,
                        )
                    };
                    let is_active = spectrum.peak > 0.001;
                    if is_active || was_active {
                        was_active = is_active;
                        let _ = app_handle_spectrum.emit("media-audio-spectrum", &spectrum);
                    }
                    let delay = if is_active { 33 } else { 220 };
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
            });

            // Start the Media Foundation sensor monitor off the launch path so it cannot delay
            // the first visible Island frame.
            if let Err(error) = std::thread::Builder::new()
                .name("orbit-camera-monitor-init".into())
                .spawn(services::privacy::init_camera_monitor)
            {
                log::error!("Failed to spawn camera monitor initializer: {error}");
            }

            // Spawn real-time background thread to continuously inspect camera & mic privacy indicators
            let app_handle_privacy = app.handle().clone();
            spawn_supervised_loop("orbit-privacy-monitor", move || {
                let _com = match services::media::ComGuard::new() {
                    Ok(g) => g,
                    Err(err) => {
                        log::error!("Failed to initialize COM for privacy monitor thread: {err}");
                        std::thread::sleep(std::time::Duration::from_secs(2));
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
            });

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
            handle_file_drop,
            share_files,
            remove_shelf_item,
            open_shelf_file,
            show_shelf_file_in_folder,
            pick_shelf_file,
            clear_shelf,
            open_external_url,
            clear_local_data,
            activate_app_source,
            reveal_diagnostic_log,
            get_live_backdrop,
            get_weather
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    services::logger::log_event("INFO", "system", "ORBIT run loop ended normally");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn hidden_island_always_has_a_tray_restore_path() {
        assert!(tray_visibility_for_state(false, false));
        assert!(tray_visibility_for_state(true, false));
        assert!(tray_visibility_for_state(true, true));
        assert!(!tray_visibility_for_state(false, true));
    }

    #[test]
    fn test_spawn_supervised_loop_recovers_from_panic() {
        let execution_count = Arc::new(AtomicUsize::new(0));
        let count_clone = execution_count.clone();

        spawn_supervised_loop("test-supervised-worker", move || {
            let attempt = count_clone.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                panic!("Simulated worker panic on attempt {attempt}");
            }
            // On attempt >= 2, exit normally
        });

        let start = std::time::Instant::now();
        while execution_count.load(Ordering::SeqCst) < 2 && start.elapsed() < std::time::Duration::from_secs(3) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(
            execution_count.load(Ordering::SeqCst) >= 2,
            "Supervised worker thread did not recover from panic!"
        );
    }
}

