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
    const LAYERS: [(f32, f32, f32); 6] = [
        (9.0, 5.5, 0.08),
        (6.5, 3.8, 0.14),
        (4.2, 2.2, 0.22),
        (2.4, 1.1, 0.34),
        (1.0, 0.4, 0.44),
        (0.4, 0.0, 0.52),
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

fn squircle_x_half(s: &Squircle, y: f32) -> f32 {
    let half = s.rect.size() * 0.5;
    let r = s.radius.min(half.x).min(half.y);
    let dy = (y - s.rect.center().y).abs();
    if dy >= half.y {
        return (half.x - r).max(0.0);
    }
    if r <= 0.0 || dy <= half.y - r {
        return half.x;
    }
    let qy = ((dy - (half.y - r)) / r).clamp(0.0, 1.0);
    let qx = (1.0 - qy.powf(s.exponent)).max(0.0).powf(1.0 / s.exponent);
    (half.x - r) + r * qx
}

fn scale_unmultiplied_alpha(color: Color32, scale: f32) -> Color32 {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let scaled_a = (a as f32 * scale.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, scaled_a)
}

fn top_specular_sheen(painter: &Painter, shape: &Squircle, highlight: Color32) {
    if highlight.a() == 0 || shape.rect.width() <= 8.0 || shape.rect.height() <= 8.0 {
        return;
    }
    let inner = Squircle::new(shape.rect.shrink(1.0), (shape.radius - 1.0).max(1.0))
        .with_exponent(shape.exponent);
    let sheen_h = (inner.rect.height() * 0.28)
        .clamp(5.0, 24.0)
        .min(inner.rect.height() * 0.45);
    if sheen_h <= 1.0 {
        return;
    }

    const STEPS: usize = 5;
    let y0 = inner.rect.min.y;
    let cx = inner.rect.center().x;
    let mut mesh = Mesh::default();

    for step in 0..=STEPS {
        let t = step as f32 / STEPS as f32;
        let y = y0 + sheen_h * t;
        let dx = squircle_x_half(&inner, y).max(0.5);
        let falloff = (1.0 - t).powi(2);
        let center_c = scale_unmultiplied_alpha(highlight, 0.28 * falloff);
        let edge_c = scale_unmultiplied_alpha(highlight, 0.10 * falloff);

        mesh.colored_vertex(pos2(cx - dx, y), edge_c);
        mesh.colored_vertex(pos2(cx, y), center_c);
        mesh.colored_vertex(pos2(cx + dx, y), edge_c);
    }

    for step in 0..STEPS {
        let row = (step * 3) as u32;
        let next = row + 3;
        // Left quad
        mesh.add_triangle(row, row + 1, next);
        mesh.add_triangle(row + 1, next + 1, next);
        // Right quad
        mesh.add_triangle(row + 1, row + 2, next + 1);
        mesh.add_triangle(row + 2, next + 2, next + 1);
    }

    painter.add(Shape::mesh(mesh));
}

fn bottom_inner_shadow(painter: &Painter, shape: &Squircle) {
    if shape.rect.width() <= 6.0 || shape.rect.height() <= 6.0 {
        return;
    }
    let inner = Squircle::new(shape.rect.shrink(1.0), (shape.radius - 1.0).max(1.0))
        .with_exponent(shape.exponent);
    let outline = inner.outline();
    if outline.len() < 3 {
        return;
    }
    let y_threshold = inner.rect.min.y + inner.rect.height() * (2.0 / 3.0);
    let mut lower_pts = Vec::with_capacity(outline.len());
    let n = outline.len();
    for i in 0..n {
        let a = outline[i];
        let b = outline[(i + 1) % n];
        let a_in = a.y >= y_threshold;
        let b_in = b.y >= y_threshold;
        if !a_in && b_in {
            let denom = b.y - a.y;
            let t = if denom.abs() > 1e-4 {
                ((y_threshold - a.y) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            lower_pts.push(pos2(a.x + (b.x - a.x) * t, y_threshold));
            lower_pts.push(b);
        } else if a_in && b_in {
            if lower_pts.is_empty() {
                lower_pts.push(a);
            }
            lower_pts.push(b);
        } else if a_in && !b_in {
            let denom = b.y - a.y;
            let t = if denom.abs() > 1e-4 {
                ((y_threshold - a.y) / denom).clamp(0.0, 1.0)
            } else {
                1.0
            };
            lower_pts.push(pos2(a.x + (b.x - a.x) * t, y_threshold));
        }
    }
    if lower_pts.len() >= 2 {
        painter.add(PathShape::line(
            lower_pts,
            PathStroke::new(1.25, Color32::from_black_alpha(68)),
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
    bottom_inner_shadow(painter, shape);
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
    top_specular_sheen(painter, shape, highlight);
    paint_rim_and_highlights(painter, shape, outline, rim, inner_rim, highlight);
}

/// Draws the 3D drop shadow, curved specular sheen, inner edge highlight, rim, and top
/// specular highlight with an unfilled interior so the Composition host-blur shows cleanly.
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
    top_specular_sheen(painter, shape, highlight);
    paint_rim_and_highlights(painter, shape, outline, rim, inner_rim, highlight);
}

/// Draws the blur-disabled glass fallback: a single tinted body with the glass rim,
/// curved specular sheen, inner edge, top highlight, and shadow.
pub fn tinted_fallback_surface(
    painter: &Painter,
    shape: &Squircle,
    fill: Color32,
    shadow: Color32,
    rim: Color32,
    inner_rim: Color32,
    highlight: Color32,
) {
    let outline = shape.outline();
    drop_shadow(painter, shape, shadow);
    painter.add(Shape::convex_polygon(outline.clone(), fill, PathStroke::NONE));
    top_specular_sheen(painter, shape, highlight);
    paint_rim_and_highlights(painter, shape, &outline, rim, inner_rim, highlight);
}

/// Draws a convex 3D opaque surface with drop shadow, vertical gradient,
/// curved top specular sheen, inner bevel, rim, and top catchlight.
#[allow(clippy::too_many_arguments)]
pub fn opaque_3d_surface(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    top: Color32,
    bottom: Color32,
    rim: Color32,
    inner_rim: Color32,
    highlight: Color32,
    shadow: Color32,
) {
    drop_shadow(painter, shape, shadow);
    fill_gradient(painter, outline, top, bottom);
    top_specular_sheen(painter, shape, highlight);
    paint_rim_and_highlights(painter, shape, outline, rim, inner_rim, highlight);
}

pub fn top_highlight(painter: &Painter, shape: &Squircle, color: Color32) {
    let r = shape.radius;
    let y = shape.rect.min.y + 0.85;
    let x0 = shape.rect.min.x + r;
    let x1 = shape.rect.max.x - r;
    if x1 <= x0 || color.a() == 0 {
        return;
    }
    let mid = 0.5 * (x0 + x1);
    let left_shoulder = x0 + (mid - x0) * 0.55;
    let right_shoulder = mid + (x1 - mid) * 0.45;
    let fade = Color32::TRANSPARENT;

    let [cr, cg, cb, ca] = color.to_srgba_unmultiplied();
    let catchlight = Color32::from_rgba_unmultiplied(
        cr.saturating_add(((255 - cr) as f32 * 0.35) as u8),
        cg.saturating_add(((255 - cg) as f32 * 0.35) as u8),
        cb.saturating_add(((255 - cb) as f32 * 0.35) as u8),
        ((ca as f32 * 1.35).round().min(255.0)) as u8,
    );

    let mut mesh = Mesh::default();
    let h = 1.65;
    let stops = [
        (x0, fade),
        (left_shoulder, color),
        (mid, catchlight),
        (right_shoulder, color),
        (x1, fade),
    ];
    for (x, c) in stops {
        mesh.colored_vertex(pos2(x, y), c);
        mesh.colored_vertex(pos2(x, y + h), fade);
    }
    for i in 0..4u32 {
        let b = i * 2;
        mesh.add_triangle(b, b + 1, b + 2);
        mesh.add_triangle(b + 1, b + 2, b + 3);
    }
    painter.add(Shape::mesh(mesh));
}

fn tint_preserving_alpha(base: Color32, tint: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [br, bg, bb, ba] = base.to_srgba_unmultiplied();
    let [tr, tg, tb, _] = tint.to_srgba_unmultiplied();
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(br, tr), l(bg, tg), l(bb, tb), ba)
}

#[allow(clippy::too_many_arguments)]
fn neu_shadow_two_layer(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    ambient_dy: f32,
    ambient_expand: f32,
    ambient_alpha: u8,
    contact_dy: f32,
    contact_alpha: u8,
) {
    let ambient = Squircle::new(
        rect.expand(ambient_expand).translate(vec2(0.0, ambient_dy)),
        (radius + ambient_expand).max(2.0),
    )
    .with_exponent(2.0);
    painter.add(Shape::convex_polygon(
        ambient.outline(),
        Color32::from_black_alpha(ambient_alpha),
        PathStroke::NONE,
    ));

    let contact = Squircle::new(rect.translate(vec2(0.0, contact_dy)), radius).with_exponent(2.0);
    painter.add(Shape::convex_polygon(
        contact.outline(),
        Color32::from_black_alpha(contact_alpha),
        PathStroke::NONE,
    ));
}

fn paint_hover_halo(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    accent: Option<Color32>,
    expand: f32,
    stroke_width: f32,
) {
    let halo_color = match accent {
        Some(tint) => scale_unmultiplied_alpha(tint, 0.30),
        None => Color32::from_white_alpha(38),
    };
    if halo_color.a() > 0 {
        let halo = Squircle::new(rect.expand(expand), (radius + expand).max(2.0))
            .with_exponent(2.0);
        painter.add(PathShape::closed_line(
            halo.outline(),
            PathStroke::new(stroke_width, halo_color),
        ));
    }
}

/// Reusable 3D bento card primitive with 2-layer drop shadow, convex gradient,
/// optional accent tint / hover lift, inner top highlight, bottom inner lip, and crisp rim border.
#[allow(clippy::too_many_arguments)]
pub fn card_3d(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    top: Color32,
    bottom: Color32,
    border: Color32,
    top_highlight: Color32,
    accent_tint: Option<Color32>,
    hovered: bool,
) {
    if rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }

    neu_shadow_two_layer(painter, rect, radius, 2.5, 0.6, 44, 1.0, 74);

    let mut effective_top = top;
    let mut effective_bottom = bottom;
    let mut effective_border = border;

    if let Some(tint) = accent_tint {
        let tint_amt = if hovered { 0.16 } else { 0.05 };
        effective_top = tint_preserving_alpha(effective_top, tint, tint_amt);
        effective_bottom = tint_preserving_alpha(effective_bottom, tint, tint_amt * 0.5);
        if hovered {
            effective_border = crate::theme::mix(border, tint, 0.55);
        }
    } else if hovered {
        effective_border = tint_preserving_alpha(border, Color32::WHITE, 0.28);
    }

    if hovered {
        effective_top = tint_preserving_alpha(effective_top, Color32::WHITE, 0.11);
    }

    let card_shape = Squircle::new(rect, radius).with_exponent(2.0);
    let outline = card_shape.outline();
    fill_gradient(painter, &outline, effective_top, effective_bottom);

    let clamped_r = card_shape.radius;
    let hl_x0 = rect.min.x + clamped_r;
    let hl_x1 = rect.max.x - clamped_r;
    if hl_x1 > hl_x0 {
        if top_highlight.a() > 0 {
            let y = rect.min.y + 1.0;
            let hl_color = if hovered {
                tint_preserving_alpha(top_highlight, Color32::WHITE, 0.28)
            } else {
                top_highlight
            };
            painter.line_segment(
                [pos2(hl_x0, y), pos2(hl_x1, y)],
                Stroke::new(1.0, hl_color),
            );
        }
        let lip_y = rect.max.y - 1.0;
        painter.line_segment(
            [pos2(hl_x0, lip_y), pos2(hl_x1, lip_y)],
            Stroke::new(1.0, Color32::from_black_alpha(64)),
        );
    }

    if hovered {
        paint_hover_halo(painter, rect, radius, accent_tint, 1.25, 2.2);
    }

    painter.add(PathShape::closed_line(
        outline,
        PathStroke::new(1.0, effective_border),
    ));
}

/// Reusable tactile neumorphic button with convex unpressed elevation, hover glow,
/// and recessed pressed indentation.
#[allow(clippy::too_many_arguments)]
pub fn neu_button(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    base_top: Color32,
    base_bottom: Color32,
    border: Color32,
    accent: Option<Color32>,
    hovered: bool,
    pressed: bool,
) {
    if rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }

    let btn_shape = Squircle::new(rect, radius).with_exponent(2.0);
    let outline = btn_shape.outline();
    let clamped_r = btn_shape.radius;
    let hl_x0 = rect.min.x + clamped_r;
    let hl_x1 = rect.max.x - clamped_r;

    if pressed {
        let mut eff_top = tint_preserving_alpha(base_bottom, Color32::BLACK, 0.14);
        let mut eff_bottom = tint_preserving_alpha(base_top, Color32::BLACK, 0.06);
        let mut eff_border = border;
        if let Some(acc) = accent {
            eff_top = tint_preserving_alpha(eff_top, acc, 0.14);
            eff_bottom = tint_preserving_alpha(eff_bottom, acc, 0.08);
            eff_border = crate::theme::mix(border, acc, 0.42);
        }

        fill_gradient(painter, &outline, eff_top, eff_bottom);

        if hl_x1 > hl_x0 {
            painter.line_segment(
                [pos2(hl_x0, rect.min.y + 1.0), pos2(hl_x1, rect.min.y + 1.0)],
                Stroke::new(1.5, Color32::from_black_alpha(86)),
            );
            painter.line_segment(
                [pos2(hl_x0, rect.max.y - 1.0), pos2(hl_x1, rect.max.y - 1.0)],
                Stroke::new(0.85, Color32::from_white_alpha(20)),
            );
        }

        painter.add(PathShape::closed_line(
            outline,
            PathStroke::new(1.0, eff_border),
        ));
        return;
    }

    neu_shadow_two_layer(painter, rect, radius, 2.0, 0.5, 40, 0.8, 66);

    let mut eff_top = base_top;
    let mut eff_bottom = base_bottom;
    let mut eff_border = border;

    if let Some(acc) = accent {
        let tint_amt = if hovered { 0.22 } else { 0.10 };
        eff_top = tint_preserving_alpha(eff_top, acc, tint_amt);
        eff_bottom = tint_preserving_alpha(eff_bottom, acc, tint_amt * 0.6);
        if hovered {
            eff_border = crate::theme::mix(border, acc, 0.52);
        }
    } else if hovered {
        eff_border = tint_preserving_alpha(border, Color32::WHITE, 0.30);
    }

    if hovered {
        eff_top = tint_preserving_alpha(eff_top, Color32::WHITE, 0.10);
        eff_bottom = tint_preserving_alpha(eff_bottom, Color32::WHITE, 0.04);
    }

    fill_gradient(painter, &outline, eff_top, eff_bottom);

    if hl_x1 > hl_x0 {
        let spec_alpha = if hovered { 92 } else { 62 };
        painter.line_segment(
            [pos2(hl_x0, rect.min.y + 1.0), pos2(hl_x1, rect.min.y + 1.0)],
            Stroke::new(1.0, Color32::from_white_alpha(spec_alpha)),
        );
        painter.line_segment(
            [pos2(hl_x0, rect.max.y - 1.0), pos2(hl_x1, rect.max.y - 1.0)],
            Stroke::new(1.0, Color32::from_black_alpha(52)),
        );
    }

    if hovered {
        paint_hover_halo(painter, rect, radius, accent, 1.0, 2.0);
    }

    painter.add(PathShape::closed_line(
        outline,
        PathStroke::new(1.0, eff_border),
    ));
}

/// Reusable 3D recessed progress bar with halo glow, vivid core fill, and top specular strip.
pub fn progress_bar_3d(
    painter: &Painter,
    rect: Rect,
    frac: f32,
    accent: Color32,
    track: Color32,
) {
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }

    let r = rect.height() * 0.5;
    painter.rect_filled(rect, r, track);

    let x0 = rect.min.x + r.min(rect.width() * 0.25);
    let x1 = rect.max.x - r.min(rect.width() * 0.25);
    if x1 > x0 {
        painter.line_segment(
            [pos2(x0, rect.min.y + 0.6), pos2(x1, rect.min.y + 0.6)],
            Stroke::new(0.85, Color32::from_black_alpha(48)),
        );
        painter.line_segment(
            [pos2(x0, rect.max.y - 0.6), pos2(x1, rect.max.y - 0.6)],
            Stroke::new(0.75, Color32::from_white_alpha(20)),
        );
    }

    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }

    let fill_w = (rect.width() * frac).max(rect.height().min(rect.width()));
    let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w.min(rect.width()), rect.height()));
    let fill_r = (fill_rect.height() * 0.5).min(fill_rect.width() * 0.5);

    let halo_color = scale_unmultiplied_alpha(accent, 0.24);
    if halo_color.a() > 0 {
        painter.rect_filled(fill_rect.expand(1.5), fill_r + 1.5, halo_color);
    }

    painter.rect_filled(fill_rect, fill_r, accent);

    let spec_alpha = ((accent.a() as f32 / 255.0) * 0.42 * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    if spec_alpha > 0 && fill_rect.width() > 3.0 && fill_rect.height() >= 3.0 {
        let inset_x = (fill_r * 0.45).min(fill_rect.width() * 0.25);
        let spec_h = (fill_rect.height() * 0.28).clamp(1.0, 2.0);
        let spec_rect = Rect::from_min_max(
            pos2(fill_rect.min.x + inset_x, fill_rect.min.y + 0.6),
            pos2(fill_rect.max.x - inset_x, fill_rect.min.y + 0.6 + spec_h),
        );
        if spec_rect.width() > 1.0 {
            painter.rect_filled(
                spec_rect,
                spec_h * 0.5,
                Color32::from_white_alpha(spec_alpha),
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn count_shapes(mut draw: impl FnMut(&Painter)) -> usize {
        let ctx = Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))),
            ..Default::default()
        };
        let mut count = 0;
        let mut out = ctx.run_ui(raw, |ui| {
            draw(ui.painter());
        });
        count += out.shapes.len();
        out.textures_delta.clear();
        count
    }

    #[test]
    fn drop_shadow_emits_six_layers_and_skips_transparent() {
        let shape = Squircle::new(Rect::from_min_size(pos2(20.0, 20.0), vec2(160.0, 34.0)), 17.0);
        let layers = count_shapes(|p| drop_shadow(p, &shape, Color32::from_black_alpha(120)));
        assert_eq!(layers, 6);

        let empty = count_shapes(|p| drop_shadow(p, &shape, Color32::TRANSPARENT));
        assert_eq!(empty, 0);
    }

    #[test]
    fn pill_surface_renders_with_bottom_inner_shadow() {
        let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(200.0, 40.0));
        let shape = Squircle::new(rect, 20.0);
        let outline = shape.outline();
        let shapes = count_shapes(|p| {
            pill_surface(
                p,
                &shape,
                &outline,
                Color32::from_rgb(30, 32, 38),
                Color32::from_rgb(6, 7, 10),
                Color32::from_white_alpha(34),
                Color32::from_white_alpha(16),
                Color32::from_white_alpha(58),
                Color32::from_black_alpha(120),
                0,
                1.0,
            );
        });
        assert!(shapes >= 10);
    }

    #[test]
    fn card_3d_and_neu_button_render_states() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(120.0, 44.0));
        let base_top = Color32::from_rgb(34, 37, 46);
        let base_bottom = Color32::from_rgb(16, 18, 24);
        let border = Color32::from_white_alpha(28);
        let highlight = Color32::from_white_alpha(32);
        let accent = Some(Color32::from_rgb(56, 189, 248));

        let card_normal = count_shapes(|p| {
            card_3d(p, rect, 12.0, base_top, base_bottom, border, highlight, accent, false);
        });
        let card_hovered = count_shapes(|p| {
            card_3d(p, rect, 12.0, base_top, base_bottom, border, highlight, accent, true);
        });
        assert!(card_hovered > card_normal);

        let btn_idle = count_shapes(|p| {
            neu_button(p, rect, 10.0, base_top, base_bottom, border, accent, false, false);
        });
        let btn_hovered = count_shapes(|p| {
            neu_button(p, rect, 10.0, base_top, base_bottom, border, accent, true, false);
        });
        let btn_pressed = count_shapes(|p| {
            neu_button(p, rect, 10.0, base_top, base_bottom, border, accent, false, true);
        });
        assert!(btn_hovered > btn_idle);
        assert!(btn_pressed < btn_idle);
    }
}
