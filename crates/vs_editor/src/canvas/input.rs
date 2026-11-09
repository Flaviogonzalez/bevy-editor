//! The mouse on the canvas: pressing a button picks what to do from what's under the pointer
//! (a pin starts a wire, a node starts a move, empty canvas starts a selection box or a pan),
//! moving updates the drag, and releasing ends it and records one undo step for the gesture.
//!
//! These are `Editor` methods rather than `CanvasState` ones because a click can also start a
//! text edit or report on the status line.

use vs_graph::{NodeId, PinRef, Side, Value};

use super::Drag;
use super::layout::{self, CardPart, Hit, Rect, V2};
use crate::editor::{EditTarget, Editing, Editor};

impl Editor {
    pub fn left_down(&mut self, local: V2, additive: bool) {
        self.commit_edit();
        self.canvas.menu = None;
        let g = self.doc.to_graph(local);
        let layouts = self.doc.layouts();
        match layout::hit_test(&layouts, g) {
            Hit::Pin(pin, side) => {
                // Grabbing a connected input picks its wire back up from the source.
                let existing = match side {
                    Side::Input => self.doc.graph.links_to(&pin).next().cloned(),
                    Side::Output => None,
                };
                if let Some(link) = existing {
                    // One undo step for the whole gesture: recorded when the drag ends.
                    let before = self.doc.graph.clone();
                    self.doc.graph.links.retain(|l| l != &link);
                    self.canvas.drag =
                        Drag::Wire { anchor: link.from, side: Side::Output, cursor: g, before: Some(before) };
                } else {
                    self.canvas.drag = Drag::Wire { anchor: pin, side, cursor: g, before: None };
                }
            }
            Hit::Field(node, pin) => {
                let Some(value) = self.doc.graph.node(node).and_then(|n| n.values.get(&pin)).cloned()
                else {
                    return;
                };
                if let Value::Bool(b) = value {
                    self.doc.checkpoint();
                    if let Some(n) = self.doc.graph.node_mut(node) {
                        n.values.insert(pin, Value::Bool(!b));
                    }
                } else {
                    self.editing = Some(Editing {
                        target: EditTarget::Field(node, pin),
                        buffer: value.display(),
                    });
                }
            }
            Hit::Card(id, part) => {
                self.doc.select_only(id);
                self.click_card(id, part);
            }
            Hit::Header(id) | Hit::Body(id) => {
                if additive {
                    if !self.doc.selected.remove(&id) {
                        self.doc.selected.insert(id);
                    }
                } else if !self.doc.selected.contains(&id) {
                    self.doc.selected.clear();
                    self.doc.selected.insert(id);
                }
                self.doc.bring_to_front(id);
                self.canvas.drag = Drag::Nodes { last: g, before: self.doc.graph.clone(), moved: false };
            }
            Hit::Empty => {
                if !additive {
                    self.doc.selected.clear();
                }
                self.canvas.drag = Drag::Select { start: g, cursor: g, additive };
            }
        }
    }

    /// A click on part of a component card: the name, a field's name and value start a text
    /// edit (a bool value flips instead), the type label switches to the next type, and the
    /// buttons remove a field or add one, whose name is then edited right away.
    fn click_card(&mut self, id: NodeId, part: CardPart) {
        let value = |doc: &crate::document::Document, i: usize| {
            doc.graph.node(id).and_then(|n| n.component.as_ref()).and_then(|d| d.fields.get(i)).map(|f| f.value.clone())
        };
        match part {
            CardPart::Name => self.start_edit(EditTarget::ComponentName(id)),
            CardPart::FieldName(i) => self.start_edit(EditTarget::ComponentField(id, i)),
            CardPart::Value(i) => match value(&self.doc, i) {
                Some(Value::Bool(_)) => self.doc.toggle_component_value(id, i),
                Some(_) => self.start_edit(EditTarget::ComponentValue(id, i)),
                None => {}
            },
            CardPart::Type(i) => self.doc.cycle_component_field_type(id, i),
            CardPart::Remove(i) => {
                self.doc.remove_component_field(id, i);
            }
            CardPart::AddField => {
                self.doc.add_component_field(id);
                let last = self.doc.graph.node(id).and_then(|n| n.component.as_ref()).map_or(0, |d| d.fields.len());
                self.name_new(EditTarget::ComponentField(id, last.saturating_sub(1)));
            }
        }
    }

    pub fn right_down(&mut self, local: V2) {
        self.commit_edit();
        self.canvas.menu = None;
        let g = self.doc.to_graph(local);
        let layouts = self.doc.layouts();
        let on_node = match layout::hit_test(&layouts, g) {
            Hit::Header(id) | Hit::Body(id) | Hit::Field(id, _) | Hit::Card(id, _) => Some(id),
            Hit::Pin(..) | Hit::Empty => None,
        };
        if let Some(id) = on_node
            && !self.doc.selected.contains(&id)
        {
            self.doc.selected.clear();
            self.doc.selected.insert(id);
        }
        self.canvas.drag = Drag::Pan { last: local, moved: false, opens_menu: on_node.is_none() };
    }

    pub fn middle_down(&mut self, local: V2) {
        self.canvas.drag = Drag::Pan { last: local, moved: false, opens_menu: false };
    }

    pub fn mouse_move(&mut self, local: V2, any_button_held: bool) -> bool {
        self.canvas.mouse = local;
        let hover = layout::hit_test(&self.doc.layouts(), self.doc.to_graph(local));
        let hover_changed = hover != self.canvas.hover;
        self.canvas.hover = hover;
        if !any_button_held && !matches!(self.canvas.drag, Drag::None) {
            // The button went up somewhere we didn't see, such as outside the window: keep what
            // the drag already changed, but don't act on a drop we didn't see.
            self.end_drag(None);
            return true;
        }
        let g = self.doc.to_graph(local);
        match &mut self.canvas.drag {
            Drag::None => return hover_changed,
            Drag::Pan { last, moved, .. } => {
                let delta = local - *last;
                if delta.x.abs() + delta.y.abs() > 0.0 {
                    *moved = true;
                }
                *last = local;
                self.doc.pan = self.doc.pan + delta;
            }
            Drag::Nodes { last, moved, .. } => {
                let delta = g - *last;
                *last = g;
                *moved = true;
                let selected = self.doc.selected.clone();
                for node in self.doc.graph.nodes.iter_mut().filter(|n| selected.contains(&n.id)) {
                    node.x += delta.x;
                    node.y += delta.y;
                }
            }
            Drag::Wire { cursor, .. } | Drag::Select { cursor, .. } => *cursor = g,
        }
        true
    }

    pub fn mouse_up(&mut self, local: V2) {
        self.end_drag(Some(local));
    }

    /// Ends the current drag. `drop` is where the button came up, canvas-local, or `None` when it
    /// came up somewhere we didn't see: then whatever the drag already changed is kept and
    /// recorded, but nothing that depends on the drop point happens (linking, the menu, the
    /// selection box).
    pub fn end_drag(&mut self, drop: Option<V2>) {
        match std::mem::replace(&mut self.canvas.drag, Drag::None) {
            Drag::None => {}
            Drag::Pan { moved, opens_menu, .. } => {
                if let Some(local) = drop
                    && !moved
                    && opens_menu
                {
                    self.open_menu(local, None);
                }
            }
            Drag::Nodes { before, moved, .. } => {
                if moved {
                    self.doc.history.record(before);
                }
            }
            Drag::Wire { anchor, side, before, .. } => {
                // A picked-up wire already changed the graph; a new one hasn't yet.
                let picked_up = before.is_some();
                let before = before.unwrap_or_else(|| self.doc.graph.clone());
                let linked = match drop {
                    Some(local) => self.drop_wire(local, anchor, side),
                    None => false,
                };
                if linked || picked_up {
                    self.doc.history.record(before);
                }
            }
            Drag::Select { .. } if drop.is_none() => {}
            Drag::Select { start, cursor, additive } => {
                let area = Rect::from_points(start, cursor);
                if area.w() > 2.0 || area.h() > 2.0 {
                    if !additive {
                        self.doc.selected.clear();
                    }
                    for l in self.doc.layouts() {
                        if l.rect.intersects(&area) {
                            self.doc.selected.insert(l.id);
                        }
                    }
                }
            }
        }
    }

    /// Drops a wire pulled from `anchor` at `local`: links it to the pin there, or opens the menu
    /// over empty canvas. Returns whether it linked. The caller records the history entry.
    fn drop_wire(&mut self, local: V2, anchor: PinRef, side: Side) -> bool {
        let layouts = self.doc.layouts();
        match layout::hit_test(&layouts, self.doc.to_graph(local)) {
            Hit::Pin(target, target_side) if target_side != side => {
                let (from, to) = match side {
                    Side::Output => (anchor, target),
                    Side::Input => (target, anchor),
                };
                if self.doc.graph.can_link(&from, &to) {
                    self.doc.graph.link(from, to);
                    return true;
                }
                self.chrome.status = "Those pins don't fit together".into();
            }
            Hit::Empty => self.open_menu(local, Some((anchor, side))),
            _ => {}
        }
        false
    }
}
