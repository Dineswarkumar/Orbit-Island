// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied and adapted from nook/crates/app/src/overlay.rs.

//! Debug overlay (feature `debug-overlay`): shows frame count, CPU time,
//! frame interval, and repaint cause. Never requests repaints itself.

use egui::{Align2, Color32, Rect, Ui, pos2, vec2};
use orbit_ui::theme::{SIZE_LABEL, Theme};
use std::time::Instant;

#[derive(Default)]
pub struct DebugOverlay {
    frame_start: Option<Instant>,
    last_start: Option<Instant>,
    last_cpu_ms: f32,
    interval_ms: f32,
    frames: u64,
}

impl DebugOverlay {
    pub fn begin(&mut self) {
        let now = Instant::now();
        if let Some(prev) = self.last_start {
            self.interval_ms = (now - prev).as_secs_f32() * 1000.0;
        }
        self.last_start = Some(now);
        self.frame_start = Some(now);
        self.frames += 1;
    }

    pub fn show(&mut self, ui: &mut Ui) -> Rect {
        let ctx = ui.ctx().clone();
        let window = ctx.content_rect();
        let rect = Rect::from_min_size(
            pos2(window.min.x + 8.0, window.max.y - 28.0),
            vec2(320.0, 22.0),
        );
        let causes = ctx.repaint_causes();
        let cause = causes
            .first()
            .map(|c| {
                let file = c.file.rsplit(['/', '\\']).next().unwrap_or(c.file);
                format!("{file}:{} {}", c.line, c.reason)
            })
            .unwrap_or_else(|| "input".into());
        let text = format!(
            "#{}  cpu {:.1} ms  Δ {:.0} ms  {}",
            self.frames, self.last_cpu_ms, self.interval_ms, cause
        );
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("orbit-debug"),
        ));
        painter.rect_filled(rect, 6.0, Color32::from_black_alpha(200));
        painter.text(
            pos2(rect.min.x + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            Theme::font(SIZE_LABEL),
            Color32::from_rgb(160, 255, 170),
        );
        if let Some(start) = self.frame_start.take() {
            self.last_cpu_ms = start.elapsed().as_secs_f32() * 1000.0;
        }
        rect
    }
}
