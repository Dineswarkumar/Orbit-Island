// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/diag.rs.

//! Logging, panic hook, and native crash handler setup.

use orbit_core::config;
use orbit_platform::system;
use std::path::PathBuf;

pub const LOG_FILE: &str = "orbit-egui.log";
const PREV_LOG_FILE: &str = "orbit-egui.prev.log";
const DEFAULT_FILTER: &str = "info,egui_glow=warn,glow=warn";

pub fn init() {
    use tracing_subscriber::EnvFilter;
    let filter =
        EnvFilter::try_from_env("ORBIT_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let path = log_path();
    let file = path.as_ref().and_then(|path| {
        std::fs::create_dir_all(path.parent()?).ok()?;
        let _ = std::fs::rename(path, path.with_file_name(PREV_LOG_FILE));
        std::fs::File::create(path).ok()
    });
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_thread_names(true);
    match file {
        Some(f) if !cfg!(debug_assertions) => {
            builder.with_writer(std::sync::Mutex::new(f)).init();
        }
        _ => builder.init(),
    }
    if let Some(path) = path {
        orbit_platform::crash::install(path);
    }
    install_panic_hook();
}

pub fn log_path() -> Option<PathBuf> {
    config::dir().ok().map(|d| d.join(LOG_FILE))
}

fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string payload>".to_owned());
        let at = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        tracing::error!(
            "PANIC on thread '{}' at {at}: {msg}\n{}",
            thread.name().unwrap_or("?"),
            std::backtrace::Backtrace::force_capture()
        );
        default(info);
    }));
}

pub fn log_startup(hidden: bool) {
    tracing::info!(
        "orbit-egui {} on {}, hidden={}, exe={}",
        env!("CARGO_PKG_VERSION"),
        system::windows_version(),
        hidden,
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    );
}
