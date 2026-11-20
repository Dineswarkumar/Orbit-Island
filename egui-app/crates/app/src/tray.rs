// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/tray.rs (Gate 1: Quit only).

//! System tray icon and menu (Gate 1: Quit only).

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use orbit_ui::geometry::Squircle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayCommand {
    Quit,
}

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    /// Must run on the UI thread that pumps Win32 messages.
    pub fn new(tx: Sender<TrayCommand>, ctx: egui::Context) -> Result<Self> {
        let quit = MenuItem::with_id("quit", "Quit ORBIT", true, None);
        let menu = Menu::new();
        menu.append(&quit).context("building tray menu")?;

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id.as_ref() == "quit" && tx.send(TrayCommand::Quit).is_ok() {
                ctx.request_repaint();
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("ORBIT")
            .with_icon(tray_icon()?)
            .build()
            .context("creating tray icon")?;
        Ok(Self { _icon: icon })
    }
}

const APP_ICON_RESOURCE: u16 = 1;

fn tray_icon() -> Result<Icon> {
    let px = orbit_platform::system::small_icon_size();
    match Icon::from_resource(APP_ICON_RESOURCE, Some((px, px))) {
        Ok(icon) => Ok(icon),
        Err(e) => {
            tracing::warn!("tray icon resource: {e}; using fallback pill glyph");
            tray_glyph()
        }
    }
}

fn tray_glyph() -> Result<Icon> {
    const N: u32 = 32;
    let pill = Squircle::new(
        egui::Rect::from_min_max(egui::pos2(3.0, 10.0), egui::pos2(29.0, 22.0)),
        6.0,
    )
    .with_exponent(2.0);
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let mut hits = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let p = egui::pos2(
                        x as f32 + (sx as f32 + 0.5) / 4.0,
                        y as f32 + (sy as f32 + 0.5) / 4.0,
                    );
                    if pill.contains(p) {
                        hits += 1;
                    }
                }
            }
            let a = (hits * 255 / 16) as u8;
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
    }
    Icon::from_rgba(rgba, N, N).context("tray icon pixels")
}
