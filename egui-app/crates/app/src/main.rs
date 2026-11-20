// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/main.rs.

//! ORBIT Dynamic Island native egui application (Gate 1 skeleton).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod diag;
#[cfg(feature = "debug-overlay")]
mod overlay;
mod tray;
mod watcher;

use anyhow::{Context, Result};
use orbit_core::config::Config;
use orbit_ui::pill::WINDOW_HEIGHT;

fn main() -> Result<()> {
    let Some(instance) = orbit_platform::single_instance::acquire("OrbitEgui.SingleInstance")
        .context("single-instance check")?
    else {
        return Ok(());
    };
    diag::init();
    start(instance).inspect_err(|e| tracing::error!("ORBIT stopped: {e:#}"))
}

fn start(instance: orbit_platform::single_instance::InstanceGuard) -> Result<()> {
    let hidden = std::env::args().any(|a| a == "--hidden");
    let cfg = Config::load().context("loading config")?;
    diag::log_startup(hidden);

    let viewport = egui::ViewportBuilder::default()
        .with_title("ORBIT")
        .with_app_id("ORBIT")
        .with_inner_size([800.0, WINDOW_HEIGHT])
        .with_position([0.0, 0.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_has_shadow(false)
        .with_taskbar(false)
        .with_active(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        centered: false,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "ORBIT",
        options,
        Box::new(move |cc| Ok(Box::new(app::OrbitApp::new(cc, cfg, instance, hidden)?))),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}
