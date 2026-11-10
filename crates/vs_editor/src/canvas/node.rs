//! One node drawn as a card: a header tinted with its category's color, a row per pin with its
//! dot on the card's edge, and inline fields for unconnected values. A component card draws the
//! same frame with its name and fields inside instead of pins.


use gpui_kit::{
    App, Div, FontWeight, Hsla, Rgba, div, prelude::*, px, rgba,
};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{ComponentDef, NodeId, PinRef, PinType, Severity, Side, Value, library};

use crate::canvas::Drag;
use crate::canvas::groups::Group;
use crate::editor::{EditTarget, Editor};
use crate::canvas::layout::{self, CardPart, Hit, NodeLayout, Rect, V2};
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
    /// `stripe` is the color of the system the node belongs to, drawn along its top edge; a
    /// node of no system, or of several, has none.
    ///
    /// `entry` is the system an event node starts. Such a node is drawn as the entry of that
    /// system's function: its title is the function's name, and its first row says what makes
    /// it run. An event that didn't compile to a system keeps its plain title.
    pub(super) fn node_element(
        &self,
        node: &vs_graph::Node,
        l: &NodeLayout,
        flag: Option<Severity>,
        stripe: Option<Hsla>,
        entry: Option<&Group>,
        cx: &App,
    ) -> Div {
        let z = self.doc.zoom;
        let def = library::find(&node.kind);
        let (title, subject) = match (node.var.as_ref().or(node.field.as_ref()), def) {
            _ if entry.is_some() => (format!("fn {}", entry.map_or("", |e| e.system.name.as_str())), None),
            // The card shows its name in its own field, under the header.
            (_, Some(d)) if d.is_definition() => (d.title.to_string(), None),
            (_, Some(d)) if d.is_copy() => (d.title.to_string(), Some("read-only".to_string())),
            (subject, Some(d)) =>(d.title.to_string(), subject.cloned()),
            _ => (format!("? {}", node.kind), None),
        };
        let accent: Hsla = theme::category(def.map_or("", |d| d.category)).into();
        let selected = self.doc.selected.contains(&node.id);
        let hovered = matches!(&self.canvas.hover, Hit::Header(id) | Hit::Body(id) | Hit::Field(id, _) | Hit::Card(id, _) if *id == node.id)
            && matches!(self.canvas.drag, Drag::None);
        let flag_color = flag.map(|f| if f == Severity::Error { theme::error(cx) } else { theme::warning(cx) });
        let border = match (flag_color, selected, hovered) {
            (Some(c), _, _) => c,
            (None, true, _) => theme::selection(cx),
            (None, false, true) => theme::dim(cx).opacity(0.7),
            (None, false, false) => theme::border(cx),
        };
        let screen = self.doc.to_screen(l.rect.min);
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
            } else if entry.is_some() && pin.def.name == "delta" {
                // The system reads it from Bevy's clock: `time.delta_secs()`.
                "delta · Res<Time>".to_string()
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
        // An event has no inputs, so the left of its first row is free for what makes it run.
        if let (Some(group), Some(first)) = (entry, l.outputs.first()) {
            let c = rel(first.center);
            el = el.child(
                div()
                    .absolute()
                    .left(px(12.0 * z))
                    .top(px(c.y - row_text_h * 0.5))
                    .h(px(row_text_h))
                    .max_w(px(w * 0.75))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(11.0 * z))
                    .text_color(theme::dim(cx))
                    .child(group.trigger.clone()),
            );
        }
        for (pin, rect) in &l.fields {
            let value = node.values.get(pin);
            let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::Field(node.id, pin.clone()));
            let field_hovered = self.canvas.hover == Hit::Field(node.id, pin.clone()) && matches!(self.canvas.drag, Drag::None);
            let text = match (editing, value) {
                (Some(e), _) => format!("{}|", e.buffer),
                (None, Some(v)) => shown(v),
                (None, None) => String::new(),
            };
            let p = rel(rect.min);
            let swatch = value.filter(|_| editing.is_none()).and_then(swatch);
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
                    .when_some(swatch, |d, color| d.child(swatch_box(color, z, cx)))
                    .child(text),
            );
        }
        let card = match (&node.component, &node.copy_of) {
            (Some(def), _) => Some((def.clone(), None)),
            (None, Some(name)) => Some(layout::copy_view(&self.doc.graph, name)),
            (None, None) => None,
        };
        if let Some((def, source)) = card {
            let view = CardView { def: &def, source: source.as_deref(), readonly: l.readonly };
            for (part, rect) in &l.parts {
                el = el.child(self.card_part(node.id, &view, *part, rect, l, cx));
            }
        }
        // Last, so it's over the header's tint.
        if let Some(color) = stripe {
            el = el.child(
                div()
                    .absolute()
                    .left(px(1.0))
                    .top(px(1.0))
                    .w(px(w - 2.0))
                    .h(px(STRIPE_H * z))
                    .rounded_t(px(radius - 1.0))
                    .bg(color),
            );
        }
        el
    }

    /// One part of a component card, drawn like a node's inline fields: a boxed text that shows
    /// the edit buffer while it has the keyboard. On a read-only copy the texts aren't boxed,
    /// since they can't be edited, and the link to the declaring script sits beside the name.
    fn card_part(&self, id: NodeId, view: &CardView, part: CardPart, rect: &Rect, l: &NodeLayout, cx: &App) -> Div {
        let def = view.def;
        let z = self.doc.zoom;
        let p = (rect.min - l.rect.min) * z;
        let hovered = self.canvas.hover == Hit::Card(id, part) && matches!(self.canvas.drag, Drag::None);
        let target = match part {
            CardPart::Name => Some(EditTarget::ComponentName(id)),
            CardPart::FieldName(i) => Some(EditTarget::ComponentField(id, i)),
            CardPart::Value(i) => Some(EditTarget::ComponentValue(id, i)),
            _ => None,
        };
        let editing = self.editing.as_ref().filter(|e| Some(&e.target) == target.as_ref());
        let field = |i: usize| def.fields.get(i);
        let base = div()
            .absolute()
            .left(px(p.x))
            .top(px(p.y))
            .w(px(rect.w() * z))
            .h(px(rect.h() * z))
            .flex()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(px(11.0 * z));
        let boxed = |d: Div| {
            if view.readonly {
                return d.px(px(5.0 * z)).gap(px(4.0 * z));
            }
            d.px(px(5.0 * z))
                .gap(px(4.0 * z))
                .rounded(px(4.0 * z))
                .bg(theme::field_bg(cx))
                .border_1()
                .border_color(if editing.is_some() {
                    theme::selection(cx)
                } else if hovered {
                    theme::dim(cx).opacity(0.7)
                } else {
                    theme::border(cx)
                })
        };
        let text = |shown: String| match editing {
            Some(e) => format!("{}|", e.buffer),
            None => shown,
        };
        match part {
            CardPart::Name => boxed(base)
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(12.0 * z))
                .child(text(def.name.clone())),
            CardPart::FieldName(i) => {
                boxed(base).font_family(MONO).child(text(field(i).map(|f| f.name.clone()).unwrap_or_default()))
            }
            CardPart::Value(i) => {
                let value = field(i).map(|f| &f.value);
                let swatch = value.filter(|_| editing.is_none()).and_then(swatch);
                boxed(base)
                    .font_family(MONO)
                    .when_some(swatch, |d, color| d.child(swatch_box(color, z, cx)))
                    .child(text(value.map(shown).unwrap_or_default()))
            }
            CardPart::Type(i) => {
                let ty = field(i).map_or(PinType::Any, |f| f.value.ty());
                let color: Hsla = theme::pin(ty).into();
                base.justify_center()
                    .rounded(px(4.0 * z))
                    .text_size(px(10.0 * z))
                    .text_color(color)
                    .when(hovered, |d| d.bg(color.opacity(0.18)))
                    .child(short_type(ty))
            }
            CardPart::Remove(_) => base
                .justify_center()
                .rounded(px(4.0 * z))
                .text_color(if hovered { theme::error(cx) } else { theme::dim(cx) })
                .child(Icon::new(IconName::Close).size(px(11.0 * z))),
            CardPart::Source => {
                let (label, color) = match view.source {
                    Some(file) => (format!("defined in {file}"), theme::selection(cx)),
                    None if self.doc.graph.defined_components().any(|d| d.name == def.name) => {
                        ("declared on this canvas".to_string(), theme::dim(cx))
                    }
                    None => ("not declared in the project".to_string(), theme::warning(cx)),
                };
                base.justify_end()
                    .text_color(color)
                    .when(hovered && view.source.is_some(), |d| d.underline())
                    .child(label)
            }
            CardPart::AddField => base
                .gap(px(4.0 * z))
                .text_color(if hovered { theme::text(cx) } else { theme::dim(cx) })
                .child(Icon::new(IconName::Plus).size(px(11.0 * z)))
                .child("Add field"),
        }
    }
}

/// How tall the stripe along a node's top edge is, in graph units. It has the color of the
/// system the node belongs to (see `groups`).
const STRIPE_H: f32 = 3.0;

/// What a component card draws: the component, and for a read-only copy, the file that
/// declares it (`None` when this script does, or nothing does).
struct CardView<'a> {
    def: &'a ComponentDef,
    source: Option<&'a str>,
    readonly: bool,
}

/// A value as a field shows it: text in quotes, so an empty string or one with spaces reads
/// as text.
fn shown(value: &Value) -> String {
    match value {
        Value::Str(s) => format!("\"{s}\""),
        v => v.display(),
    }
}

/// The color a color value shows in its field's swatch.
fn swatch(value: &Value) -> Option<Rgba> {
    let Value::Color { rgba: c } = value else { return None };
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(rgba(byte(c[0]) << 24 | byte(c[1]) << 16 | byte(c[2]) << 8 | byte(c[3])))
}

fn swatch_box(color: Rgba, z: f32, cx: &App) -> Div {
    div()
        .w(px(10.0 * z))
        .h(px(10.0 * z))
        .flex_none()
        .rounded(px(2.0 * z))
        .border_1()
        .border_color(theme::border(cx))
        .bg(color)
}

/// A type's name short enough for the card's type column.
fn short_type(ty: PinType) -> &'static str {
    match ty {
        PinType::Number => "num",
        PinType::Bool => "bool",
        PinType::String => "text",
        PinType::Vec3 => "vec3",
        PinType::Quat => "rot",
        PinType::Color => "color",
        PinType::Exec | PinType::Any => "?",
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
