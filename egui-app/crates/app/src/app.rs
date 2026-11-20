// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/app.rs (Gate 1 skeleton).

//! The eframe app: owns the store, tray, watcher, hit-tester, and keeps the
//! Win32 overlay window synchronized with the rendered pill shape.

use crate::tray::{Tray, TrayCommand};
use crate::watcher::{self, Watcher};
use anyhow::{Context as _, Result};
use crossbeam_channel::{Receiver, unbounded};
use egui::{Rect, Ui, pos2};
use orbit_core::config::Config;
use orbit_core::events::{ClockStatus, CoreEvent};
use orbit_core::store::AppStore;
use orbit_platform::hittest::{HitShape, HitTester};
use orbit_platform::single_instance::InstanceGuard;
use orbit_platform::{Hwnd, fonts, material, monitor, process, system, window};
use orbit_ui::anim::{self, Motion};
use orbit_ui::geometry::Squircle;
use orbit_ui::pill::{self, WINDOW_HEIGHT};
use orbit_ui::theme::{self, FontAssets, Theme};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::time::Duration;

type PushedShape = (Vec<(f32, f32)>, Option<Rect>);

fn to_clock_status(c: system::Clock) -> ClockStatus {
    ClockStatus {
        time: c.time,
        date: c.date,
        millis_to_next_minute: c.millis_to_next_minute,
    }
}

pub struct OrbitApp {
    hwnd: Hwnd,
    store: AppStore,
    theme: Theme,
    tray: Option<Tray>,
    tray_rx: Receiver<TrayCommand>,
    _watcher: Watcher,
    sys_rx: Receiver<CoreEvent>,
    hit: Option<HitTester>,
    pushed: Option<PushedShape>,
    shape_dirty: bool,
    was_inside: bool,
    applied_on_top: Option<bool>,
    frames: u32,
    collapsed_since: Option<f64>,
    trimmed: bool,
    anchor: Option<(i32, i32, i32, i32)>,
    _instance: InstanceGuard,
    #[cfg(feature = "debug-overlay")]
    overlay: crate::overlay::DebugOverlay,
}

impl OrbitApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: Config,
        instance: InstanceGuard,
        hidden: bool,
    ) -> Result<Self> {
        let ctx = cc.egui_ctx.clone();
        let segoe = fonts::load_segoe_ui();
        theme::install_fonts(
            &ctx,
            FontAssets {
                regular: segoe.regular,
                semibold: segoe.semibold,
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

        let repaint = ctx.clone();
        let hit = match HitTester::spawn(move |_| repaint.request_repaint()) {
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
        let initial_clock = to_clock_status(system::clock());
        let store = AppStore::new(cfg, initial_sys, initial_clock, hidden);

        let (sys_tx, sys_rx) = unbounded();
        let watcher = Watcher::spawn(sys_tx, ctx.clone()).context("spawning watcher")?;

        Ok(Self {
            hwnd,
            store,
            theme: Theme::default(),
            tray,
            tray_rx,
            _watcher: watcher,
            sys_rx,
            hit,
            pushed: None,
            shape_dirty: true,
            was_inside: false,
            applied_on_top: None,
            frames: 0,
            collapsed_since: None,
            trimmed: false,
            anchor: None,
            _instance: instance,
            #[cfg(feature = "debug-overlay")]
            overlay: Default::default(),
        })
    }

    fn anchor_window(&mut self) {
        let Some(mon) = self.store.snapshot().system.monitor else {
            return;
        };
        let scale = window::dpi(self.hwnd) as f32 / 96.0;
        let h = (WINDOW_HEIGHT * scale).ceil() as i32;
        let rect = (mon.work.left, mon.work.top, mon.work.width(), h);
        if self.anchor != Some(rect) {
            match window::set_rect(self.hwnd, rect.0, rect.1, rect.2, rect.3) {
                Ok(()) => {
                    self.anchor = Some(rect);
                    self.shape_dirty = true;
                }
                Err(e) => tracing::warn!("anchoring window: {e}"),
            }
        }
    }

    fn apply_layer(&mut self) {
        let on_top = self.store.config().on_top;
        let drift = window::is_topmost(self.hwnd) != on_top;
        if self.applied_on_top == Some(on_top) && !drift {
            return;
        }
        match window::set_layer(self.hwnd, None, on_top) {
            Ok(()) => {
                self.applied_on_top = Some(on_top);
            }
            Err(e) => tracing::warn!("window layer: {e}"),
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
                TrayCommand::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        while let Ok(ev) = self.sys_rx.try_recv() {
            self.store.apply(ev);
        }
        let animations = self.store.snapshot().system.animations;
        anim::set_motion(
            ctx,
            Motion {
                reduced: !animations,
            },
        );

        let _ = window::ensure_overlay(self.hwnd, false);
        self.anchor_window();
        self.apply_layer();
        self.apply_click_through();
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        #[cfg(feature = "debug-overlay")]
        self.overlay.begin();

        let ctx = ui.ctx().clone();
        let clock = to_clock_status(system::clock());
        let millis_to_next_minute = clock.millis_to_next_minute;
        self.store.apply(CoreEvent::ClockTick(clock));

        let snap = self.store.snapshot().clone();
        let (shape, hovered) = if snap.hidden {
            (None, false)
        } else {
            let out = pill::show(ui, &self.theme, &snap.clock, snap.system.battery);
            (Some(out.shape), out.hovered)
        };

        // Wake up once per minute, right after the minute rolls over.
        ctx.request_repaint_after(Duration::from_millis(millis_to_next_minute));

        self.trim_when_idle(&ctx, hovered);

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
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
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
        self.tray.take();
        self.hit.take();
        let _ = self.store.config().save();
        tracing::info!("orbit-egui exited cleanly");
    }
}
