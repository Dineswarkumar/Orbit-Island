// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/anim.rs and spring.rs.

//! Spring-physics and tween helpers with strict repaint discipline:
//! `request_repaint()` is called ONLY while a spring or tween is still moving.

use egui::{Context, Id};

#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub reduced: bool,
}

pub fn set_motion(ctx: &Context, motion: Motion) {
    ctx.data_mut(|d| d.insert_temp(Id::NULL.with("orbit-motion"), motion));
}

pub fn motion(ctx: &Context) -> Motion {
    ctx.data(|d| d.get_temp(Id::NULL.with("orbit-motion")))
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringParams {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringParams {
    pub fn critical(stiffness: f32) -> Self {
        let mass = 1.0;
        Self {
            stiffness,
            damping: 2.0 * (stiffness * mass).sqrt(),
            mass,
        }
    }

    pub fn with_overshoot(stiffness: f32, overshoot: f32) -> Self {
        let mass = 1.0;
        let zeta = damping_ratio_for_overshoot(overshoot);
        Self {
            stiffness,
            damping: 2.0 * zeta * (stiffness * mass).sqrt(),
            mass,
        }
    }

    pub fn snappy() -> Self {
        Self::critical(420.0)
    }

    pub fn panel_width() -> Self {
        Self::with_overshoot(320.0, 0.04)
    }

    pub fn panel_height() -> Self {
        Self::with_overshoot(250.0, 0.04)
    }

    pub fn placement_x() -> Self {
        Self::critical(340.0)
    }
}

impl Default for SpringParams {
    fn default() -> Self {
        Self::snappy()
    }
}

pub fn damping_ratio_for_overshoot(overshoot: f32) -> f32 {
    if overshoot <= 0.0 {
        return 1.0;
    }
    let ln = overshoot.min(0.99).ln();
    let pi = std::f32::consts::PI;
    (-ln / (pi * pi + ln * ln).sqrt()).clamp(0.0, 1.0)
}

const SUBSTEP: f32 = 1.0 / 480.0;
const MAX_DT: f32 = 1.0 / 15.0;
const REST_DISTANCE: f32 = 0.05;
const REST_VELOCITY: f32 = 0.05;

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    pub params: SpringParams,
}

impl Spring {
    pub fn new(value: f32, params: SpringParams) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            params,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    pub fn step(&mut self, dt: f32) -> bool {
        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }
        let prev_value = self.value;
        let mut offset = self.value - self.target;
        let mut remaining = dt.clamp(0.0, MAX_DT);
        let SpringParams {
            stiffness,
            damping,
            mass,
        } = self.params;
        let mass = mass.max(1e-4);
        while remaining > 0.0 {
            let h = remaining.min(SUBSTEP);
            let force = -stiffness * offset - damping * self.velocity;
            self.velocity += force / mass * h;
            offset += self.velocity * h;
            remaining -= h;
        }
        self.value = self.target + offset;
        let stalled = dt > 0.0
            && (self.value - prev_value).abs() == 0.0
            && self.velocity.abs() < 1.0;
        if self.is_settled() || stalled {
            self.value = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }

    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < REST_DISTANCE && self.velocity.abs() < REST_VELOCITY
    }
}

#[derive(Clone, Copy)]
struct SpringSlot {
    spring: Spring,
    last_time: f64,
}

fn frame_dt(ctx: &Context, last_time: f64) -> f32 {
    ctx.input(|i| {
        let gap = (i.time - last_time) as f32;
        if (0.0..0.1).contains(&gap) {
            gap
        } else {
            i.predicted_dt
        }
    })
}

/// Animates towards `target` with a spring. Requests repaint ONLY while moving.
pub fn spring(ctx: &Context, id: Id, target: f32, params: SpringParams) -> f32 {
    spring_with_status(ctx, id, target, params).0
}

/// Like [`spring`], also returning whether the spring is currently still moving.
pub fn spring_with_status(ctx: &Context, id: Id, target: f32, params: SpringParams) -> (f32, bool) {
    let now = ctx.input(|i| i.time);
    let mut slot = ctx
        .data(|d| d.get_temp::<SpringSlot>(id))
        .unwrap_or(SpringSlot {
            spring: Spring::new(target, params),
            last_time: now,
        });
    slot.spring.params = params;
    slot.spring.set_target(target);
    let moving = if motion(ctx).reduced {
        slot.spring.snap(target);
        false
    } else {
        let dt = frame_dt(ctx, slot.last_time);
        let active = slot.spring.step(dt);
        if active {
            ctx.request_repaint();
        }
        active
    };
    slot.last_time = now;
    let value = slot.spring.value;
    ctx.data_mut(|d| d.insert_temp(id, slot));
    (value, moving)
}

/// Returns `true` if a stored spring is still moving towards its target.
pub fn spring_is_moving(ctx: &Context, id: Id) -> bool {
    ctx.data(|d| d.get_temp::<SpringSlot>(id))
        .is_some_and(|slot| !slot.spring.is_settled())
}

/// Smooth 0..1 boolean tween stored in egui temp data.
/// Calls `ctx.request_repaint()` ONLY while `0.0 < value < 1.0`.
#[derive(Clone, Copy)]
struct TweenSlot {
    value: f32,
    last_time: f64,
}

pub fn tween(ctx: &Context, id: Id, on: bool, duration: f32) -> f32 {
    let target = if on { 1.0 } else { 0.0 };
    if motion(ctx).reduced || duration <= 0.0 {
        ctx.data_mut(|d| {
            d.insert_temp(
                id,
                TweenSlot {
                    value: target,
                    last_time: ctx.input(|i| i.time),
                },
            );
        });
        return target;
    }
    let now = ctx.input(|i| i.time);
    let mut slot = ctx
        .data(|d| d.get_temp::<TweenSlot>(id))
        .unwrap_or(TweenSlot {
            value: target,
            last_time: now,
        });
    if (slot.value - target).abs() > 1e-4 {
        let dt = frame_dt(ctx, slot.last_time).max(1.0 / 240.0);
        let step = dt / duration;
        if on {
            slot.value = (slot.value + step).min(1.0);
        } else {
            slot.value = (slot.value - step).max(0.0);
        }
        if (slot.value - target).abs() > 1e-4 {
            ctx.request_repaint();
        } else {
            slot.value = target;
        }
    } else {
        slot.value = target;
    }
    slot.last_time = now;
    ctx.data_mut(|d| d.insert_temp(id, slot));
    let t = slot.value;
    // Smoothstep easing.
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settled_spring_stops_moving() {
        let mut s = Spring::new(0.0, SpringParams::snappy());
        assert!(!s.step(1.0 / 60.0));
        s.set_target(1.0);
        assert!(s.step(1.0 / 60.0));
        for _ in 0..240 {
            s.step(1.0 / 60.0);
        }
        assert!(s.is_settled());
        assert!(!s.step(1.0 / 60.0));
    }

    #[test]
    fn panel_springs_overshoot_about_four_percent_and_settle() {
        for params in [SpringParams::panel_width(), SpringParams::panel_height()] {
            let mut s = Spring::new(0.0, params);
            s.set_target(100.0);
            let mut peak = 0.0f32;
            for _ in 0..240 {
                s.step(1.0 / 60.0);
                peak = peak.max(s.value);
            }
            assert!(peak > 102.5 && peak < 105.5, "unexpected peak {peak}");
            assert!(s.is_settled());
        }
    }

    #[test]
    fn large_coordinate_spring_settles_without_ulp_stall() {
        let mut s = Spring::new(620.0, SpringParams::placement_x());
        s.set_target(1356.0);
        for _ in 0..120 {
            if !s.step(1.0 / 60.0) {
                break;
            }
        }
        assert!(s.is_settled(), "stalled at value={} vel={}", s.value, s.velocity);
        assert_eq!(s.value, 1356.0);
        assert!(!s.step(1.0 / 60.0));
    }
}
