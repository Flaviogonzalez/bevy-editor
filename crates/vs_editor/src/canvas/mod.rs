//! The node canvas: the grid, the wires, the nodes and the add-node menu, plus the mouse and
//! wheel handlers that drive them.
//!
//! - `layout`: node geometry in graph units and hit testing, shared by drawing and input.
//! - `input`: the mouse handlers (press, move, release) and the drags they drive.
//! - `node`: one node drawn as a card, with its pins and inline fields.
//! - `add_menu`: the searchable menu that adds a node.
//! - `paint`: what is painted directly rather than as elements (grid, wires, selection box).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

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

use crate::editor::Editor;
pub use add_menu::Menu;
use layout::{Hit, NodeLayout, Rect, V2};
use vs_graph::{Graph, PinRef};
use crate::theme;
use paint::paint_scene;

mod add_menu;
mod input;
pub mod layout;
mod node;
mod paint;

pub const MENU_ROWS: usize = 10;

/// What the left or right mouse button is doing right now.
///
/// Every drag ends in [`Editor::end_drag`], whether the button comes up over the canvas or
/// somewhere we didn't see, so its history entry is recorded either way.
pub enum Drag {
    None,
    Pan { last: V2, moved: bool, opens_menu: bool },
    Nodes { last: V2, before: Graph, moved: bool },
    /// A wire being pulled from `anchor`. `side` is the side of the anchor pin. `before` is the
    /// graph before the drag when it picked up an existing wire (and so already changed the graph).
    Wire { anchor: PinRef, side: Side, cursor: V2, before: Option<Graph> },
    Select { start: V2, cursor: V2, additive: bool },
}

/// The pointer's interaction with the canvas. None of it is saved or undone: it describes what
/// the mouse is doing to the document, not the document.
pub struct CanvasState {
    pub drag: Drag,
    pub menu: Option<Menu>,
    /// What lies under the pointer, so the canvas can highlight the node or pin it would act on.
    pub hover: Hit,
    /// Last pointer position, canvas-local.
    pub mouse: V2,
    /// The canvas origin in window coordinates and its size, written while painting.
    pub bounds: Rc<Cell<(V2, V2)>>,
    /// A script a click asked to open (the link on a component copy). The mouse handler opens it,
    /// since opening needs the window.
    pub open_request: Option<std::path::PathBuf>,
}

impl CanvasState {
    /// A window position in canvas-local coordinates.
    pub fn to_local(&self, window_pos: gpui_kit::Point<gpui_kit::Pixels>) -> V2 {
        let origin = self.bounds.get().0;
        V2::new(window_pos.x.as_f32() - origin.x, window_pos.y.as_f32() - origin.y)
    }

    /// The canvas size, as last painted.
    pub fn size(&self) -> V2 {
        self.bounds.get().1
    }
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            drag: Drag::None,
            menu: None,
            hover: Hit::Empty,
            mouse: V2::default(),
            bounds: Rc::new(Cell::new((V2::default(), V2::new(800.0, 600.0)))),
            open_request: None,
        }
    }
}

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
    /// The system each event node compiles to (see [`vs_graph::access`]), by node.
    pub(crate) fn systems(&self, compiled: &Compiled) -> BTreeMap<NodeId, vs_graph::access::System> {
        let names = vs_graph::rust::names(&self.doc.file_name());
        vs_graph::access::systems(&compiled.module, &names)
            .into_iter()
            .map(|s| (compiled.module.blocks[s.block].node, s))
            .collect()
    }

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
        let cell = self.canvas.bounds.clone();
        let painter = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let origin = V2::new(bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
                let extent = V2::new(bounds.size.width.as_f32(), bounds.size.height.as_f32());
                // The nodes of this frame were picked for the size the canvas had before (see
                // `visible`): when it changed (the first frame, a resize), draw again.
                if cell.replace((origin, extent)).1 != extent {
                    window.refresh();
                }
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
                    this.left_down(this.canvas.to_local(e.position), additive);
                    if let Some(path) = this.canvas.open_request.take() {
                        this.open_path(path, window);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.right_down(this.canvas.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.middle_down(this.canvas.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if this.mouse_move(this.canvas.to_local(e.position), e.pressed_button.is_some()) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.canvas.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.canvas.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.mouse_up(this.canvas.to_local(e.position));
                    cx.notify();
                }),
            )
            .on_scroll(cx.listener(|this, e: &ScrollEvent, _, cx| {
                let dy = e.delta.pixel_delta(px(40.0)).y.as_f32();
                this.doc.zoom_at(this.canvas.to_local(e.position), dy);
                cx.notify();
            }))
            .child(painter);

        let systems = self.systems(compiled);
        // Only the nodes in view become elements. Each node is a few dozen elements that GPUI
        // lays out and paints every frame, so with hundreds of nodes the ones off screen were
        // most of the cost of panning. Wires are painted apart, so they still show.
        let view = self.visible();
        for l in layouts.iter().filter(|l| l.rect.intersects(&view)) {
            if let Some(node) = self.doc.graph.node(l.id) {
                let system = systems.get(&l.id);
                area = area.child(self.node_element(node, l, flagged.get(&l.id).copied(), system, cx));
            }
        }
        if self.doc.graph.nodes.is_empty() && self.canvas.menu.is_none() {
            area = area.child(self.empty_state(cx));
        }
        if self.canvas.menu.is_some() {
            area = area.child(self.menu_element(cx));
        }
        area
    }

    /// The part of the graph the canvas shows, in graph units, as of the last paint.
    fn visible(&self) -> Rect {
        let size = self.canvas.size();
        Rect::from_points(self.doc.to_graph(V2::default()), self.doc.to_graph(size))
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
                            let (_, size) = this.canvas.bounds.get();
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

    /// The first card or copy on the canvas that shows `component`, and the point on its right
    /// edge level with the row of field `path`, in graph units.
    fn field_row(&self, layouts: &[NodeLayout], component: &str, path: &str) -> Option<(NodeId, V2)> {
        let (node, def) = self.doc.graph.nodes.iter().find_map(|n| match (&n.component, &n.copy_of) {
            (Some(def), _) if def.name == component => Some((n, def.clone())),
            (None, Some(name)) if name == component => Some((n, layout::copy_view(&self.doc.graph, name).0)),
            _ => None,
        })?;
        let i = def.fields.iter().position(|f| f.name == path)?;
        let l = layouts.iter().find(|l| l.id == node.id)?;
        let (_, rect) = l.parts.iter().find(|(part, _)| *part == layout::CardPart::FieldName(i))?;
        Some((node.id, V2::new(l.rect.max.x, rect.min.y + rect.h() * 0.5)))
    }

    fn scene(&self, layouts: &[NodeLayout], cx: &App) -> Scene {
        let find = |id: NodeId| layouts.iter().find(|l| l.id == id);
        let mut wires = Vec::new();
        // A faint line from each Get / Set to its field's row on a card (or a copy) on this
        // canvas, so it's plain which component a node reads or writes. Painted first, so the
        // wires go over them.
        for node in &self.doc.graph.nodes {
            let Some((component, path)) = node.field.as_deref().and_then(vs_graph::components::split) else {
                continue;
            };
            let Some((card, row)) = self.field_row(layouts, component, path) else { continue };
            let lit = self.doc.selected.contains(&node.id) || self.doc.selected.contains(&card);
            wires.push(WireSeg {
                a: self.doc.to_screen(row),
                b: self.doc.to_screen(V2::new(node.x, node.y + layout::HEADER_H * 0.5)),
                color: theme::dim(cx).opacity(if lit { 0.8 } else { 0.3 }),
                width: 1.2 * self.doc.zoom.max(0.6),
            });
        }
        for link in &self.doc.graph.links {
            let (Some(a), Some(b)) = (find(link.from.node), find(link.to.node)) else { continue };
            let (Some(pa), Some(pb)) = (
                a.pin_center(&link.from.pin, Side::Output),
                b.pin_center(&link.to.pin, Side::Input),
            ) else {
                continue;
            };
            let ty = self.doc.graph.pin(&link.from, Side::Output).map_or(PinType::Any, |p| p.ty);
            let lit = self.doc.selected.contains(&link.from.node) || self.doc.selected.contains(&link.to.node);
            let base = if ty == PinType::Exec { 2.6 } else { 2.0 };
            wires.push(WireSeg {
                a: self.doc.to_screen(pa),
                b: self.doc.to_screen(pb),
                color: if lit { theme::text(cx) } else { theme::pin(ty).into() },
                width: (base + if lit { 0.8 } else { 0.0 }) * self.doc.zoom.max(0.6),
            });
        }
        if let Drag::Wire { anchor, side, cursor, .. } = &self.canvas.drag
            && let Some(l) = find(anchor.node)
            && let Some(p) = l.pin_center(&anchor.pin, *side)
        {
            let ty = self.doc.graph.pin(anchor, *side).map_or(PinType::Any, |p| p.ty);
            let (a, b) = match side {
                Side::Output => (p, *cursor),
                Side::Input => (*cursor, p),
            };
            wires.push(WireSeg {
                a: self.doc.to_screen(a),
                b: self.doc.to_screen(b),
                color: theme::pin(ty).into(),
                width: 2.0 * self.doc.zoom.max(0.6),
            });
        }
        let selection = match &self.canvas.drag {
            Drag::Select { start, cursor, .. } => {
                Some(Rect::from_points(self.doc.to_screen(*start), self.doc.to_screen(*cursor)))
            }
            _ => None,
        };
        Scene {
            pan: self.doc.pan,
            zoom: self.doc.zoom,
            wires,
            selection,
            grid_minor: theme::grid_minor(cx),
            grid_major: theme::grid_major(cx),
            selection_color: theme::selection(cx),
        }
    }
}
