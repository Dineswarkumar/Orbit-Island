// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/watcher.rs.

//! 1 Hz system watcher thread: polls monitor work area, theme, animations,
//! and battery, and wakes the UI ONLY when a value changes.

use crossbeam_channel::Sender;
use orbit_core::events::{
    BatteryStatus, CoreEvent, MonitorInfo, MonitorRect, SystemSnapshot,
};
use orbit_platform::{monitor, power, system};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

fn to_monitor_info(m: monitor::Monitor) -> MonitorInfo {
    let work = monitor::primary_work_area();
    MonitorInfo {
        id: m.id,
        bounds: MonitorRect {
            left: m.bounds.left,
            top: m.bounds.top,
            right: m.bounds.right,
            bottom: m.bounds.bottom,
        },
        work: MonitorRect {
            left: work.left,
            top: work.top,
            right: work.right,
            bottom: work.bottom,
        },
        dpi: m.dpi,
    }
}

pub fn sample() -> SystemSnapshot {
    SystemSnapshot {
        monitor: monitor::primary().ok().map(to_monitor_info),
        light_theme: system::apps_use_light_theme(),
        animations: system::animations_enabled(),
        transparency_enabled: system::transparency_enabled(),
        battery: power::battery().map(|b| BatteryStatus {
            percent: b.percent,
            charging: b.charging,
        }),
    }
}

pub struct Watcher {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    pub fn spawn(tx: Sender<CoreEvent>, ctx: egui::Context) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let initial = sample();
        let thread = std::thread::Builder::new()
            .name("orbit-watcher".into())
            .spawn(move || {
                let mut last = initial;
                while !flag.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1000));
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let now = sample();
                    if last != now {
                        last = now;
                        if tx.send(CoreEvent::SystemChanged(now)).is_err() {
                            break;
                        }
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                }
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
