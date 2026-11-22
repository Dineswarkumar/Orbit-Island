// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/main.rs.

//! ORBIT Dynamic Island native egui application (Gate 1 skeleton).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(feature = "bughunt-probe")]
mod bughunt;
mod diag;
#[cfg(feature = "debug-overlay")]
mod overlay;
mod tray;
mod watcher;

use anyhow::{Context as _, Result};
use orbit_core::config::{Config, GlassMode};
use orbit_ui::pill::WINDOW_HEIGHT;
use std::sync::{Arc, OnceLock};

pub(crate) const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-64.png");

pub(crate) fn app_icon_data() -> Option<Arc<egui::IconData>> {
    static ICON: OnceLock<Option<Arc<egui::IconData>>> = OnceLock::new();
    ICON.get_or_init(|| {
        let rgba = orbit_services::thumbnails::decode_image_bytes_rgba(APP_ICON_PNG, 64)?;
        Some(Arc::new(egui::IconData {
            rgba: rgba.rgba,
            width: rgba.width as u32,
            height: rgba.height as u32,
        }))
    })
    .clone()
}

fn parse_glass_override() -> Option<GlassMode> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let raw = if let Some(rest) = arg.strip_prefix("--glass-mode=") {
            Some(rest.to_string())
        } else if arg == "--glass-mode" {
            args.next()
        } else {
            None
        };
        if let Some(val) = raw {
            return match val.to_ascii_lowercase().as_str() {
                "normal" => Some(GlassMode::Normal),
                "realistic" => Some(GlassMode::Realistic),
                "blurred" => Some(GlassMode::Blurred),
                "opaque" => Some(GlassMode::Opaque),
                _ => None,
            };
        }
    }
    None
}

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
    let glass_override = parse_glass_override();
    let cfg = Config::load().context("loading config")?;
    diag::log_startup(hidden);

    let mut viewport = egui::ViewportBuilder::default()
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
        .with_drag_and_drop(true)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);
    if let Some(icon) = app_icon_data() {
        viewport = viewport.with_icon(icon);
    }

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
        Box::new(move |cc| {
            Ok(Box::new(app::OrbitApp::new(
                cc,
                cfg,
                instance,
                hidden,
                glass_override,
            )?))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}
