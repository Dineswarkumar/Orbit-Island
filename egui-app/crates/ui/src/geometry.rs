// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/geometry.rs.

//! Shell geometry: the continuous-corner squircle/capsule outline and hit-testing.

use egui::{Pos2, Rect, Vec2, pos2, vec2};

/// Exponent of the superellipse corners. 2.0 is a circular arc (true capsule);
/// 4.0 gives continuous squircle corners.
pub const CORNER_EXPONENT: f32 = 4.0;

/// Points sampled per corner when building the outline polygon.
pub const CORNER_SEGMENTS: usize = 14;

/// A rounded rectangle whose corners are superellipse quadrants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Squircle {
    pub rect: Rect,
    pub radius: f32,
    /// Corner superellipse exponent (2.0 = circular arc).
    pub exponent: f32,
}

impl Squircle {
    pub fn new(rect: Rect, radius: f32) -> Self {
        let max_r = 0.5 * rect.width().min(rect.height());
        Self {
            rect,
            radius: radius.clamp(0.0, max_r.max(0.0)),
            exponent: CORNER_EXPONENT,
        }
    }

    pub fn with_exponent(mut self, exponent: f32) -> Self {
        self.exponent = exponent.max(2.0);
        self
    }

    pub fn contains(&self, p: Pos2) -> bool {
        let r = self.radius;
        let half = self.rect.size() * 0.5;
        let d = (p - self.rect.center()).abs();
        if d.x > half.x || d.y > half.y {
            return false;
        }
        if r <= 0.0 {
            return true;
        }
        let qx = ((d.x - (half.x - r)) / r).max(0.0);
        let qy = ((d.y - (half.y - r)) / r).max(0.0);
        qx.powf(self.exponent) + qy.powf(self.exponent) <= 1.0 + 1e-4
    }

    pub fn outline(&self) -> Vec<Pos2> {
        let mut pts = Vec::with_capacity(4 * (CORNER_SEGMENTS + 1));
        self.outline_into(&mut pts);
        pts
    }

    pub fn outline_into(&self, pts: &mut Vec<Pos2>) {
        pts.clear();
        let r = self.radius;
        let Rect { min, max } = self.rect;
        let corners = [
            (pos2(max.x - r, min.y + r), -90.0_f32), // top-right
            (pos2(max.x - r, max.y - r), 0.0),       // bottom-right
            (pos2(min.x + r, max.y - r), 90.0),      // bottom-left
            (pos2(min.x + r, min.y + r), 180.0),     // top-left
        ];
        let e = 2.0 / self.exponent;
        let shape = |v: f32| v.signum() * v.abs().powf(e);
        for (c, start) in corners {
            for s in 0..=CORNER_SEGMENTS {
                let deg = start + 90.0 * s as f32 / CORNER_SEGMENTS as f32;
                let (sin, cos) = deg.to_radians().sin_cos();
                pts.push(c + vec2(shape(cos), shape(sin)) * r);
            }
        }
    }

    pub fn translate(self, delta: Vec2) -> Self {
        Self {
            rect: self.rect.translate(delta),
            ..self
        }
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_clamped_to_half_short_side() {
        let s = Squircle::new(Rect::from_min_size(Pos2::ZERO, vec2(160.0, 34.0)), 50.0);
        assert_eq!(s.radius, 17.0);
    }

    #[test]
    fn capsule_contains_center_and_excludes_outside_corners() {
        let s = Squircle::new(Rect::from_min_size(Pos2::ZERO, vec2(160.0, 34.0)), 17.0)
            .with_exponent(2.0);
        assert!(s.contains(pos2(80.0, 17.0)));
        assert!(s.contains(pos2(1.0, 17.0)));
        assert!(!s.contains(pos2(0.5, 0.5)));
        assert!(!s.contains(pos2(161.0, 17.0)));
    }
}
