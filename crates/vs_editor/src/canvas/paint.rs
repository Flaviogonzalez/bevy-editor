//! What the canvas paints directly instead of building elements: the grid, the wires and the
//! selection box, all captured in a `Scene` so the paint closure owns its data.


use gpui_kit::{
    Bounds, PathBuilder, Hsla,
    Window, point, px, size,
};

use crate::canvas::layout::V2;
use super::Scene;

pub(super) fn paint_scene(scene: &Scene, bounds: Bounds<gpui_kit::Pixels>, window: &mut Window) {
    let o = V2::new(bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
    let w = bounds.size.width.as_f32();
    let h = bounds.size.height.as_f32();
    let quad = |x: f32, y: f32, qw: f32, qh: f32, color: Hsla, window: &mut Window| {
        window.paint_quad(
            gpui_kit::fill(Bounds::new(point(px(x), px(y)), size(px(qw), px(qh))), color),
        );
    };

    // Grid: minor lines every 20 units, major every 100.
    let step = 20.0 * scene.zoom;
    if step >= 6.0 {
        let first_x = (-scene.pan.x / step).floor() as i64;
        let first_y = (-scene.pan.y / step).floor() as i64;
        let cols = (w / step).ceil() as i64 + 1;
        let rows = (h / step).ceil() as i64 + 1;
        for i in first_x..first_x + cols {
            let x = scene.pan.x + i as f32 * step;
            let color = if i % 5 == 0 { scene.grid_major } else { scene.grid_minor };
            quad(o.x + x, o.y, 1.0, h, color, window);
        }
        for j in first_y..first_y + rows {
            let y = scene.pan.y + j as f32 * step;
            let color = if j % 5 == 0 { scene.grid_major } else { scene.grid_minor };
            quad(o.x, o.y + y, w, 1.0, color, window);
        }
    }

    for wire in &scene.wires {
        let a = wire.a + o;
        let b = wire.b + o;
        let dx = ((b.x - a.x).abs() * 0.5).max(40.0 * scene.zoom);
        let mut path = PathBuilder::stroke(px(wire.width));
        path.move_to(point(px(a.x), px(a.y)));
        path.cubic_bezier_to(
            point(px(b.x), px(b.y)),
            point(px(a.x + dx), px(a.y)),
            point(px(b.x - dx), px(b.y)),
        );
        if let Ok(path) = path.build() {
            window.paint_path(path, wire.color);
        }
    }

    if let Some(sel) = &scene.selection {
        window.paint_quad(
            gpui_kit::fill(
                Bounds::new(point(px(o.x + sel.min.x), px(o.y + sel.min.y)), size(px(sel.w()), px(sel.h()))),
                scene.selection_color.opacity(0.08),
            )
            .border_widths(px(1.0))
            .border_color(scene.selection_color),
        );
    }
}
