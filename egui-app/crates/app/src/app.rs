// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/app.rs.

//! The eframe app: owns the store, tray, watcher, hit-tester, background services,
//! and keeps the Win32 overlay window synchronized with the rendered pill shape.

use crate::tray::{Tray, TrayCommand};
use crate::watcher::{self, Watcher};
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, Rect, TextureHandle, TextureId, TextureOptions, Ui, pos2};
use orbit_core::config::{Config, GlassMode, TimeFormat};
use orbit_core::events::{ClockStatus, CoreEvent, FocusTimerState, WakeFn};
use orbit_core::store::AppStore;
use orbit_platform::backdrop::{Backdrop, BackdropColors, BackdropRect, blur_sigma_for_mode};
use orbit_platform::fullscreen::FullscreenWatcher;
use orbit_platform::hittest::{HitShape, HitTester};
use orbit_platform::single_instance::InstanceGuard;
use orbit_platform::{Hwnd, fonts, material, monitor, process, system, window};
use orbit_services::calendar::CalendarService;
use orbit_services::db::DatabaseManager;
use orbit_services::hardware::HardwareService;
use orbit_services::media::MediaService;
use orbit_services::notifications::{self, NotificationService};
use orbit_services::privacy::PrivacyService;
use orbit_services::shelf::ShelfService;
use orbit_services::spectrum::SpectrumService;
use orbit_services::thumbnails::{
    ThumbnailCache, ThumbnailRgba, decode_data_uri_rgba, is_supported_image_ext, load_thumbnail_rgba,
};
use orbit_services::weather::WeatherService;
use orbit_ui::anim::{self, Motion};
use orbit_ui::geometry::Squircle;
use orbit_ui::panel::{
    self, PanelAction, PanelState, SectionKind, UtilityAlertKind, UtilityToast,
};
use orbit_ui::pill::WINDOW_HEIGHT;
use orbit_ui::settings::{self, SettingsAction, SettingsUiState};
use orbit_ui::theme::{self, FontAssets, Theme};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type PushedShape = (Vec<(f32, f32)>, Option<Rect>);

enum SettingsAsyncEvent {
    NotifAccessStatus(String),
    NotifAccessFinished(Result<String, String>),
    DataCleared(Result<(), String>),
    MemoryMeasured(Option<f32>),
    MemoryTrimmed {
        trimmed_procs: u32,
        freed_mb: f32,
        current_mb: Option<f32>,
        metrics: orbit_core::events::HardwareMetrics,
    },
}

#[derive(Default)]
struct SettingsShared {
    is_open: bool,
    cfg: Config,
    system_light: bool,
    os_transparency: bool,
    ui_state: Option<SettingsUiState>,
    pending_actions: Vec<SettingsAction>,
    close_requested: bool,
    #[cfg(feature = "bughunt-probe")]
    open_started_at: Option<std::time::Instant>,
    #[cfg(feature = "bughunt-probe")]
    open_start_counters: orbit_platform::process::ProcessCounters,
    #[cfg(feature = "bughunt-probe")]
    first_frame_logged: bool,
    #[cfg(feature = "bughunt-probe")]
    frames_since_open: u32,
    #[cfg(feature = "bughunt-probe")]
    prev_frame_at: Option<std::time::Instant>,
}

fn settings_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("orbit-settings")
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn to_clock_status(c: system::Clock) -> ClockStatus {
    ClockStatus {
        time: c.time,
        time_digits: c.time_digits,
        ampm: c.ampm,
        date: c.date,
        full_date: c.full_date,
        year: c.year,
        month: c.month,
        day: c.day,
        weekday: c.weekday,
        millis_to_next_minute: c.millis_to_next_minute,
    }
}

pub struct OrbitApp {
    hwnd: Hwnd,
    backdrop: Option<Backdrop>,
    glass_override: Option<GlassMode>,
    store: AppStore,
    config_shared: Arc<RwLock<Config>>,
    theme: Theme,
    panel: PanelState,
    tray: Option<Tray>,
    tray_rx: Receiver<TrayCommand>,
    watcher: Option<Watcher>,
    fullscreen: Option<FullscreenWatcher>,
    hardware: Option<Arc<HardwareService>>,
    weather: Option<WeatherService>,
    media: Option<Arc<MediaService>>,
    privacy: Option<PrivacyService>,
    spectrum: Option<SpectrumService>,
    notifications: Option<Arc<NotificationService>>,
    db: Arc<DatabaseManager>,
    calendar: Arc<CalendarService>,
    shelf: Arc<ShelfService>,
    thumb_cache: ThumbnailCache,
    thumb_textures: HashMap<String, TextureHandle>,
    thumb_tex_ids: HashMap<String, TextureId>,
    thumb_in_flight: HashSet<String>,
    thumb_tx: Sender<(String, Option<ThumbnailRgba>)>,
    thumb_rx: Receiver<(String, Option<ThumbnailRgba>)>,
    media_art_key: Option<String>,
    media_art_tex: Option<TextureHandle>,
    shelf_msg_tx: Sender<(String, bool)>,
    shelf_msg_rx: Receiver<(String, bool)>,
    drop_msg_tx: Sender<(String, bool)>,
    drop_msg_rx: Receiver<(String, bool)>,
    boost_tx: Sender<(u32, f32, orbit_core::events::HardwareMetrics)>,
    boost_rx: Receiver<(u32, f32, orbit_core::events::HardwareMetrics)>,
    settings_open: bool,
    settings_created: bool,
    settings_shared: Arc<Mutex<SettingsShared>>,
    settings_msg_tx: Sender<SettingsAsyncEvent>,
    settings_msg_rx: Receiver<SettingsAsyncEvent>,
    ev_tx: Sender<CoreEvent>,
    ev_rx: Receiver<CoreEvent>,
    wake: WakeFn,
    hit: Option<HitTester>,
    pushed: Option<PushedShape>,
    shape_dirty: bool,
    was_inside: bool,
    applied_on_top: Option<bool>,
    applied_stealth: Option<bool>,
    frames: u32,
    collapsed_since: Option<f64>,
    trimmed: bool,
    anchor: Option<(i32, i32, i32, i32)>,
    last_rec_check: Option<std::time::Instant>,
    rec_bar_bottom: Option<i32>,
    prev_ac_power: Option<bool>,
    prev_wifi: bool,
    prev_bt: bool,
    prev_location: bool,
    privacy_initialized: bool,
    _instance: InstanceGuard,
    #[cfg(feature = "bughunt-probe")]
    bughunt: crate::bughunt::BughuntProbe,
    #[cfg(feature = "bughunt-probe")]
    bughunt_panel_was_hovered: bool,
    #[cfg(feature = "debug-overlay")]
    overlay: crate::overlay::DebugOverlay,
}

impl OrbitApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: Config,
        instance: InstanceGuard,
        hidden: bool,
        glass_override: Option<GlassMode>,
    ) -> Result<Self> {
        let ctx = cc.egui_ctx.clone();
        let segoe = fonts::load_segoe_ui();
        theme::install_fonts(
            &ctx,
            FontAssets {
                regular: segoe.regular,
                semibold: segoe.semibold,
                bold: segoe.bold,
            },
        );
        log_gl(cc);

        let hwnd = match cc.window_handle().context("window handle")?.as_raw() {
            RawWindowHandle::Win32(h) => Hwnd::from_raw(h.hwnd.get()),
            other => anyhow::bail!("unexpected window handle {other:?}"),
        };
        window::ensure_overlay(hwnd, false).context("overlay window styles")?;
        if let Err(e) = window::fill_client(hwnd) {
            tracing::warn!("frameless client area: {e}");
        }
        material::prepare(hwnd, true);
        window::set_opacity(hwnd, 0).context("initial window opacity")?;
        let backdrop = Backdrop::new(hwnd)
            .inspect_err(|e| tracing::warn!("Composition backdrop unavailable, using OpenGL fallback: {e}"))
            .ok();

        let repaint = ctx.clone();
        let hit = match HitTester::spawn(move |_| repaint.request_repaint_of(egui::ViewportId::ROOT)) {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::error!("hit testing unavailable: {e}");
                None
            }
        };

        let (tray_tx, tray_rx) = unbounded();
        let tray = match Tray::new(tray_tx, ctx.clone()) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::error!("tray icon unavailable: {e:#}");
                None
            }
        };

        let initial_sys = watcher::sample();
        let prev_ac_power = initial_sys.battery.map(|b| b.charging);
        let prev_wifi = initial_sys.wifi_connected;
        let prev_bt = initial_sys.bluetooth_enabled;
        let initial_clock =
            to_clock_status(system::clock_with_format(cfg.time_format == TimeFormat::Hour24));
        let initial_theme =
            Theme::for_config(cfg.theme, cfg.color_theme, initial_sys.light_theme);
        let config_shared = Arc::new(RwLock::new(cfg.clone()));
        let mut store = AppStore::new(cfg.clone(), initial_sys, initial_clock, hidden);

        let (ev_tx, ev_rx) = unbounded();
        let wake: WakeFn = Arc::new({
            let ctx = ctx.clone();
            move || ctx.request_repaint_of(egui::ViewportId::ROOT)
        });

        let watcher = Watcher::spawn(ev_tx.clone(), ctx.clone()).context("spawning watcher")?;

        let fullscreen = match FullscreenWatcher::spawn_with_recording_wake(
            hwnd,
            {
                let tx = ev_tx.clone();
                let w = wake.clone();
                move |is_fs| {
                    let _ = tx.send(CoreEvent::FullscreenChanged(is_fs));
                    w();
                }
            },
            {
                let w = wake.clone();
                move || w()
            },
        ) {
            Ok(fw) => Some(fw),
            Err(e) => {
                tracing::warn!("fullscreen watcher unavailable: {e}");
                None
            }
        };

        let db = Arc::new(DatabaseManager::default_app_db().unwrap_or_else(|e| {
            tracing::error!("DatabaseManager::default_app_db failed: {e}; using in-memory fallback");
            DatabaseManager::in_memory().expect("in-memory db must succeed")
        }));
        if let Ok(reminders) = db.get_reminders() {
            store.apply(CoreEvent::RemindersUpdated(reminders));
        }
        if let Ok(shelf_items) = db.get_shelf_items() {
            store.apply(CoreEvent::ShelfUpdated(shelf_items));
        }

        let hardware = HardwareService::spawn(ev_tx.clone(), wake.clone())
            .inspect_err(|e| tracing::warn!("hardware service unavailable: {e}"))
            .ok()
            .map(Arc::new);
        let weather = WeatherService::spawn(
            ev_tx.clone(),
            wake.clone(),
            Arc::new(AtomicBool::new(cfg.weather_enabled)),
        )
        .inspect_err(|e| tracing::warn!("weather service unavailable: {e}"))
        .ok();

        let playing_flag = Arc::new(AtomicBool::new(false));
        let spectrum_visible = Arc::new(AtomicBool::new(false));
        let spectrum_park = Arc::new((Mutex::new(()), Condvar::new()));

        let media = MediaService::spawn(
            ev_tx.clone(),
            wake.clone(),
            playing_flag.clone(),
            spectrum_park.clone(),
        )
        .inspect_err(|e| tracing::warn!("media service unavailable: {e}"))
        .ok()
        .map(Arc::new);

        let spectrum = SpectrumService::spawn(
            ev_tx.clone(),
            wake.clone(),
            playing_flag,
            spectrum_visible,
            spectrum_park,
        )
        .inspect_err(|e| tracing::warn!("spectrum service unavailable: {e}"))
        .ok();

        let privacy = PrivacyService::spawn(
            ev_tx.clone(),
            wake.clone(),
            Arc::new(AtomicBool::new(cfg.show_privacy_indicators)),
        )
        .inspect_err(|e| tracing::warn!("privacy service unavailable: {e}"))
        .ok();

        let notifications =
            NotificationService::spawn(ev_tx.clone(), wake.clone(), config_shared.clone())
                .inspect_err(|e| tracing::warn!("notification service unavailable: {e}"))
                .ok()
                .map(Arc::new);

        let calendar = Arc::new(CalendarService::new(db.clone(), ev_tx.clone(), wake.clone()));
        let shelf = Arc::new(ShelfService::new(
            db.clone(),
            config_shared.clone(),
            ev_tx.clone(),
            wake.clone(),
        ));
        let (thumb_tx, thumb_rx) = unbounded();
        let (shelf_msg_tx, shelf_msg_rx) = unbounded();
        let (drop_msg_tx, drop_msg_rx) = unbounded();
        let (boost_tx, boost_rx) = unbounded();
        let (settings_msg_tx, settings_msg_rx) = unbounded();

        Ok(Self {
            hwnd,
            backdrop,
            glass_override,
            store,
            config_shared,
            theme: initial_theme,
            panel: PanelState::default(),
            tray,
            tray_rx,
            watcher: Some(watcher),
            fullscreen,
            hardware,
            weather,
            media,
            privacy,
            spectrum,
            notifications,
            db,
            calendar,
            shelf,
            thumb_cache: ThumbnailCache::default(),
            thumb_textures: HashMap::new(),
            thumb_tex_ids: HashMap::new(),
            thumb_in_flight: HashSet::new(),
            thumb_tx,
            thumb_rx,
            media_art_key: None,
            media_art_tex: None,
            shelf_msg_tx,
            shelf_msg_rx,
            drop_msg_tx,
            drop_msg_rx,
            boost_tx,
            boost_rx,
            settings_open: false,
            settings_created: false,
            settings_shared: Arc::new(Mutex::new(SettingsShared::default())),
            settings_msg_tx,
            settings_msg_rx,
            ev_tx,
            ev_rx,
            wake,
            hit,
            pushed: None,
            shape_dirty: true,
            was_inside: false,
            applied_on_top: None,
            applied_stealth: None,
            frames: 0,
            collapsed_since: None,
            trimmed: false,
            anchor: None,
            last_rec_check: None,
            rec_bar_bottom: None,
            prev_ac_power,
            prev_wifi,
            prev_bt,
            prev_location: false,
            privacy_initialized: false,
            _instance: instance,
            #[cfg(feature = "bughunt-probe")]
            bughunt: crate::bughunt::BughuntProbe::new(crate::bughunt::parse_mode()),
            #[cfg(feature = "bughunt-probe")]
            bughunt_panel_was_hovered: false,
            #[cfg(feature = "debug-overlay")]
            overlay: Default::default(),
        })
    }

    fn anchor_window(&mut self) {
        let work = monitor::primary_work_area();
        let scale = window::dpi(self.hwnd) as f32 / 96.0;
        let h = (WINDOW_HEIGHT * scale).ceil() as i32;
        if self.store.config().placement == orbit_core::config::Placement::Center {
            let refresh = self
                .last_rec_check
                .is_none_or(|t| t.elapsed() >= Duration::from_millis(400));
            if refresh {
                self.last_rec_check = Some(std::time::Instant::now());
                self.rec_bar_bottom = window::top_center_recording_bar_bottom(
                    self.hwnd,
                    self.backdrop.as_ref().map(Backdrop::hwnd),
                );
            }
        } else {
            self.rec_bar_bottom = None;
        }
        let top_y = if let Some(rec_b) = self.rec_bar_bottom {
            (rec_b + (4.0 * scale).round() as i32).clamp(work.top, work.top + 160)
        } else {
            work.top
        };
        let rect = (work.left, top_y, work.width(), h);
        if self.anchor != Some(rect) {
            match window::set_rect(self.hwnd, rect.0, rect.1, rect.2, rect.3) {
                Ok(()) => {
                    if let Some(b) = self.backdrop.as_mut() {
                        b.set_bounds(rect.0, rect.1, rect.2, rect.3);
                    }
                    self.anchor = Some(rect);
                    self.shape_dirty = true;
                }
                Err(e) => tracing::warn!("anchoring window: {e}"),
            }
        }
    }

    fn apply_layer(&mut self) {
        let on_top = self.store.config().on_top;
        let backdrop_hwnd = self.backdrop.as_ref().map(Backdrop::hwnd);
        let drift = window::is_topmost(self.hwnd) != on_top
            || backdrop_hwnd.is_some_and(|bh| window::is_topmost(bh) != on_top);
        if self.applied_on_top == Some(on_top) && !drift {
            return;
        }
        match window::set_layer(self.hwnd, backdrop_hwnd, on_top) {
            Ok(()) => {
                self.applied_on_top = Some(on_top);
            }
            Err(e) => tracing::warn!("window layer: {e}"),
        }
    }

    fn apply_stealth(&mut self) {
        let stealth = self.store.config().hide_from_screen_capture;
        if self.applied_stealth == Some(stealth) {
            return;
        }
        if let Err(e) = window::set_capture_stealth(self.hwnd, stealth) {
            tracing::warn!("capture stealth: {e}");
        } else {
            if let Some(b) = &self.backdrop {
                b.set_stealth(stealth);
            }
            self.applied_stealth = Some(stealth);
        }
    }

    fn apply_click_through(&self) {
        let _ = window::ensure_client_fills(self.hwnd);
        let through = !self.hit.as_ref().is_none_or(HitTester::inside);
        if let Err(e) = window::set_click_through(self.hwnd, through) {
            tracing::warn!("click-through: {e}");
        }
    }

    fn update_shape(&mut self, ctx: &egui::Context, shape: Option<Squircle>, extra: Option<Rect>) {
        let ppp = ctx.pixels_per_point();
        let outline: Vec<(f32, f32)> = shape
            .map(|s| s.outline().iter().map(|p| (p.x * ppp, p.y * ppp)).collect())
            .unwrap_or_default();
        let key = (outline, extra);
        if self.pushed.as_ref() == Some(&key) && !self.shape_dirty {
            return;
        }
        let (outline, extra) = key;
        if let Some(hit) = &self.hit {
            let (ox, oy) = self
                .anchor
                .map(|(x, y, _, _)| (x as f32, y as f32))
                .unwrap_or_default();
            let mut polygons = Vec::with_capacity(2);
            if let Some(s) = shape {
                let padded =
                    Squircle::new(s.rect.expand(1.0), s.radius + 1.0).with_exponent(s.exponent);
                polygons.push(
                    padded
                        .outline()
                        .iter()
                        .map(|p| (ox + p.x * ppp, oy + p.y * ppp))
                        .collect(),
                );
            }
            if let Some(r) = extra {
                let (x0, y0) = (ox + r.min.x * ppp, oy + r.min.y * ppp);
                let (x1, y1) = (ox + r.max.x * ppp, oy + r.max.y * ppp);
                polygons.push(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            }
            hit.set_shape(HitShape { polygons });
        }
        self.pushed = Some((outline, extra));
        self.shape_dirty = false;
    }

    /// Calls `K32EmptyWorkingSet` on this process ONLY after 3 s of idle,
    /// never while the pointer is over the island, and never schedules extra
    /// repaints once already trimmed until the pointer interacts again.
    fn trim_when_idle(&mut self, ctx: &egui::Context, hovered: bool) {
        const AFTER: f64 = 3.0;
        if hovered {
            self.collapsed_since = None;
            self.trimmed = false;
            return;
        }
        if self.trimmed {
            return;
        }
        let now = ctx.input(|i| i.time);
        let since = *self.collapsed_since.get_or_insert(now);
        let elapsed = now - since;
        if elapsed >= AFTER {
            if let Err(e) = process::trim_working_set() {
                tracing::debug!("trim_working_set: {e}");
            }
            self.trimmed = true;
        } else {
            ctx.request_repaint_after(Duration::from_secs_f64(AFTER - elapsed + 0.02));
        }
    }

    fn sync_service_gates(&self, suppressed: bool, current_section: SectionKind) {
        let visible_expanded = !suppressed && self.panel.expanded;
        if let Some(hw) = &self.hardware {
            hw.set_active(visible_expanded && current_section == SectionKind::Hardware);
        }
        if let Some(spec) = &self.spectrum {
            spec.set_visible(visible_expanded && current_section == SectionKind::Home);
        }
    }

    fn dispatch_actions(&mut self, ctx: &egui::Context, actions: Vec<PanelAction>) {
        let now_ms = now_unix_ms();
        let now_ui = ctx.input(|i| i.time);
        for action in actions {
            match action {
                PanelAction::StartFocusTimer(secs) => {
                    let timer = FocusTimerState {
                        label: "Focus".to_string(),
                        total_secs: secs,
                        deadline_unix_ms: Some(now_ms + secs * 1000),
                        paused_remaining_secs: None,
                    };
                    self.store.apply(CoreEvent::FocusTimerUpdated(timer));
                }
                PanelAction::TogglePauseFocusTimer => {
                    let cur = self.store.snapshot().focus_timer.clone();
                    if cur.is_running() {
                        let rem = cur.remaining_secs(now_ms);
                        self.store.apply(CoreEvent::FocusTimerUpdated(FocusTimerState {
                            label: cur.label,
                            total_secs: cur.total_secs,
                            deadline_unix_ms: None,
                            paused_remaining_secs: Some(rem.max(1)),
                        }));
                    } else if let Some(rem) = cur.paused_remaining_secs {
                        self.store.apply(CoreEvent::FocusTimerUpdated(FocusTimerState {
                            label: cur.label,
                            total_secs: cur.total_secs,
                            deadline_unix_ms: Some(now_ms + rem * 1000),
                            paused_remaining_secs: None,
                        }));
                    }
                }
                PanelAction::ResetFocusTimer => {
                    self.store
                        .apply(CoreEvent::FocusTimerUpdated(FocusTimerState::default()));
                }
                PanelAction::MediaCommand(cmd) => {
                    if let Some(media) = self.media.as_ref().map(Arc::clone) {
                        let tx = self.ev_tx.clone();
                        let wake = self.wake.clone();
                        let _ = std::thread::Builder::new()
                            .name("orbit-media-cmd".into())
                            .spawn(move || match media.send_command(cmd) {
                                Ok(updated) => {
                                    let _ = tx.send(CoreEvent::MediaUpdated(updated));
                                    wake();
                                }
                                Err(e) => tracing::debug!("media command '{cmd}' failed: {e}"),
                            });
                    }
                }
                PanelAction::MediaSeek(pos_secs) => {
                    if let Some(media) = self.media.as_ref().map(Arc::clone) {
                        let tx = self.ev_tx.clone();
                        let wake = self.wake.clone();
                        let _ = std::thread::Builder::new()
                            .name("orbit-media-seek".into())
                            .spawn(move || match media.seek(pos_secs) {
                                Ok(updated) => {
                                    let _ = tx.send(CoreEvent::MediaUpdated(updated));
                                    wake();
                                }
                                Err(e) => tracing::debug!("media seek failed: {e}"),
                            });
                    }
                }
                PanelAction::BoostRam => {
                    if let Some(hw) = self.hardware.as_ref().map(Arc::clone) {
                        let tx = self.boost_tx.clone();
                        let wake = self.wake.clone();
                        let _ = std::thread::Builder::new()
                            .name("orbit-ram-boost".into())
                            .spawn(move || {
                                let res = hw.free_up_memory();
                                let metrics = hw.sample_once();
                                let _ = tx.send((res.trimmed_processes, res.freed_mb, metrics));
                                wake();
                            });
                    }
                }
                PanelAction::AddReminder {
                    title,
                    due_timestamp_ms,
                } => {
                    let cal = Arc::clone(&self.calendar);
                    let _ = std::thread::Builder::new()
                        .name("orbit-cal-add".into())
                        .spawn(move || {
                            if let Err(e) = cal.add_reminder(&title, due_timestamp_ms, "task", "local")
                            {
                                tracing::warn!("add_reminder failed: {e}");
                            }
                        });
                }
                PanelAction::ToggleReminder(id) => {
                    let cal = Arc::clone(&self.calendar);
                    let _ = std::thread::Builder::new()
                        .name("orbit-cal-toggle".into())
                        .spawn(move || {
                            if let Err(e) = cal.toggle_reminder(&id) {
                                tracing::warn!("toggle_reminder failed: {e}");
                            }
                        });
                }
                PanelAction::ActivateNotification {
                    id,
                    app_id,
                    app_name,
                    title,
                } => {
                    self.panel.bubble.dismiss_active();
                    self.panel.return_expanded_after_bubble = false;
                    if let Some(svc) = self.notifications.as_ref().map(Arc::clone) {
                        let _ = std::thread::Builder::new()
                            .name("orbit-notif-open".into())
                            .spawn(move || {
                                if let Err(e) =
                                    window::activate_app_window(&app_id, &app_name, Some(&title))
                                {
                                    tracing::debug!("activate_app_window error: {e}");
                                }
                                svc.dismiss_notification(id);
                            });
                    }
                }
                PanelAction::DismissNotification(id) => {
                    self.panel.bubble.dismiss_active();
                    if let Some(svc) = self.notifications.as_ref().map(Arc::clone) {
                        let _ = std::thread::Builder::new()
                            .name("orbit-notif-dismiss".into())
                            .spawn(move || {
                                svc.dismiss_notification(id);
                            });
                    }
                    if self.panel.bubble.active.is_none() && self.panel.return_expanded_after_bubble {
                        self.panel.return_expanded_after_bubble = false;
                        self.panel.expanded = true;
                        self.panel.active_section = 0;
                        self.panel.leave_since = None;
                        ctx.request_repaint();
                    }
                }
                PanelAction::ClearAllNotifications => {
                    self.panel.bubble.dismiss_active();
                    self.panel.bubble.queue.clear();
                    if let Some(svc) = self.notifications.as_ref().map(Arc::clone) {
                        let _ = std::thread::Builder::new()
                            .name("orbit-notif-clear".into())
                            .spawn(move || {
                                svc.clear_all();
                            });
                    }
                    if self.panel.return_expanded_after_bubble {
                        self.panel.return_expanded_after_bubble = false;
                        self.panel.expanded = true;
                        self.panel.active_section = 0;
                        self.panel.leave_since = None;
                        ctx.request_repaint();
                    }
                }
                PanelAction::ShowLatestNotification => {
                    let items = self.store.snapshot().notifications.notifications.clone();
                    if !items.is_empty() {
                        self.panel.return_expanded_after_bubble = self.panel.expanded;
                        self.panel.expanded = false;
                        self.panel.shelf.open = false;
                        self.panel.adding_task = false;
                        self.panel.leave_since = None;
                        self.panel.bubble.show_all_pending(&items);
                    }
                }
                PanelAction::DismissUtilityToast => {
                    self.panel.utility_toast = None;
                }
                PanelAction::OpenShelfStub => {
                    self.panel.expanded = true;
                    self.panel.shelf.open = true;
                    self.panel.leave_since = None;
                }
                PanelAction::OpenSettings => {
                    self.open_settings(ctx);
                }
                other => self.dispatch_shelf_action(ctx, now_ui, other),
            }
        }
    }

    fn dispatch_shelf_action(&mut self, ctx: &egui::Context, now_ui: f64, action: PanelAction) {
        let shelf = Arc::clone(&self.shelf);
        let msg_tx = self.shelf_msg_tx.clone();
        let wake = self.wake.clone();

        match action {
            PanelAction::ShelfAddEntry(raw) => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-add".into())
                    .spawn(move || {
                        match shelf.add_entry(&raw) {
                            Ok(item) => {
                                let _ = msg_tx.send((format!("Added {}", item.display_name), false));
                            }
                            Err(e) => {
                                let _ = msg_tx.send((e, true));
                            }
                        }
                        wake();
                    });
            }
            PanelAction::ShelfPickFiles => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-pick".into())
                    .spawn(move || {
                        match system::pick_files_native() {
                            Ok(paths) if !paths.is_empty() => {
                                let res = shelf.handle_file_drop(&paths, "shelf");
                                let err = res.iter().find_map(|r| r.error.clone());
                                if let Some(e) = err {
                                    let _ = msg_tx.send((e, true));
                                } else {
                                    let _ = msg_tx.send((format!("Added {} file(s)", res.len()), false));
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                let _ = msg_tx.send((e, true));
                            }
                        }
                        wake();
                    });
            }
            PanelAction::ShelfOpenItem(id) => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-open".into())
                    .spawn(move || {
                        if let Err(e) = shelf.open_file(&id) {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfOpenWithItem(id) => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-open-with".into())
                    .spawn(move || {
                        if let Err(e) = shelf.open_with_file(&id) {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfCopyItem(id) => {
                if let Some(item) = self.store.snapshot().shelf.iter().find(|i| i.id == id) {
                    ctx.copy_text(item.file_path.clone());
                    let label = if item.kind == "link" {
                        "Copied link"
                    } else if item.kind == "text" {
                        "Copied text"
                    } else {
                        "Copied file path"
                    };
                    self.panel.shelf.set_status(label, false, now_ui);
                }
            }
            PanelAction::ShelfRevealItem(id) => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-reveal".into())
                    .spawn(move || {
                        if let Err(e) = shelf.reveal_in_folder(&id) {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfShareFiles => {
                let paths: Vec<String> = self
                    .store
                    .snapshot()
                    .shelf
                    .iter()
                    .filter(|i| i.kind == "file" && i.exists)
                    .map(|i| i.file_path.clone())
                    .collect();
                if paths.is_empty() {
                    self.panel
                        .shelf
                        .set_status("No local files available to share", true, now_ui);
                    return;
                }
                let hwnd_raw = self.hwnd.raw();
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-share".into())
                    .spawn(move || {
                        if let Err(e) = shelf.share_files(hwnd_raw, &paths) {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfRemoveItem(id) => {
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-rm".into())
                    .spawn(move || {
                        if let Err(e) = shelf.remove_item(&id) {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfClear => {
                self.thumb_cache.clear();
                self.thumb_textures.clear();
                self.thumb_tex_ids.clear();
                let _ = std::thread::Builder::new()
                    .name("orbit-shelf-clear".into())
                    .spawn(move || {
                        if let Err(e) = shelf.clear() {
                            let _ = msg_tx.send((e, true));
                            wake();
                        }
                    });
            }
            PanelAction::ShelfToggleStorageMode => {
                let mut cfg = self.store.config().clone();
                cfg.shelf_storage_mode = if cfg.shelf_storage_mode == "copy" {
                    "reference".to_string()
                } else {
                    "copy".to_string()
                };
                let label = if cfg.shelf_storage_mode == "copy" {
                    "Storage mode: Copy-file"
                } else {
                    "Storage mode: Path-only"
                };
                let _ = cfg.save();
                self.store.set_config(cfg);
                self.panel.shelf.set_status(label, false, now_ui);
            }
            _ => {}
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let paths: Vec<String> = dropped
            .into_iter()
            .map(|f| f.path().to_string_lossy().into_owned())
            .collect();
        if paths.is_empty() {
            return;
        }
        self.panel.leave_since = None;

        let shelf = Arc::clone(&self.shelf);
        let tx = if self.panel.shelf.open {
            self.shelf_msg_tx.clone()
        } else {
            self.drop_msg_tx.clone()
        };
        let wake = self.wake.clone();
        let _ = std::thread::Builder::new()
            .name("orbit-shelf-drop".into())
            .spawn(move || {
                let res = shelf.handle_file_drop(&paths, "shelf");
                let feedback = format_drop_feedback(&paths, &res);
                let _ = tx.send(feedback);
                wake();
            });
    }

    fn sync_media_art(&mut self, ctx: &egui::Context) {
        let next = self.store.snapshot().media.as_ref().and_then(|m| {
            m.track.thumbnail_base64.as_ref().map(|b64| {
                (
                    format!("{}:{}:{}", m.source_id, m.track.title, b64.len()),
                    b64.clone(),
                )
            })
        });
        match next {
            Some((key, b64)) => {
                if self.media_art_key.as_ref() != Some(&key) {
                    self.media_art_key = Some(key);
                    self.media_art_tex = decode_data_uri_rgba(&b64, 96).map(|rgba| {
                        let img = ColorImage::from_rgba_unmultiplied(
                            [rgba.width, rgba.height],
                            &rgba.rgba,
                        );
                        ctx.load_texture("orbit-media-art", img, TextureOptions::LINEAR)
                    });
                }
            }
            None => {
                self.media_art_key = None;
                self.media_art_tex = None;
            }
        }
    }

    fn sync_thumbnails(&mut self, ctx: &egui::Context) {
        while let Ok((key, maybe_rgba)) = self.thumb_rx.try_recv() {
            self.thumb_in_flight.remove(&key);
            if let Some(rgba) = maybe_rgba {
                let evicted = self.thumb_cache.insert(key.clone(), rgba.clone());
                for old in evicted {
                    self.thumb_textures.remove(&old);
                    self.thumb_tex_ids.remove(&old);
                }
                let img = ColorImage::from_rgba_unmultiplied([rgba.width, rgba.height], &rgba.rgba);
                let tex = ctx.load_texture(format!("shelf-thumb:{key}"), img, TextureOptions::LINEAR);
                let tid = tex.id();
                self.thumb_textures.insert(key.clone(), tex);
                self.thumb_tex_ids.insert(key, tid);
            }
        }

        let live_paths: HashSet<String> = self
            .store
            .snapshot()
            .shelf
            .iter()
            .map(|i| i.file_path.clone())
            .collect();
        for dropped_key in self.thumb_cache.retain_keys(&live_paths) {
            self.thumb_textures.remove(&dropped_key);
            self.thumb_tex_ids.remove(&dropped_key);
        }

        if !(self.panel.expanded && self.panel.shelf.open) {
            return;
        }
        for item in &self.store.snapshot().shelf {
            if item.kind == "file"
                && item.exists
                && is_supported_image_ext(&item.extension)
                && !self.thumb_cache.contains(&item.file_path)
                && !self.thumb_in_flight.contains(&item.file_path)
            {
                let path_str = item.file_path.clone();
                self.thumb_in_flight.insert(path_str.clone());
                let tx = self.thumb_tx.clone();
                let wake = self.wake.clone();
                let _ = std::thread::Builder::new()
                    .name("orbit-thumb-load".into())
                    .spawn(move || {
                        let loaded = load_thumbnail_rgba(Path::new(&path_str));
                        let _ = tx.send((path_str, loaded));
                        wake();
                    });
            }
        }
    }

    fn set_settings_banner(&self, ctx: &egui::Context, msg: String, is_err: bool) {
        let mut s = self
            .settings_shared
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(ui_state) = s.ui_state.as_mut() {
            ui_state.status_banner = Some((msg, is_err));
            ctx.request_repaint_of(settings_viewport_id());
        }
    }

    fn sync_settings_inputs(&self) {
        let cfg = self.store.config().clone();
        let sys = self.store.snapshot().system;
        let mut s = self
            .settings_shared
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        s.is_open = self.settings_open;
        s.cfg = cfg;
        s.system_light = sys.light_theme;
        s.os_transparency = sys.transparency_enabled;
    }

    fn open_settings(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "bughunt-probe")]
        let (open_started_at, open_start_counters) = self.bughunt.on_open_settings(self.trimmed);
        let already_created = self.settings_created;
        self.settings_created = true;
        self.settings_open = true;
        self.panel.expanded = false;
        self.panel.adding_task = false;
        self.panel.shelf.open = false;
        self.panel.shelf.adding = false;
        self.panel.shelf.menu_open = false;
        self.panel.leave_since = None;
        {
            let cfg = self.store.config().clone();
            let sys = self.store.snapshot().system;
            let mem_mb =
                process::current_working_set_bytes().map(|b| b as f32 / (1024.0 * 1024.0));
            let mut s = self
                .settings_shared
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            s.is_open = true;
            s.cfg = cfg;
            s.system_light = sys.light_theme;
            s.os_transparency = sys.transparency_enabled;
            if s.ui_state.is_none() {
                s.ui_state = Some(SettingsUiState::default());
            }
            if let Some(ui_state) = s.ui_state.as_mut() {
                ui_state.orbit_mem_mb = mem_mb;
                ui_state.is_refreshing_mem = false;
            }
            s.pending_actions.clear();
            s.close_requested = false;
            #[cfg(feature = "bughunt-probe")]
            {
                s.open_started_at = Some(open_started_at);
                s.open_start_counters = open_start_counters;
                s.first_frame_logged = false;
                s.frames_since_open = 0;
                s.prev_frame_at = None;
            }
        }
        let tx = self.settings_msg_tx.clone();
        let wake = self.wake.clone();
        let _ = std::thread::Builder::new()
            .name("orbit-settings-access".into())
            .spawn(move || {
                #[cfg(feature = "bughunt-probe")]
                let t0 = std::time::Instant::now();
                let status = notifications::get_access_status();
                #[cfg(feature = "bughunt-probe")]
                {
                    let c = orbit_platform::process::capture_counters();
                    crate::bughunt::BughuntProbe::on_access_status_finished(
                        t0.elapsed().as_secs_f64() * 1000.0,
                        c.tid,
                        &status,
                    );
                }
                let _ = tx.send(SettingsAsyncEvent::NotifAccessStatus(status));
                wake();
            });
        let settings_id = settings_viewport_id();
        if already_created {
            ctx.send_viewport_cmd_to(settings_id, egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd_to(settings_id, egui::ViewportCommand::Focus);
        }
        ctx.request_repaint_of(egui::ViewportId::ROOT);
        ctx.request_repaint_of(settings_id);
    }

    fn close_settings(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "bughunt-probe")]
        self.bughunt.on_close_settings();
        self.settings_open = false;
        {
            let mut s = self
                .settings_shared
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            s.is_open = false;
            s.ui_state = None;
            s.pending_actions.clear();
            s.close_requested = false;
        }
        // Keep the viewport created and hide it instead of destroying/recreating Intel WGL surfaces.
        ctx.send_viewport_cmd_to(settings_viewport_id(), egui::ViewportCommand::Visible(false));
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn apply_config_change(&mut self, ctx: &egui::Context, new_cfg: Config) {
        let new_cfg = new_cfg.normalized();
        let prev_launch = self.store.config().launch_at_startup;
        if prev_launch != new_cfg.launch_at_startup {
            match std::env::current_exe() {
                Ok(exe) => {
                    if let Err(e) =
                        orbit_platform::registry::sync_startup_registry(new_cfg.launch_at_startup, &exe)
                    {
                        tracing::warn!("sync_startup_registry: {e}");
                        self.set_settings_banner(ctx, e, true);
                    }
                }
                Err(e) => {
                    let msg = format!("Failed to locate executable path: {e}");
                    tracing::warn!("{msg}");
                    self.set_settings_banner(ctx, msg, true);
                }
            }
        }

        if let Some(w) = &self.weather {
            w.set_enabled(new_cfg.weather_enabled);
        }
        if let Some(p) = &self.privacy {
            p.set_enabled(new_cfg.show_privacy_indicators);
        }
        let sys_light = self.store.snapshot().system.light_theme;
        self.theme = Theme::for_config(new_cfg.theme, new_cfg.color_theme, sys_light);

        if let Err(e) = new_cfg.save() {
            tracing::warn!("saving config: {e}");
            self.set_settings_banner(ctx, format!("Failed to save config: {e}"), true);
        }
        self.store.set_config(new_cfg.clone());
        let mut shared = self
            .config_shared
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *shared = new_cfg;
    }

    fn handle_settings_async_events(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        while let Ok(ev) = self.settings_msg_rx.try_recv() {
            let mut s = self
                .settings_shared
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            match ev {
                SettingsAsyncEvent::NotifAccessStatus(status) => {
                    if let Some(ui_state) = s.ui_state.as_mut() {
                        ui_state.notif_access_status = status;
                        updated = true;
                    }
                }
                SettingsAsyncEvent::NotifAccessFinished(res) => {
                    if let Some(ui_state) = s.ui_state.as_mut() {
                        ui_state.requesting_notif_access = false;
                        match res {
                            Ok(status) => {
                                ui_state.notif_access_status = status.clone();
                                ui_state.status_banner = Some((
                                    format!("Notification listener access: {status}"),
                                    status == "denied",
                                ));
                            }
                            Err(e) => {
                                ui_state.status_banner = Some((e, true));
                            }
                        }
                        updated = true;
                    }
                }
                SettingsAsyncEvent::DataCleared(res) => match res {
                    Ok(()) => {
                        drop(s);
                        self.store.apply(CoreEvent::RemindersUpdated(Vec::new()));
                        self.store.apply(CoreEvent::ShelfUpdated(Vec::new()));
                        let mut s = self
                            .settings_shared
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        if let Some(ui_state) = s.ui_state.as_mut() {
                            ui_state.status_banner = Some((
                                "Cleared local calendar tasks and Shelf entries.".into(),
                                false,
                            ));
                            updated = true;
                        }
                    }
                    Err(e) => {
                        if let Some(ui_state) = s.ui_state.as_mut() {
                            ui_state.status_banner = Some((e, true));
                            updated = true;
                        }
                    }
                },
                SettingsAsyncEvent::MemoryMeasured(mb) => {
                    if let Some(ui_state) = s.ui_state.as_mut() {
                        ui_state.orbit_mem_mb = mb;
                        ui_state.is_refreshing_mem = false;
                        updated = true;
                    }
                }
                SettingsAsyncEvent::MemoryTrimmed {
                    trimmed_procs,
                    freed_mb,
                    current_mb,
                    metrics,
                } => {
                    drop(s);
                    self.store.apply(CoreEvent::HardwareUpdated(metrics));
                    let mut s = self
                        .settings_shared
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if let Some(ui_state) = s.ui_state.as_mut() {
                        ui_state.orbit_mem_mb = current_mb;
                        ui_state.is_refreshing_mem = false;
                        ui_state.status_banner = if trimmed_procs > 0 {
                            Some((
                                format!("Trimmed ORBIT working set (freed {:.1} MB)", freed_mb),
                                false,
                            ))
                        } else {
                            Some(("Working set trim failed".into(), true))
                        };
                        updated = true;
                    }
                }
            }
        }
        if updated && self.settings_open {
            ctx.request_repaint_of(settings_viewport_id());
        }
    }

    fn drain_and_handle_settings(&mut self, ctx: &egui::Context) {
        let (actions, close_req) = {
            let mut s = self
                .settings_shared
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            (
                std::mem::take(&mut s.pending_actions),
                std::mem::take(&mut s.close_requested),
            )
        };
        let mut close_now = close_req;
        for action in actions {
            match action {
                SettingsAction::ConfigChanged(new_cfg) => {
                    self.apply_config_change(ctx, new_cfg);
                }
                SettingsAction::RequestNotificationAccess => {
                    let tx = self.settings_msg_tx.clone();
                    let wake = self.wake.clone();
                    let _ = std::thread::Builder::new()
                        .name("orbit-notif-req".into())
                        .spawn(move || {
                            let res = notifications::request_access_blocking();
                            let _ = tx.send(SettingsAsyncEvent::NotifAccessFinished(res));
                            wake();
                        });
                }
                SettingsAction::ClearNotifications => {
                    self.panel.bubble.dismiss_active();
                    self.panel.bubble.queue.clear();
                    if let Some(svc) = self.notifications.as_ref().map(Arc::clone) {
                        svc.clear_all();
                    }
                    self.set_settings_banner(ctx, "Cleared active notifications.".into(), false);
                }
                SettingsAction::ResetDefaults => {
                    self.apply_config_change(ctx, Config::default());
                    self.set_settings_banner(ctx, "Settings restored to defaults.".into(), false);
                }
                SettingsAction::ClearLocalData => {
                    self.thumb_cache.clear();
                    self.thumb_textures.clear();
                    self.thumb_tex_ids.clear();
                    let db = Arc::clone(&self.db);
                    let tx = self.settings_msg_tx.clone();
                    let wake = self.wake.clone();
                    let _ = std::thread::Builder::new()
                        .name("orbit-clear-data".into())
                        .spawn(move || {
                            let res = db.clear_local_data();
                            let _ = tx.send(SettingsAsyncEvent::DataCleared(res));
                            wake();
                        });
                }
                SettingsAction::RevealConfigFolder => {
                    let (msg, is_err) = match orbit_core::config::dir()
                        .map_err(|e| e.to_string())
                        .and_then(|d| system::reveal_dir_in_explorer(&d))
                    {
                        Ok(()) => ("Opened config folder in Explorer.".into(), false),
                        Err(e) => (e, true),
                    };
                    self.set_settings_banner(ctx, msg, is_err);
                }
                SettingsAction::RefreshMemory => {
                    let tx = self.settings_msg_tx.clone();
                    let wake = self.wake.clone();
                    let _ = std::thread::Builder::new()
                        .name("orbit-mem-refresh".into())
                        .spawn(move || {
                            let mb = process::current_working_set_bytes()
                                .map(|b| b as f32 / (1024.0 * 1024.0));
                            let _ = tx.send(SettingsAsyncEvent::MemoryMeasured(mb));
                            wake();
                        });
                }
                SettingsAction::TrimMemoryNow => {
                    let tx = self.settings_msg_tx.clone();
                    let wake = self.wake.clone();
                    if let Some(hw) = self.hardware.as_ref().map(Arc::clone) {
                        let _ = std::thread::Builder::new()
                            .name("orbit-mem-trim".into())
                            .spawn(move || {
                                let res = hw.free_up_memory();
                                let metrics = hw.sample_once();
                                let current_mb = process::current_working_set_bytes()
                                    .map(|b| b as f32 / (1024.0 * 1024.0));
                                let _ = tx.send(SettingsAsyncEvent::MemoryTrimmed {
                                    trimmed_procs: res.trimmed_processes,
                                    freed_mb: res.freed_mb,
                                    current_mb,
                                    metrics,
                                });
                                wake();
                            });
                    } else {
                        let _ = std::thread::Builder::new()
                            .name("orbit-mem-trim".into())
                            .spawn(move || {
                                let _ = process::trim_working_set();
                                let mb = process::current_working_set_bytes()
                                    .map(|b| b as f32 / (1024.0 * 1024.0));
                                let _ = tx.send(SettingsAsyncEvent::MemoryMeasured(mb));
                                wake();
                            });
                    }
                }
                SettingsAction::Close => {
                    close_now = true;
                }
            }
        }
        if close_now && self.settings_open {
            self.close_settings(ctx);
        } else if self.settings_open {
            self.sync_settings_inputs();
        }
    }

    fn show_settings_viewport(&mut self, ctx: &egui::Context) {
        if !self.settings_created {
            return;
        }
        self.sync_settings_inputs();
        let settings_id = settings_viewport_id();
        let shared = Arc::clone(&self.settings_shared);

        // Explicitly disable root transparency inheritance so Settings uses a standard opaque HWND.
        let mut builder = egui::ViewportBuilder::default()
            .with_title("ORBIT Settings")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([520.0, 460.0])
            .with_resizable(true)
            .with_visible(self.settings_open)
            .with_transparent(false);
        if let Some(icon) = crate::app_icon_data() {
            builder = builder.with_icon(icon);
        }

        ctx.show_viewport_deferred(settings_id, builder, move |settings_ui, _class| {
            #[cfg(feature = "bughunt-probe")]
            let cb_start = std::time::Instant::now();
            let (is_open, cfg, system_light, os_transparency, mut ui_state) = {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                if !s.is_open {
                    return;
                }
                (
                    true,
                    s.cfg.clone(),
                    s.system_light,
                    s.os_transparency,
                    s.ui_state.take().unwrap_or_default(),
                )
            };
            if !is_open {
                return;
            }
            let mut close_requested = settings_ui.input(|i| i.viewport().close_requested());
            let logo_key = egui::Id::NULL.with("orbit-settings-logo");
            let logo_tex = match settings_ui.data(|d| d.get_temp::<TextureHandle>(logo_key)) {
                Some(t) => Some(t),
                None => orbit_services::thumbnails::decode_image_bytes_rgba(crate::APP_ICON_PNG, 64)
                    .map(|rgba| {
                        let img = ColorImage::from_rgba_unmultiplied(
                            [rgba.width, rgba.height],
                            &rgba.rgba,
                        );
                        let tex = settings_ui.load_texture(
                            "orbit-settings-logo",
                            img,
                            TextureOptions::LINEAR,
                        );
                        settings_ui.data_mut(|d| d.insert_temp(logo_key, tex.clone()));
                        tex
                    }),
            };
            let canvas = settings::canvas_color_for(&cfg, system_light);
            #[cfg(feature = "bughunt-probe")]
            let t_ui = std::time::Instant::now();
            let mut actions = Vec::new();
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(canvas)
                        .inner_margin(egui::Margin::symmetric(16, 12)),
                )
                .show(settings_ui, |ui| {
                    actions = settings::show_settings(
                        ui,
                        &cfg,
                        system_light,
                        os_transparency,
                        logo_tex.as_ref().map(TextureHandle::id),
                        &mut ui_state,
                    );
                });
            #[cfg(feature = "bughunt-probe")]
            let ui_build_ms = t_ui.elapsed().as_secs_f64() * 1000.0;

            let mut wake_root = close_requested || !actions.is_empty();
            {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                for action in &actions {
                    match action {
                        SettingsAction::ConfigChanged(next_cfg) => {
                            s.cfg = next_cfg.clone();
                        }
                        SettingsAction::ResetDefaults => {
                            s.cfg = Config::default();
                        }
                        SettingsAction::Close => {
                            close_requested = true;
                            wake_root = true;
                        }
                        _ => {}
                    }
                }
                s.pending_actions.extend(actions);
                if close_requested {
                    s.close_requested = true;
                    s.ui_state = None;
                } else {
                    s.ui_state = Some(ui_state);
                }
                #[cfg(feature = "bughunt-probe")]
                {
                    let started_at = s.open_started_at;
                    let start_counters = s.open_start_counters;
                    let mut first_logged = s.first_frame_logged;
                    let mut frames_since_open = s.frames_since_open;
                    let mut prev_frame_at = s.prev_frame_at;
                    crate::bughunt::BughuntProbe::on_settings_deferred_frame(
                        started_at,
                        start_counters,
                        &mut first_logged,
                        &mut frames_since_open,
                        &mut prev_frame_at,
                        ui_build_ms,
                        cb_start.elapsed().as_secs_f64() * 1000.0,
                    );
                    s.first_frame_logged = first_logged;
                    s.frames_since_open = frames_since_open;
                    s.prev_frame_at = prev_frame_at;
                }
            }
            if wake_root {
                settings_ui
                    .ctx()
                    .request_repaint_of(egui::ViewportId::ROOT);
            }
        });
    }
}

fn log_gl(cc: &eframe::CreationContext<'_>) {
    if let Some(gl) = &cc.gl {
        use eframe::glow::HasContext;
        // SAFETY: plain string queries on the current OpenGL context.
        let (renderer, version, alpha) = unsafe {
            (
                gl.get_parameter_string(eframe::glow::RENDERER),
                gl.get_parameter_string(eframe::glow::VERSION),
                gl.get_framebuffer_attachment_parameter_i32(
                    eframe::glow::FRAMEBUFFER,
                    eframe::glow::BACK_LEFT,
                    eframe::glow::FRAMEBUFFER_ATTACHMENT_ALPHA_SIZE,
                ),
            )
        };
        tracing::info!("gpu: {renderer} (OpenGL {version}), alpha bits {alpha}");
    }
}

impl eframe::App for OrbitApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(cmd) = self.tray_rx.try_recv() {
            match cmd {
                TrayCommand::OpenSettings => self.open_settings(ctx),
                TrayCommand::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        let mut system_changed = false;
        let mut privacy_changed = false;
        while let Ok(ev) = self.ev_rx.try_recv() {
            let is_notif = matches!(ev, CoreEvent::NotificationsUpdated(_));
            if matches!(ev, CoreEvent::SystemChanged(_)) {
                system_changed = true;
            }
            if matches!(ev, CoreEvent::PrivacyUpdated(_)) {
                privacy_changed = true;
            }
            self.store.apply(ev);
            if is_notif {
                let s = self.store.snapshot();
                let sup = s.hidden || (s.fullscreen_suppressed && s.config.pause_in_fullscreen);
                self.panel.bubble.ingest_snapshot(
                    &s.notifications,
                    s.config.notifications_enabled,
                    sup,
                );
            }
        }
        if system_changed || privacy_changed {
            let s = self.store.snapshot();
            if system_changed {
                self.theme = Theme::for_config(s.config.theme, s.config.color_theme, s.system.light_theme);
            }
            let suppressed =
                s.hidden || (s.fullscreen_suppressed && s.config.pause_in_fullscreen);
            let alerts_on = s.config.utility_alerts_enabled && !suppressed;
            let now_ui = ctx.input(|i| i.time);
            let timer_toast_active = self
                .panel
                .utility_toast
                .as_ref()
                .is_some_and(|t| t.is_timer_complete && t.until_secs > now_ui);
            let mut next_toast: Option<UtilityToast> = None;

            if system_changed {
                let cur_ac = s.system.battery.map(|b| b.charging);
                let cur_pct = s.system.battery.map(|b| b.percent);
                if let (Some(prev), Some(cur)) = (self.prev_ac_power, cur_ac)
                    && prev != cur
                    && alerts_on
                {
                    let detail = cur_pct
                        .map(|p| format!("Battery {p}%"))
                        .unwrap_or_else(|| "AC power state updated".into());
                    next_toast = Some(if cur {
                        UtilityToast {
                            kind: UtilityAlertKind::PowerConnected,
                            title: "Power Connected".into(),
                            detail,
                            accent: self.theme.accent_green,
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    } else {
                        UtilityToast {
                            kind: UtilityAlertKind::PowerDisconnected,
                            title: "On Battery Power".into(),
                            detail,
                            accent: self.theme.accent_amber,
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    });
                } else if s.system.wifi_connected != self.prev_wifi && alerts_on {
                    next_toast = Some(if s.system.wifi_connected {
                        UtilityToast {
                            kind: UtilityAlertKind::WifiConnected,
                            title: "Wi-Fi Connected".into(),
                            detail: "Network connection online".into(),
                            accent: self.theme.accent_green,
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    } else {
                        UtilityToast {
                            kind: UtilityAlertKind::WifiDisconnected,
                            title: "Wi-Fi Disconnected".into(),
                            detail: "No wireless network connection".into(),
                            accent: egui::Color32::from_rgb(248, 113, 113),
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    });
                } else if s.system.bluetooth_enabled != self.prev_bt && alerts_on {
                    next_toast = Some(if s.system.bluetooth_enabled {
                        UtilityToast {
                            kind: UtilityAlertKind::BluetoothOn,
                            title: "Bluetooth On".into(),
                            detail: "Wireless devices ready".into(),
                            accent: self.theme.accent,
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    } else {
                        UtilityToast {
                            kind: UtilityAlertKind::BluetoothOff,
                            title: "Bluetooth Off".into(),
                            detail: "Bluetooth radio disabled".into(),
                            accent: self.theme.accent_amber,
                            until_secs: now_ui + 2.6,
                            is_timer_complete: false,
                        }
                    });
                }
                self.prev_ac_power = cur_ac;
                self.prev_wifi = s.system.wifi_connected;
                self.prev_bt = s.system.bluetooth_enabled;
            }

            if privacy_changed {
                if !self.privacy_initialized {
                    self.privacy_initialized = true;
                } else if !self.prev_location
                    && s.privacy.location_active
                    && alerts_on
                    && next_toast.is_none()
                {
                    next_toast = Some(UtilityToast {
                        kind: UtilityAlertKind::LocationActive,
                        title: "Location In Use".into(),
                        detail: s
                            .privacy
                            .location_app
                            .clone()
                            .unwrap_or_else(|| "An app is accessing location".into()),
                        accent: self.theme.accent_cyan,
                        until_secs: now_ui + 2.8,
                        is_timer_complete: false,
                    });
                }
                self.prev_location = s.privacy.location_active;
            }

            if let Some(toast) = next_toast
                && !timer_toast_active
            {
                let dur = (toast.until_secs - now_ui).max(0.1) + 0.05;
                self.panel.utility_toast = Some(toast);
                ctx.request_repaint_after(Duration::from_secs_f64(dur));
            }
        }
        self.handle_settings_async_events(ctx);
        self.drain_and_handle_settings(ctx);
        {
            let mut shared = self
                .config_shared
                .write()
                .unwrap_or_else(|e| e.into_inner());
            if *shared != *self.store.config() {
                *shared = self.store.config().clone();
            }
        }

        let animations = self.store.snapshot().system.animations;
        let motion_preset = self.store.config().motion_preset;
        anim::set_motion(
            ctx,
            Motion {
                reduced: !animations || motion_preset == orbit_core::config::MotionPreset::Reduced,
                preset: motion_preset,
            },
        );

        while let Ok((trimmed_procs, freed_mb, metrics)) = self.boost_rx.try_recv() {
            self.store.apply(CoreEvent::HardwareUpdated(metrics));
            let now_ui = ctx.input(|i| i.time);
            let (msg, is_err) = if trimmed_procs > 0 {
                (format!("Freed {:.1} MB", freed_mb), false)
            } else {
                ("Trim failed".to_string(), true)
            };
            self.panel.ram_boost_feedback = Some((msg, is_err, now_ui + 4.0));
            self.panel.last_ram_boost = if trimmed_procs > 0 {
                Some((trimmed_procs, freed_mb, now_ui + 12.0))
            } else {
                None
            };
            ctx.request_repaint_after(Duration::from_secs_f64(4.05));
        }

        while let Ok((msg, is_err)) = self.shelf_msg_rx.try_recv() {
            let now_ui = ctx.input(|i| i.time);
            self.panel.shelf.set_status(msg, is_err, now_ui);
        }
        while let Ok((msg, is_err)) = self.drop_msg_rx.try_recv() {
            let now_ui = ctx.input(|i| i.time);
            self.panel.drop_feedback = Some((msg, is_err, now_ui + 3.0));
            ctx.request_repaint_after(Duration::from_secs_f64(3.05));
        }
        self.sync_thumbnails(ctx);
        self.sync_media_art(ctx);
        self.handle_dropped_files(ctx);

        let _ = window::ensure_overlay(self.hwnd, self.panel.adding_task || self.panel.shelf.adding);
        self.anchor_window();
        self.apply_layer();
        self.apply_stealth();
        self.apply_click_through();
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        #[cfg(feature = "debug-overlay")]
        self.overlay.begin();

        let ctx = ui.ctx().clone();
        let use_24h = self.store.config().time_format == TimeFormat::Hour24;
        let clock = to_clock_status(system::clock_with_format(use_24h));
        let millis_to_next_minute = clock.millis_to_next_minute;
        self.store.apply(CoreEvent::ClockTick(clock));

        let now_ms = now_unix_ms();
        let timer = self.store.snapshot().focus_timer.clone();
        if timer.is_running() && timer.remaining_secs(now_ms) == 0 {
            self.store
                .apply(CoreEvent::FocusTimerUpdated(FocusTimerState::default()));
            self.panel.expanded = false;
            self.panel.shelf.open = false;
            self.panel.adding_task = false;
            self.panel.leave_since = None;
            let now_ui = ctx.input(|i| i.time);
            self.panel.utility_toast = Some(UtilityToast {
                kind: UtilityAlertKind::TimerComplete,
                title: "Focus Timer Complete!".into(),
                detail: "Session finished · Take a short break".into(),
                accent: self.theme.accent_green,
                until_secs: now_ui + 8.0,
                is_timer_complete: true,
            });
            let _ = std::thread::Builder::new()
                .name("orbit-timer-chime".into())
                .spawn(system::play_timer_chime);
            ctx.request_repaint();
        }

        let snap = self.store.snapshot().clone();
        let suppressed =
            snap.hidden || (snap.fullscreen_suppressed && snap.config.pause_in_fullscreen);
        if suppressed && self.panel.expanded {
            self.panel.expanded = false;
            self.panel.adding_task = false;
        }

        #[cfg(feature = "bughunt-probe")]
        {
            use crate::bughunt::ProbeDirective;
            let settings_frames = self
                .settings_shared
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .frames_since_open;
            match self
                .bughunt
                .tick(&ctx, self.settings_open, settings_frames, self.trimmed)
            {
                ProbeDirective::None => {}
                ProbeDirective::OpenSettings => {
                    if !self.settings_open {
                        self.open_settings(&ctx);
                    }
                }
                ProbeDirective::CloseSettings => {
                    if self.settings_open {
                        self.close_settings(&ctx);
                    }
                }
                ProbeDirective::OpenPanel => {
                    self.panel.expanded = true;
                    self.panel.leave_since = None;
                    ctx.request_repaint();
                }
                ProbeDirective::ClosePanel => {
                    self.panel.expanded = false;
                    self.panel.shelf.open = false;
                    self.panel.leave_since = None;
                    ctx.request_repaint();
                }
                ProbeDirective::QuitApp => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        let effective_glass = self.glass_override.unwrap_or(snap.config.glass_mode);
        let live_glass = self.backdrop.is_some()
            && effective_glass != GlassMode::Opaque
            && snap.system.transparency_enabled;
        let media_art = self.media_art_tex.as_ref().map(TextureHandle::id);
        let (shape, hovered, hover_t, bubble_draining, current_section) = if suppressed {
            anim::entry_drop_progress(&ctx, egui::Id::new("orbit-island"), false);
            (None, false, 0.0, false, SectionKind::Home)
        } else {
            let out = panel::show_island(
                ui,
                &snap,
                &self.theme,
                &mut self.panel,
                &self.thumb_tex_ids,
                media_art,
                now_ms,
                live_glass,
                effective_glass == GlassMode::Opaque,
            );
            let cur_sec = out.current_section;
            let hov = out.hovered;
            let ht = out.hover_t;
            let draining = out.bubble_draining;
            let shp = out.shape;
            self.dispatch_actions(&ctx, out.actions);
            (Some(shp), hov, ht, draining, cur_sec)
        };

        self.sync_service_gates(suppressed, current_section);

        // Wake up for draining notification bar (~25 fps), 1s while focus timer runs, or next minute.
        if bubble_draining && !suppressed {
            ctx.request_repaint_after(Duration::from_millis(40));
        } else if self.store.snapshot().focus_timer.is_running() && !suppressed {
            ctx.request_repaint_after(Duration::from_secs(1));
        } else {
            ctx.request_repaint_after(Duration::from_millis(millis_to_next_minute));
        }

        self.trim_when_idle(
            &ctx,
            hovered
                || self.panel.expanded
                || self.panel.bubble.active.is_some()
                || self.panel.utility_toast.is_some()
                || self.panel.drop_feedback.is_some()
                || self.settings_open,
        );

        self.show_settings_viewport(&ctx);

        #[cfg(feature = "debug-overlay")]
        let overlay_rect = Some(self.overlay.show(ui));
        #[cfg(not(feature = "debug-overlay"))]
        let overlay_rect: Option<Rect> = None;

        self.update_shape(&ctx, shape, overlay_rect);

        self.frames = self.frames.saturating_add(1);
        if self.frames == 3 {
            window::ensure_shown(self.hwnd);
            window::clear_gdi_surface(self.hwnd);
            if let Err(e) = window::set_opacity(self.hwnd, 255) {
                tracing::warn!("window opacity: {e}");
            }
        } else if self.frames < 3 {
            ctx.request_repaint();
        }

        if let Some(backdrop) = self.backdrop.as_mut() {
            let show_glass = live_glass && !suppressed && self.frames >= 3;
            let rect_px = if show_glass {
                let ppp = ctx.pixels_per_point();
                shape.map(|s| BackdropRect {
                    x: s.rect.min.x * ppp,
                    y: s.rect.min.y * ppp,
                    width: s.rect.width() * ppp,
                    height: s.rect.height() * ppp,
                    radius: s.radius * ppp,
                })
            } else {
                None
            };
            let (sigma, blur_enabled) =
                blur_sigma_for_mode(effective_glass, snap.system.transparency_enabled);
            let (top, bottom, rim, highlight) =
                self.theme
                    .backdrop_palette(effective_glass, snap.system.transparency_enabled, hover_t);
            backdrop.update(
                rect_px,
                BackdropColors {
                    top,
                    bottom,
                    rim,
                    highlight,
                    blur_radius: sigma,
                    blur_enabled,
                },
            );
            backdrop.set_visible(rect_px.is_some());
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if raw.viewport_id != egui::ViewportId::ROOT {
            return;
        }
        #[cfg(feature = "bughunt-probe")]
        if self.bughunt.want_panel_hover() {
            if let Some((_, _, w_px, _)) = self.anchor {
                let ppp = ctx.pixels_per_point();
                raw.events.push(egui::Event::PointerMoved(pos2(
                    (w_px as f32 / ppp) * 0.5,
                    24.0,
                )));
                self.bughunt_panel_was_hovered = true;
            }
            return;
        } else if self.bughunt_panel_was_hovered {
            raw.events.push(egui::Event::PointerGone);
            self.bughunt_panel_was_hovered = false;
        }
        let inside = self.hit.as_ref().is_none_or(HitTester::inside);
        if self.was_inside && !inside {
            let clicking = raw
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::PointerButton { .. }));
            if clicking {
                ctx.request_repaint();
                return;
            }
            raw.events.push(egui::Event::PointerGone);
        }
        if !self.was_inside
            && inside
            && let (Ok((x, y)), Some((ox, oy, _, _))) = (monitor::cursor_pos(), self.anchor)
        {
            let ppp = ctx.pixels_per_point();
            raw.events.push(egui::Event::PointerMoved(pos2(
                (x - ox) as f32 / ppp,
                (y - oy) as f32 / ppp,
            )));
        }
        self.was_inside = inside;
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.backdrop.take();
        self.tray.take();
        self.hit.take();
        self.fullscreen.take();
        self.hardware.take();
        self.weather.take();
        self.media.take();
        self.privacy.take();
        self.spectrum.take();
        self.notifications.take();
        self.watcher.take();
        let _ = self.store.config().save();
        tracing::info!("orbit-egui exited cleanly (frames={})", self.frames);
    }
}

pub(crate) fn format_drop_feedback(
    paths: &[String],
    results: &[orbit_core::events::DropFileResult],
) -> (String, bool) {
    let blocked = results.iter().find(|r| r.error.is_some());
    if let Some(bad) = blocked {
        let err = bad.error.as_deref().unwrap_or("Failed to add file");
        if err.contains("blocked for security")
            && let Some(ext) = Path::new(&bad.path).extension().and_then(|s| s.to_str())
            && !ext.is_empty()
        {
            return (format!("Blocked: .{} files are not allowed", ext.to_lowercase()), true);
        }
        return (err.to_string(), true);
    }

    let ok_items: Vec<_> = results.iter().filter_map(|r| r.shelf_item.as_ref()).collect();
    match ok_items.len() {
        0 => ("No valid file added".to_string(), true),
        1 => {
            let name = ok_items[0].display_name.clone();
            (format!("Added: {name}"), false)
        }
        n => {
            let _ = paths;
            (format!("Added {n} files"), false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_drop_feedback;
    use orbit_core::config::Config;
    use orbit_core::events::WakeFn;
    use orbit_services::db::DatabaseManager;
    use orbit_services::shelf::ShelfService;
    use std::fs;
    use std::sync::{Arc, RwLock};

    #[test]
    fn drop_feedback_formats_good_blocked_js_and_three_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("orbit_app_drop_{nanos}"));
        fs::create_dir_all(&dir).unwrap();

        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let wake: WakeFn = Arc::new(|| {});
        let shelf = ShelfService::new(db, cfg, tx, wake);

        // 1. One good file
        let good = dir.join("notes.txt");
        fs::write(&good, b"hello").unwrap();
        let good_paths = vec![good.to_string_lossy().into_owned()];
        let good_res = shelf.handle_file_drop(&good_paths, "shelf");
        assert_eq!(format_drop_feedback(&good_paths, &good_res), ("Added: notes.txt".into(), false));

        // 2. One .js file -> blocked message
        let js = dir.join("script.js");
        fs::write(&js, b"console.log(1);").unwrap();
        let js_paths = vec![js.to_string_lossy().into_owned()];
        let js_res = shelf.handle_file_drop(&js_paths, "shelf");
        assert_eq!(
            format_drop_feedback(&js_paths, &js_res),
            ("Blocked: .js files are not allowed".into(), true)
        );

        // 3. Three files at once
        let mut batch = Vec::new();
        for name in ["one.txt", "two.pdf", "three.png"] {
            let p = dir.join(name);
            fs::write(&p, b"data").unwrap();
            batch.push(p.to_string_lossy().into_owned());
        }
        let batch_res = shelf.handle_file_drop(&batch, "shelf");
        assert_eq!(format_drop_feedback(&batch, &batch_res), ("Added 3 files".into(), false));

        let _ = fs::remove_dir_all(&dir);
    }
}
