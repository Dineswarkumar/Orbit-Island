// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/paint.rs.

//! Low-level painting: vertical gradient squircles, film grain, rim, and top highlight.

use crate::geometry::Squircle;
use egui::epaint::{Mesh, PathShape, PathStroke, RectShape, Vertex};
use egui::{
    Color32, ColorImage, Context, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind,
    TextureHandle, TextureId, TextureOptions, TextureWrapMode, pos2, vec2,
};

const NOISE_SIZE: usize = 128;

pub fn fill_gradient(painter: &Painter, outline: &[Pos2], top: Color32, bottom: Color32) {
    if outline.len() < 3 {
        return;
    }
    let (min_y, max_y) = outline.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    let span = (max_y - min_y).max(1.0);
    let color_at = |y: f32| lerp_color(top, bottom, (y - min_y) / span);
    let mut mesh = Mesh::default();
    let center = centroid(outline);
    mesh.colored_vertex(center, color_at(center.y));
    for p in outline {
        mesh.colored_vertex(*p, color_at(p.y));
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

pub fn fill_noise(
    painter: &Painter,
    outline: &[Pos2],
    texture: TextureId,
    alpha: u8,
    grain_px: f32,
) {
    if outline.len() < 3 || alpha == 0 {
        return;
    }
    let tint = Color32::from_white_alpha(alpha);
    let scale = painter.ctx().pixels_per_point() / (NOISE_SIZE as f32 * grain_px.max(1.0));
    let uv = |p: Pos2| pos2(p.x * scale, p.y * scale);
    let mut mesh = Mesh::with_texture(texture);
    let center = centroid(outline);
    mesh.vertices.push(Vertex {
        pos: center,
        uv: uv(center),
        color: tint,
    });
    for p in outline {
        mesh.vertices.push(Vertex {
            pos: *p,
            uv: uv(*p),
            color: tint,
        });
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

/// Draws a soft multi-step drop shadow beneath `shape` using pure egui shapes.
pub fn drop_shadow(painter: &Painter, shape: &Squircle, shadow_color: Color32) {
    if shadow_color.a() == 0 {
        return;
    }
    const LAYERS: [(f32, f32, f32); 4] = [
        (5.5, 4.0, 0.10),
        (4.0, 2.8, 0.18),
        (2.5, 1.6, 0.28),
        (1.2, 0.7, 0.38),
    ];
    let [r, g, b, base_a] = shadow_color.to_srgba_unmultiplied();
    for (dy, expand, weight) in LAYERS {
        let a = (base_a as f32 * weight).round().clamp(0.0, 255.0) as u8;
        if a == 0 {
            continue;
        }
        let s = Squircle::new(
            shape.rect.expand(expand).translate(vec2(0.0, dy)),
            (shape.radius + expand).max(2.0),
        )
        .with_exponent(shape.exponent);
        let pts = s.outline();
        painter.add(Shape::convex_polygon(
            pts,
            Color32::from_rgba_unmultiplied(r, g, b, a),
            PathStroke::NONE,
        ));
    }
}

fn paint_rim_and_highlights(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    rim: Color32,
    inner_rim: Color32,
    highlight: Color32,
) {
    if shape.rect.width() > 6.0 && shape.rect.height() > 6.0 && inner_rim.a() > 0 {
        let inner = Squircle::new(shape.rect.shrink(1.0), (shape.radius - 1.0).max(1.0))
            .with_exponent(shape.exponent);
        painter.add(PathShape::closed_line(
            inner.outline(),
            PathStroke::new(0.85, inner_rim),
        ));
    }
    painter.add(PathShape::closed_line(
        outline.to_vec(),
        PathStroke::new(1.0, rim),
    ));
    top_highlight(painter, shape, highlight);
}

#[allow(clippy::too_many_arguments)]
pub fn pill_surface(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    top: Color32,
    bottom: Color32,
    rim: Color32,
    inner_rim: Color32,
    highlight: Color32,
    shadow: Color32,
    noise_alpha: u8,
    grain_px: f32,
) {
    drop_shadow(painter, shape, shadow);
    fill_gradient(painter, outline, top, bottom);
    if noise_alpha > 0 {
        let noise = noise_texture(painter.ctx());
        fill_noise(painter, outline, noise.id(), noise_alpha, grain_px);
    }
    paint_rim_and_highlights(painter, shape, outline, rim, inner_rim, highlight);
}

/// Draws the 3D drop shadow, inner edge highlight, rim, and top specular highlight
/// with an unfilled interior so the Composition host-blur shows cleanly behind it.
pub fn glass_frame(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    shadow: Color32,
    rim: Color32,
    inner_rim: Color32,
    highlight: Color32,
) {
    drop_shadow(painter, shape, shadow);
    paint_rim_and_highlights(painter, shape, outline, rim, inner_rim, highlight);
}

pub fn top_highlight(painter: &Painter, shape: &Squircle, color: Color32) {
    let r = shape.radius;
    let y = shape.rect.min.y + 0.85;
    let x0 = shape.rect.min.x + r;
    let x1 = shape.rect.max.x - r;
    if x1 <= x0 {
        return;
    }
    let mid = 0.5 * (x0 + x1);
    let fade = Color32::TRANSPARENT;
    let mut mesh = Mesh::default();
    let h = 1.25;
    for (x, c) in [(x0, fade), (mid, color), (x1, fade)] {
        mesh.colored_vertex(pos2(x, y), c);
        mesh.colored_vertex(pos2(x, y + h), c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    mesh.add_triangle(2, 3, 4);
    mesh.add_triangle(3, 4, 5);
    painter.add(Shape::mesh(mesh));
}

/// Paints a textured rectangle with rounded corners and crisp border.
pub fn rounded_texture(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    tex: TextureId,
    alpha: f32,
    border_color: Color32,
) {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    let tint = Color32::from_white_alpha(a);
    let shape = RectShape::filled(rect, CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8), tint)
        .with_texture(tex, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
    painter.add(shape);
    if border_color.a() > 0 {
        painter.rect_stroke(rect, radius, Stroke::new(1.0, border_color), StrokeKind::Inside);
    }
}

fn centroid(points: &[Pos2]) -> Pos2 {
    let sum = points
        .iter()
        .fold(vec2(0.0, 0.0), |acc, p| acc + p.to_vec2());
    (sum / points.len() as f32).to_pos2()
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

pub fn noise_texture(ctx: &Context) -> TextureHandle {
    let key = egui::Id::NULL.with("orbit-noise");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        return tex;
    }
    let mut seed: u32 = 0x9E37_79B9;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 24) as i32
    };
    let mut pixels = Vec::with_capacity(NOISE_SIZE * NOISE_SIZE);
    for _ in 0..NOISE_SIZE * NOISE_SIZE {
        let v = next() - next();
        let a = v.unsigned_abs().min(255) as u8;
        pixels.push(if v >= 0 {
            Color32::from_rgba_premultiplied(a, a, a, a)
        } else {
            Color32::from_rgba_premultiplied(0, 0, 0, a)
        });
    }
    let image = ColorImage::new([NOISE_SIZE, NOISE_SIZE], pixels);
    let tex = ctx.load_texture(
        "orbit-noise",
        image,
        TextureOptions {
            wrap_mode: TextureWrapMode::Repeat,
            ..TextureOptions::NEAREST
        },
    );
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    tex
}
