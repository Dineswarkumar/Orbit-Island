// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/anim.rs.

//! Spring & tween animation helpers with strict repaint discipline:
//! `request_repaint()` is called only while an animation is still moving.

use egui::{Context, Id};
use orbit_core::config::MotionPreset;

#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub reduced: bool,
    pub preset: MotionPreset,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            reduced: false,
            preset: MotionPreset::LiquidSpring,
        }
    }
}

pub fn set_motion(ctx: &Context, motion: Motion) {
    ctx.data_mut(|d| d.insert_temp(Id::NULL.with("orbit-motion"), motion));
}

pub fn motion(ctx: &Context) -> Motion {
    ctx.data(|d| d.get_temp(Id::NULL.with("orbit-motion")))
        .unwrap_or_default()
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

/// Smooth 0..1 boolean tween stored in egui temp data.
/// Calls `ctx.request_repaint()` ONLY while `0.0 < value < 1.0`.
#[derive(Clone, Copy)]
struct TweenSlot {
    value: f32,
    from: f32,
    target: f32,
    elapsed: f32,
    duration: f32,
    last_time: f64,
}

#[derive(Clone, Copy)]
struct SpringSlot {
    value: f32,
    velocity: f32,
    target: f32,
    last_time: f64,
}

fn spring_params(preset: MotionPreset, opening: bool) -> Option<(f32, f32)> {
    match (preset, opening) {
        (MotionPreset::LiquidSpring, true) => Some((430.0, 30.0)),
        (MotionPreset::LiquidSpring, false) => Some((520.0, 40.0)),
        (MotionPreset::Snappy, true) => Some((640.0, 50.0)),
        (MotionPreset::Snappy, false) => Some((680.0, 54.0)),
        (MotionPreset::Bouncy, true) => Some((380.0, 21.0)),
        (MotionPreset::Bouncy, false) => Some((480.0, 34.0)),
        (MotionPreset::Smooth | MotionPreset::Reduced, _) => None,
    }
}

/// Semi-implicit 2-substep second-order spring integrator.
/// Returns `(next_value, next_velocity, still_moving)`.
#[allow(clippy::too_many_arguments)]
fn step_spring(
    mut value: f32,
    mut velocity: f32,
    target: f32,
    dt: f32,
    stiffness: f32,
    damping: f32,
    pos_eps: f32,
    vel_eps: f32,
) -> (f32, f32, bool) {
    let dt = dt.clamp(1.0 / 240.0, 0.032);
    let sub = dt * 0.5;
    for _ in 0..2 {
        let force = -stiffness * (value - target) - damping * velocity;
        velocity += force * sub;
        value += velocity * sub;
    }

    if !value.is_finite() || !velocity.is_finite() {
        return (target, 0.0, false);
    }

    if (value - target).abs() <= pos_eps && velocity.abs() <= vel_eps {
        (target, 0.0, false)
    } else {
        (value, velocity, true)
    }
}

fn tween_to(ctx: &Context, id: Id, target: f32, duration: f32) -> (f32, bool) {
    let now = ctx.input(|i| i.time);
    let duration = duration.max(0.0);
    let mut slot = ctx.data(|d| d.get_temp::<TweenSlot>(id)).unwrap_or(TweenSlot {
        value: target,
        from: target,
        target,
        elapsed: duration,
        duration,
        last_time: now,
    });

    let m = motion(ctx);
    if m.reduced || m.preset == MotionPreset::Reduced || duration == 0.0 {
        slot.value = target;
        slot.from = target;
        slot.target = target;
        slot.elapsed = duration;
        slot.duration = duration;
    } else {
        if (slot.target - target).abs() > f32::EPSILON {
            slot.from = slot.value;
            slot.target = target;
            slot.elapsed = 0.0;
            slot.duration = duration;
        }
        if slot.elapsed < slot.duration {
            slot.elapsed = (slot.elapsed + frame_dt(ctx, slot.last_time).max(1.0 / 240.0))
                .min(slot.duration);
            let t = (slot.elapsed / slot.duration).clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            slot.value = slot.from + (slot.target - slot.from) * eased;
            if slot.elapsed < slot.duration {
                ctx.request_repaint();
            } else {
                slot.value = slot.target;
            }
        }
    }

    slot.last_time = now;
    let moving = slot.elapsed < slot.duration;
    let value = slot.value;
    ctx.data_mut(|d| d.insert_temp(id, slot));
    (value, moving)
}

fn spring_to(
    ctx: &Context,
    id: Id,
    target: f32,
    opening: bool,
    pos_eps: f32,
    vel_eps: f32,
) -> (f32, bool) {
    let m = motion(ctx);
    if m.reduced || m.preset == MotionPreset::Reduced {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| {
            d.insert_temp(
                id,
                SpringSlot {
                    value: target,
                    velocity: 0.0,
                    target,
                    last_time: now,
                },
            );
        });
        return (target, false);
    }

    let Some((stiffness, damping)) = spring_params(m.preset, opening) else {
        return tween_to(ctx, id, target, if opening { 0.20 } else { 0.16 });
    };

    let now = ctx.input(|i| i.time);
    let mut slot = ctx.data(|d| d.get_temp::<SpringSlot>(id)).unwrap_or(SpringSlot {
        value: target,
        velocity: 0.0,
        target,
        last_time: now,
    });

    slot.target = target;
    let needs_step = (slot.value - target).abs() > pos_eps || slot.velocity.abs() > vel_eps;
    let moving = if needs_step {
        let dt = frame_dt(ctx, slot.last_time);
        let (next_val, next_vel, still_moving) = step_spring(
            slot.value,
            slot.velocity,
            target,
            dt,
            stiffness,
            damping,
            pos_eps,
            vel_eps,
        );
        slot.value = next_val;
        slot.velocity = next_vel;
        if still_moving {
            ctx.request_repaint();
        }
        still_moving
    } else {
        slot.value = target;
        slot.velocity = 0.0;
        false
    };

    slot.last_time = now;
    let value = slot.value;
    ctx.data_mut(|d| d.insert_temp(id, slot));
    (value, moving)
}

pub fn tween(ctx: &Context, id: Id, on: bool, duration: f32) -> f32 {
    tween_to(ctx, id, if on { 1.0 } else { 0.0 }, duration).0
}

/// Spring / ease-out boolean transition used for the island's open/close motion.
/// Repaints are requested only while the motion has not settled.
pub fn transition_tween(ctx: &Context, id: Id, on: bool) -> f32 {
    spring_to(ctx, id, if on { 1.0 } else { 0.0 }, on, 0.002, 0.015).0
}

/// Animates an arbitrary geometry value with the active `MotionPreset`.
/// Repaints are requested only until the value reaches its target.
pub fn transition_value(ctx: &Context, id: Id, target: f32, opening: bool) -> (f32, bool) {
    spring_to(ctx, id, target, opening, 0.12, 0.6)
}

/// Animates an element's drop-in progress (`0.0 -> 1.0`) whenever `visible`
/// transitions from `false` (or uninitialized) to `true`.
pub fn entry_drop_progress(ctx: &Context, id: Id, visible: bool) -> f32 {
    let vis_id = id.with("entry-vis");
    let drop_id = id.with("entry-drop");
    let was_visible = ctx.data(|d| d.get_temp::<bool>(vis_id)).unwrap_or(false);

    if !visible {
        ctx.data_mut(|d| d.insert_temp(vis_id, false));
        return 0.0;
    }

    if !was_visible {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| {
            d.insert_temp(vis_id, true);
            d.insert_temp(
                drop_id,
                SpringSlot {
                    value: 0.0,
                    velocity: 0.0,
                    target: 1.0,
                    last_time: now,
                },
            );
            d.insert_temp(
                drop_id,
                TweenSlot {
                    value: 0.0,
                    from: 0.0,
                    target: 1.0,
                    elapsed: 0.0,
                    duration: 0.20,
                    last_time: now,
                },
            );
        });
    }

    transition_tween(ctx, drop_id, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_out_has_no_overshoot_and_finishes_at_target() {
        let ease_out = |t: f32| 1.0 - (1.0 - t).powi(3);
        let samples = (0..=20).map(|step| ease_out(step as f32 / 20.0));
        let values: Vec<_> = samples.collect();
        assert_eq!(values.first().copied(), Some(0.0));
        assert_eq!(values.last().copied(), Some(1.0));
        assert!(values.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn spring_presets_converge_without_nan_or_runaway() {
        let presets = [
            MotionPreset::LiquidSpring,
            MotionPreset::Snappy,
            MotionPreset::Bouncy,
        ];
        let dts = [1.0 / 120.0, 1.0 / 60.0, 0.032];

        for preset in presets {
            for opening in [true, false] {
                let (stiffness, damping) = spring_params(preset, opening).expect("spring params");
                for &dt in &dts {
                    // Unit tween (0..1)
                    let mut value = if opening { 0.0 } else { 1.0 };
                    let mut velocity = 0.0;
                    let target = if opening { 1.0 } else { 0.0 };
                    let mut settled = false;
                    for _ in 0..300 {
                        let (v, vel, moving) = step_spring(
                            value,
                            velocity,
                            target,
                            dt,
                            stiffness,
                            damping,
                            0.002,
                            0.015,
                        );
                        assert!(v.is_finite() && vel.is_finite());
                        assert!(v.abs() < 2.0, "runaway value {v} for {preset:?}");
                        value = v;
                        velocity = vel;
                        if !moving {
                            settled = true;
                            break;
                        }
                    }
                    assert!(settled, "{preset:?} (opening={opening}, dt={dt}) failed to settle");
                    assert_eq!(value, target);
                    assert_eq!(velocity, 0.0);

                    // Pixel-scale value (160..420)
                    let mut px = if opening { 160.0 } else { 420.0 };
                    let mut px_vel = 0.0;
                    let px_target = if opening { 420.0 } else { 160.0 };
                    let mut px_settled = false;
                    for _ in 0..300 {
                        let (v, vel, moving) = step_spring(
                            px,
                            px_vel,
                            px_target,
                            dt,
                            stiffness,
                            damping,
                            0.12,
                            0.6,
                        );
                        assert!(v.is_finite() && vel.is_finite());
                        assert!((0.0..=600.0).contains(&v), "runaway px {v} for {preset:?}");
                        px = v;
                        px_vel = vel;
                        if !moving {
                            px_settled = true;
                            break;
                        }
                    }
                    assert!(
                        px_settled,
                        "{preset:?} px (opening={opening}, dt={dt}) failed to settle"
                    );
                    assert_eq!(px, px_target);
                    assert_eq!(px_vel, 0.0);
                }
            }
        }
    }

    #[test]
    fn reduced_motion_snaps_immediately() {
        let ctx = Context::default();
        set_motion(
            &ctx,
            Motion {
                reduced: false,
                preset: MotionPreset::Reduced,
            },
        );
        let id = Id::new("reduced-test");
        assert_eq!(entry_drop_progress(&ctx, id, false), 0.0);
        assert_eq!(entry_drop_progress(&ctx, id, true), 1.0);
        assert_eq!(tween(&ctx, Id::new("reduced-tween"), true, 0.25), 1.0);
    }
}
