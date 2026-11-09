//! The node canvas: the grid, the wires, the nodes and the add-node menu, plus the mouse and
//! wheel handlers that drive them.
//!
//! - `layout`: node geometry in graph units and hit testing, shared by drawing and input.
//! - `node`: one node drawn as a card, with its pins and inline fields.
//! - `add_menu`: the searchable menu that adds a node.
//! - `paint`: what is painted directly rather than as elements (grid, wires, selection box).

use std::collections::BTreeMap;

use gpui_kit::{
    App, Context, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Hsla, ScrollEvent, canvas, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{Compiled, NodeId, PinType, Severity, Side};

use crate::editor::{Drag, Editor};
use layout::{NodeLayout, Rect, V2};
use crate::theme;
use paint::paint_scene;

mod add_menu;
pub mod layout;
mod node;
mod paint;

pub const MENU_ROWS: usize = 10;

struct WireSeg {
    a: V2,
    b: V2,
    color: Hsla,
    width: f32,
}

/// Everything the canvas paint closure needs, captured by value.
struct Scene {
    pan: V2,
    zoom: f32,
    wires: Vec<WireSeg>,
    selection: Option<Rect>,
    grid_minor: Hsla,
    grid_major: Hsla,
    selection_color: Hsla,
}

impl Editor {
    pub(crate) fn canvas_area(
        &self,
        layouts: &[NodeLayout],
        compiled: &Compiled,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut flagged: BTreeMap<NodeId, Severity> = BTreeMap::new();
        for p in &compiled.problems {
            if let Some(id) = p.node {
                let entry = flagged.entry(id).or_insert(p.severity);
                if p.severity == Severity::Error {
                    *entry = Severity::Error;
                }
            }
        }

        let scene = self.scene(layouts, cx);
        let cell = self.canvas.clone();
        let painter = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let origin = V2::new(bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
                let extent = V2::new(bounds.size.width.as_f32(), bounds.size.height.as_f32());
                cell.set((origin, extent));
                paint_scene(&scene, bounds, window);
            },
        )
        .absolute()
        .size_full();

        let mut area = div()
            .id("canvas")
            .flex_1()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .bg(theme::canvas_bg(cx))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    let additive = e.modifiers.shift || e.modifiers.control;
                    this.left_down(this.to_local(e.position), additive);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.right_down(this.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.middle_down(this.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if this.mouse_move(this.to_local(e.position), e.pressed_button.is_some()) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_scroll(cx.listener(|this, e: &ScrollEvent, _, cx| {
                let dy = e.delta.pixel_delta(px(40.0)).y.as_f32();
                this.scroll(this.to_local(e.position), dy);
                cx.notify();
            }))
            .child(painter);

        for l in layouts {
            if let Some(node) = self.graph.node(l.id) {
                area = area.child(self.node_element(node, l, flagged.get(&l.id).copied(), cx));
            }
        }
        if self.graph.nodes.is_empty() && self.menu.is_none() {
            area = area.child(self.empty_state(cx));
        }
        if self.menu.is_some() {
            area = area.child(self.menu_element(cx));
        }
        area
    }

    /// What the canvas shows while the graph has no nodes: what to do next, instead of a blank grid.
    fn empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let card = Empty::new()
            .flex_none()
            .w(px(380.0))
            .bg(theme::panel_bg(cx))
            .border_1()
            .header(
                EmptyHeader::new()
                    .media(
                        EmptyMedia::new()
                            .with_variant(EmptyMediaVariant::Icon)
                            .child(Icon::new(IconName::Network)),
                    )
                    .title(EmptyTitle::new().child("No nodes yet"))
                    .description(EmptyDescription::new().child(concat!(
                        "A script starts with an event, such as On Update. ",
                        "Add one, or right-click the canvas or press Tab at any time.",
                    ))),
            )
            .content(
                EmptyContent::new().flex_row().justify_center().gap_2().child(
                    Button::new("empty-add-node").primary().icon(IconName::Plus).label("Add node").on_click(
                        cx.listener(|this, _, _, cx| {
                            let (_, size) = this.canvas.get();
                            this.open_menu(size * 0.5, None);
                            cx.notify();
                        }),
                    ),
                )
                .child(Button::new("empty-open").outline().icon(IconName::FolderOpen).label("Open script").on_click(
                    cx.listener(|this, _, window, cx| {
                        this.open(window);
                        cx.notify();
                    }),
                )),
            );

        // Covers the canvas only to center the card; the canvas still gets clicks around it.
        div().absolute().size_full().flex().items_center().justify_center().child(
            div()
                // Keeps a click on the card from also starting a selection on the canvas.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(card),
        )
    }

    fn scene(&self, layouts: &[NodeLayout], cx: &App) -> Scene {
        let find = |id: NodeId| layouts.iter().find(|l| l.id == id);
        let mut wires = Vec::new();
        for link in &self.graph.links {
            let (Some(a), Some(b)) = (find(link.from.node), find(link.to.node)) else { continue };
            let (Some(pa), Some(pb)) = (
                a.pin_center(&link.from.pin, Side::Output),
                b.pin_center(&link.to.pin, Side::Input),
            ) else {
                continue;
            };
            let ty = self.graph.pin(&link.from, Side::Output).map_or(PinType::Any, |p| p.ty);
            let lit = self.selected.contains(&link.from.node) || self.selected.contains(&link.to.node);
            let base = if ty == PinType::Exec { 2.6 } else { 2.0 };
            wires.push(WireSeg {
                a: self.to_screen(pa),
                b: self.to_screen(pb),
                color: if lit { theme::text(cx) } else { theme::pin(ty).into() },
                width: (base + if lit { 0.8 } else { 0.0 }) * self.zoom.max(0.6),
            });
        }
        if let Drag::Wire { anchor, side, cursor, .. } = &self.drag
            && let Some(l) = find(anchor.node)
            && let Some(p) = l.pin_center(&anchor.pin, *side)
        {
            let ty = self.graph.pin(anchor, *side).map_or(PinType::Any, |p| p.ty);
            let (a, b) = match side {
                Side::Output => (p, *cursor),
                Side::Input => (*cursor, p),
            };
            wires.push(WireSeg {
                a: self.to_screen(a),
                b: self.to_screen(b),
                color: theme::pin(ty).into(),
                width: 2.0 * self.zoom.max(0.6),
            });
        }
        let selection = match &self.drag {
            Drag::Select { start, cursor, .. } => {
                Some(Rect::from_points(self.to_screen(*start), self.to_screen(*cursor)))
            }
            _ => None,
        };
        Scene {
            pan: self.pan,
            zoom: self.zoom,
            wires,
            selection,
            grid_minor: theme::grid_minor(cx),
            grid_major: theme::grid_major(cx),
            selection_color: theme::selection(cx),
        }
    }
}
