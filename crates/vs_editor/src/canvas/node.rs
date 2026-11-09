//! One node drawn as a card: a header tinted with its category's color, a row per pin with its
//! dot on the card's edge, and inline fields for unconnected values.


use gpui_kit::{
    App, Div, FontWeight, Hsla, div, prelude::*, px, rgba,
};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{NodeId, PinRef, PinType, Severity, Side, Value, library};

use crate::editor::{Drag, EditTarget, Editor};
use crate::canvas::layout::{self, Hit, NodeLayout, V2};
use crate::theme;
use crate::ui::MONO;

impl Editor {
    /// How a pin should look right now: under the pointer, or ruled out as the end of the wire
    /// being dragged (an incompatible pin, or one on the same side as the wire's anchor).
    fn pin_look(&self, node: NodeId, pin: &str, side: Side) -> PinLook {
        let me = PinRef::new(node, pin.to_string());
        let hovered = self.canvas.hover == Hit::Pin(me.clone(), side);
        let dimmed = match &self.canvas.drag {
            Drag::Wire { anchor, side: anchor_side, .. } => {
                if *anchor == me && *anchor_side == side {
                    false
                } else if *anchor_side == side {
                    true
                } else {
                    match side {
                        Side::Input => !self.doc.graph.can_link(anchor, &me),
                        Side::Output => !self.doc.graph.can_link(&me, anchor),
                    }
                }
            }
            _ => false,
        };
        PinLook { hovered: hovered && !dimmed, dimmed }
    }

    /// One node as a card: a header tinted with its category's color, then a row per pin with
    /// the pin's dot on the card's edge, its name, and an inline field for unconnected values.
    /// Selection draws a ring around the card, so the border stays free to show problems.
    pub(super) fn node_element(&self, node: &vs_graph::Node, l: &NodeLayout, flag: Option<Severity>, cx: &App) -> Div {
        let z = self.doc.zoom;
        let def = library::find(&node.kind);
        let (title, subject) = match (node.var.as_ref().or(node.field.as_ref()), def) {
            (subject, Some(d)) => (d.title.to_string(), subject.cloned()),
            _ => (format!("? {}", node.kind), None),
        };
        let accent: Hsla = theme::category(def.map_or("", |d| d.category)).into();
        let selected = self.doc.selected.contains(&node.id);
        let hovered = matches!(&self.canvas.hover, Hit::Header(id) | Hit::Body(id) | Hit::Field(id, _) if *id == node.id)
            && matches!(self.canvas.drag, Drag::None);
        let flag_color = flag.map(|f| if f == Severity::Error { theme::error(cx) } else { theme::warning(cx) });
        let border = match (flag_color, selected, hovered) {
            (Some(c), _, _) => c,
            (None, true, _) => theme::selection(cx),
            (None, false, true) => theme::dim(cx).opacity(0.7),
            (None, false, false) => theme::border(cx),
        };
        let screen = self.to_screen(l.rect.min);
        let rel = |p: V2| (p - l.rect.min) * z;
        let (w, h) = (l.rect.w() * z, l.rect.h() * z);
        let radius = 8.0 * z;

        let mut el = div()
            .absolute()
            .left(px(screen.x))
            .top(px(screen.y))
            .w(px(w))
            .h(px(h))
            .rounded(px(radius))
            .bg(theme::node_bg(cx))
            .text_size(px(12.0 * z))
            .map(|d| if selected { d.shadow_lg() } else { d.shadow_md() })
            // The outline is its own first child rather than this div's border: GPUI paints a
            // div's border after its children, which would draw it across the pins on the edge.
            .child(div().absolute().size_full().rounded(px(radius)).border_1().border_color(border))
            .when(selected, |d| {
                let gap = 3.0;
                d.child(
                    div()
                        .absolute()
                        .left(px(-gap - 1.0))
                        .top(px(-gap - 1.0))
                        .w(px(w + 2.0 * gap + 2.0))
                        .h(px(h + 2.0 * gap + 2.0))
                        .rounded(px(radius + gap))
                        .border_2()
                        .border_color(theme::selection(cx).opacity(0.45)),
                )
            })
            .child(
                div()
                    .absolute()
                    // Inset by the outline's width, since it's painted over the outline.
                    .left(px(1.0))
                    .top(px(1.0))
                    .w(px(w - 2.0))
                    .h(px(layout::HEADER_H * z - 1.0))
                    .rounded_t(px(radius - 1.0))
                    .bg(accent.opacity(0.28))
                    .border_b_1()
                    .border_color(theme::border(cx))
                    .px(px(10.0 * z))
                    .flex()
                    .items_center()
                    .gap(px(6.0 * z))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(div().flex_none().w(px(8.0 * z)).h(px(8.0 * z)).rounded_full().bg(accent))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .gap(px(4.0 * z))
                            .overflow_hidden()
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(12.5 * z)).child(title))
                            .when_some(subject, |d, subject| {
                                d.child(div().text_size(px(12.5 * z)).text_color(theme::dim(cx)).child(subject))
                            }),
                    )
                    .when_some(flag.zip(flag_color), |d, (f, color)| {
                        let icon = if f == Severity::Error { IconName::CircleX } else { IconName::TriangleAlert };
                        d.child(Icon::new(icon).flex_none().size(px(13.0 * z)).text_color(color))
                    }),
            );

        let row_text_h = layout::ROW_H * z;
        let label_color = |look: PinLook| {
            if look.dimmed {
                theme::dim(cx).opacity(0.35)
            } else if look.hovered {
                theme::text(cx)
            } else {
                theme::dim(cx)
            }
        };
        for pin in &l.inputs {
            let c = rel(pin.center);
            let look = self.pin_look(node.id, &pin.def.name, Side::Input);
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, look, z, cx));
            if pin.def.ty != PinType::Exec || l.inputs.iter().filter(|p| p.def.ty == PinType::Exec).count() > 1 {
                el = el.child(
                    div()
                        .absolute()
                        .left(px(12.0 * z))
                        .top(px(c.y - row_text_h * 0.5))
                        .h(px(row_text_h))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_color(label_color(look))
                        .child(pin.def.name.clone()),
                );
            }
        }
        for pin in &l.outputs {
            let c = rel(pin.center);
            let look = self.pin_look(node.id, &pin.def.name, Side::Output);
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, look, z, cx));
            let label = if pin.def.ty == PinType::Exec && pin.def.name == "then" {
                String::new()
            } else {
                pin.def.name.clone()
            };
            el = el.child(
                div()
                    .absolute()
                    .right(px(12.0 * z))
                    .top(px(c.y - row_text_h * 0.5))
                    .h(px(row_text_h))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .text_color(label_color(look))
                    .child(label),
            );
        }
        for (pin, rect) in &l.fields {
            let value = node.values.get(pin);
            let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::Field(node.id, pin.clone()));
            let field_hovered = self.canvas.hover == Hit::Field(node.id, pin.clone()) && matches!(self.canvas.drag, Drag::None);
            let text = match (editing, value) {
                (Some(e), _) => format!("{}|", e.buffer),
                (None, Some(Value::Str(s))) => format!("\"{s}\""),
                (None, Some(v)) => v.display(),
                (None, None) => String::new(),
            };
            let p = rel(rect.min);
            let swatch = match value {
                Some(Value::Color { rgba: c }) if editing.is_none() => {
                    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                    Some(rgba(byte(c[0]) << 24 | byte(c[1]) << 16 | byte(c[2]) << 8 | byte(c[3])))
                }
                _ => None,
            };
            el = el.child(
                div()
                    .absolute()
                    .left(px(p.x))
                    .top(px(p.y))
                    .w(px(rect.w() * z))
                    .h(px(rect.h() * z))
                    .px(px(5.0 * z))
                    .rounded(px(4.0 * z))
                    .bg(theme::field_bg(cx))
                    .border_1()
                    .border_color(if editing.is_some() {
                        theme::selection(cx)
                    } else if field_hovered {
                        theme::dim(cx).opacity(0.7)
                    } else {
                        theme::border(cx)
                    })
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.0 * z))
                    .font_family(MONO)
                    .gap(px(4.0 * z))
                    .when_some(swatch, |d, color| {
                        d.child(
                            div()
                                .w(px(10.0 * z))
                                .h(px(10.0 * z))
                                .flex_none()
                                .rounded(px(2.0 * z))
                                .border_1()
                                .border_color(theme::border(cx))
                                .bg(color),
                        )
                    })
                    .child(text),
            );
        }
        el
    }
}

/// How a pin is drawn beyond its type and whether it's connected; see `Editor::pin_look`.
#[derive(Clone, Copy)]
struct PinLook {
    hovered: bool,
    dimmed: bool,
}

/// A pin on the card's edge: filled when connected, hollow when not; exec pins square, data pins
/// round. Under the pointer it grows and gets a halo of its color; ruled out, it fades.
fn pin_dot(center: V2, ty: PinType, connected: bool, look: PinLook, z: f32, cx: &App) -> Div {
    let r = layout::PIN_R * z * if look.hovered { 1.25 } else { 1.0 };
    let halo = r * 1.9;
    let base: Hsla = theme::pin(ty).into();
    let color = if look.dimmed { base.opacity(0.25) } else { base };
    let dot = div()
        .w(px(r * 2.0))
        .h(px(r * 2.0))
        .border(px(1.5 * z.max(0.7)))
        .border_color(color)
        .bg(if connected || look.hovered { color } else { theme::node_bg(cx) });
    let dot = if ty == PinType::Exec { dot.rounded(px(2.0 * z)) } else { dot.rounded_full() };
    div()
        .absolute()
        .left(px(center.x - halo))
        .top(px(center.y - halo))
        .w(px(halo * 2.0))
        .h(px(halo * 2.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .when(look.hovered, |d| d.bg(base.opacity(0.22)))
        .child(dot)
}
