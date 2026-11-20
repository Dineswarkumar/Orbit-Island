// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/ui/src/paint.rs.

//! Low-level painting: vertical gradient squircles, film grain, rim, and top highlight.

use crate::geometry::Squircle;
use egui::epaint::{Mesh, PathShape, PathStroke, Vertex};
use egui::{
    Color32, ColorImage, Context, Painter, Pos2, Shape, TextureHandle, TextureId, TextureOptions,
    TextureWrapMode, pos2, vec2,
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

#[allow(clippy::too_many_arguments)]
pub fn pill_surface(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    top: Color32,
    bottom: Color32,
    rim: Color32,
    highlight: Color32,
    noise_alpha: u8,
    grain_px: f32,
) {
    fill_gradient(painter, outline, top, bottom);
    if noise_alpha > 0 {
        let noise = noise_texture(painter.ctx());
        fill_noise(painter, outline, noise.id(), noise_alpha, grain_px);
    }
    painter.add(PathShape::closed_line(
        outline.to_vec(),
        PathStroke::new(1.0, rim),
    ));
    top_highlight(painter, shape, highlight);
}

pub fn top_highlight(painter: &Painter, shape: &Squircle, color: Color32) {
    let r = shape.radius;
    let y = shape.rect.min.y + 0.75;
    let x0 = shape.rect.min.x + r;
    let x1 = shape.rect.max.x - r;
    if x1 <= x0 {
        return;
    }
    let mid = 0.5 * (x0 + x1);
    let fade = Color32::TRANSPARENT;
    let mut mesh = Mesh::default();
    let h = 1.0;
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
