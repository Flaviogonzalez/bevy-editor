//! The systems of a script as groups on the canvas: which nodes make up each system, and the
//! frame drawn around them, with the system's name and what it asks Bevy for in its header.
//!
//! Input: the script's blocks (each lists the nodes it was lowered from, see
//! [`vs_graph::ir::Block::nodes`]), its systems in the order the plugin adds them, and the node
//! layouts. Output: one [`Group`] per system, in that order. Nothing here is stored in the
//! script: a group is what the compiler made of the graph, so it always shows what compiles.
//!
//! A node belongs to a group when only that system's block was lowered from it. A data node
//! that feeds two systems belongs to none: each evaluates it on its own, so it's neither's.
//! Membership comes from the wires, not from where a node sits, so a frame may pass over a node
//! that isn't in its group; the stripe each member carries (see `node`) tells them apart.

use std::collections::BTreeMap;

use gpui_kit::{Context, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, div, prelude::*, px};
use vs_graph::{NodeId, library};
use vs_graph::access::System;
use vs_graph::ir::{Block, Event};

use crate::canvas::layout::{NodeLayout, Rect};
use crate::editor::{EditTarget, Editor};
use crate::theme;
use crate::ui::MONO;

/// Space between a group's frame and its outermost nodes, in graph units.
const PAD: f32 = 16.0;
/// The header of a group's frame, above its nodes: two lines of text.
const HEADER_H: f32 = 40.0;

/// One system and the nodes that make it up.
pub struct Group {
    pub system: System,
    /// The event node it compiles from. Its name, when it has one, is the system's.
    pub event: NodeId,
    /// Whether the system's name is the event's name. Otherwise it's made from the kind of
    /// event and the script's name (`update_light`), see [`vs_graph::access::systems`].
    pub named: bool,
    /// What makes it run, in Bevy's terms: `every frame in Update`, `when Spinner is added`,
    /// `run_if: Space just pressed`. Its event node shows it.
    pub trigger: String,
    /// Its place among the script's systems, from 1. The plugin chains them in this order.
    pub order: usize,
    /// The nodes only this system was lowered from, its event among them.
    pub members: Vec<NodeId>,
    /// The frame around its members, header included, in graph units.
    pub rect: Rect,
}

impl Group {
    /// The color that marks this group's frame and its members. Systems get them in order, so
    /// a color says "the second system", not anything about what the system does.
    pub fn color(&self) -> Hsla {
        theme::system(self.order - 1).into()
    }
}

/// The block that owns each node: the only one lowered from it. A node no block or more than
/// one block was lowered from is missing. An event is always its own block's, even when another
/// event reads one of its outputs.
fn owners(blocks: &[Block]) -> BTreeMap<NodeId, usize> {
    let mut seen: BTreeMap<NodeId, (usize, usize)> = BTreeMap::new();
    for (index, block) in blocks.iter().enumerate() {
        for node in &block.nodes {
            seen.entry(*node).or_insert((index, 0)).1 += 1;
        }
    }
    let mut owners: BTreeMap<NodeId, usize> =
        seen.into_iter().filter(|(_, (_, count))| *count == 1).map(|(node, (index, _))| (node, index)).collect();
    for (index, block) in blocks.iter().enumerate() {
        owners.insert(block.node, index);
    }
    owners
}

/// What makes the system of an event run, as the generated plugin and query say it: every
/// system is in `Update`, a Start event's query takes only the entities that just got `own`
/// (`Added<own>`), and a key event's system has a run condition.
fn trigger(event: &Event, own: &str) -> String {
    match event {
        Event::Update => "every frame in Update".into(),
        Event::Start => format!("when {own} is added"),
        Event::Key(key) => format!("run_if: {key} just pressed"),
    }
}

/// A script's groups, one per system, in the order of `systems`.
pub fn groups(blocks: &[Block], systems: Vec<System>, layouts: &[NodeLayout]) -> Vec<Group> {
    let owners = owners(blocks);
    systems
        .into_iter()
        .enumerate()
        .map(|(i, system)| {
            let members: Vec<NodeId> =
                owners.iter().filter(|(_, block)| **block == system.block).map(|(node, _)| *node).collect();
            let around = layouts
                .iter()
                .filter(|l| members.contains(&l.id))
                .map(|l| l.rect)
                .reduce(|a, b| a.union(&b))
                .unwrap_or_default();
            let rect = Rect::new(
                around.min.x - PAD,
                around.min.y - PAD - HEADER_H,
                around.w() + 2.0 * PAD,
                around.h() + 2.0 * PAD + HEADER_H,
            );
            let block = &blocks[system.block];
            let trigger = trigger(&block.event, &system.own);
            Group { event: block.node, named: block.name.is_some(), trigger, system, order: i + 1, members, rect }
        })
        .collect()
}

impl Editor {
    /// A group's frame: a faint tint of its color over its nodes' area, and a header with the
    /// system's order and name, where that name comes from, the schedule it runs in, and its
    /// parameters. Long parameters are cut short; the Inspector shows them whole when the event
    /// is selected.
    ///
    /// The name takes clicks: pressing it selects the event and starts editing the event's name,
    /// which is the system's. The rest of the frame takes none, so pressing on it is pressing on
    /// the canvas.
    pub(super) fn group_element(&self, group: &Group, cx: &mut Context<Self>) -> Div {
        let z = self.doc.zoom;
        let color = group.color();
        let at = self.doc.to_screen(group.rect.min);
        let radius = 10.0 * z;
        let event = group.event;
        let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::NodeName(event));
        let name = match editing {
            Some(e) => format!("{}|", e.buffer),
            None => group.system.name.clone(),
        };
        let event_title = self
            .doc
            .graph
            .node(event)
            .and_then(|n| library::find(&n.kind))
            .map_or("the event", |d| d.title);
        // Where the name comes from, in the words the canvas uses: the event node and the script.
        let source = if group.named {
            format!("name of {event_title}")
        } else {
            let script = vs_graph::rust::names(&self.doc.file_name()).module;
            format!("automatic: {event_title} + {script}")
        };
        let name_box = div()
            .flex_shrink(1.0)
            .min_w_0()
            .px(px(4.0 * z))
            .rounded(px(4.0 * z))
            .border_1()
            .border_color(if editing.is_some() { theme::selection(cx) } else { color.opacity(0.0) })
            .when(editing.is_some(), |d| d.bg(theme::field_bg(cx)))
            .hover(|s| s.border_color(theme::dim(cx).opacity(0.7)))
            .cursor_text()
            .overflow_hidden()
            .text_ellipsis()
            .font_family(MONO)
            .font_weight(FontWeight::SEMIBOLD)
            .child(name)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    // The canvas under it would start a selection box.
                    cx.stop_propagation();
                    window.focus(&this.focus, cx);
                    this.doc.selected.clear();
                    this.doc.selected.insert(event);
                    this.start_edit(EditTarget::NodeName(event));
                    cx.notify();
                }),
            );
        let header = div()
            .h(px(HEADER_H * z))
            .px(px(12.0 * z))
            .rounded_t(px(radius))
            .bg(color.opacity(0.22))
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(2.0 * z))
            .whitespace_nowrap()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0 * z))
                    .text_size(px(12.5 * z))
                    .child(div().flex_none().text_color(color).font_weight(FontWeight::BOLD).child(group.order.to_string()))
                    .child(name_box)
                    // Every system of a script runs in `Update`: a Start event is an `Update`
                    // system whose query takes only the entities that just got the component.
                    // It sits by the name, not at the frame's far edge, where a wide frame hides it.
                    .child(div().flex_none().text_size(px(10.5 * z)).text_color(theme::dim(cx)).child("in Update"))
                    .child(
                        div()
                            .flex_none()
                            .px(px(5.0 * z))
                            .rounded(px(4.0 * z))
                            .bg(theme::dim(cx).opacity(0.15))
                            .text_size(px(10.0 * z))
                            .text_color(theme::dim(cx))
                            .child(source),
                    ),
            )
            .child(
                div()
                    .overflow_hidden()
                    .text_ellipsis()
                    .font_family(MONO)
                    .text_size(px(10.0 * z))
                    .text_color(theme::dim(cx))
                    .child(group.system.params().join(", ")),
            );
        div()
            .absolute()
            .left(px(at.x))
            .top(px(at.y))
            .w(px(group.rect.w() * z))
            .h(px(group.rect.h() * z))
            .rounded(px(radius))
            .bg(color.opacity(0.06))
            .border_1()
            .border_color(color.opacity(0.55))
            .text_color(theme::text(cx))
            .child(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::layout::layout;
    use vs_graph::{Graph, PinRef};

    #[test]
    fn a_node_two_systems_use_is_in_neither_group() {
        let mut g = Graph::default();
        let update = g.add_node("event.update", None, 0.0, 0.0);
        let tick = g.add_node("action.print", None, 300.0, 0.0);
        let start = g.add_node("event.start", None, 0.0, 400.0);
        let hello = g.add_node("action.print", None, 300.0, 400.0);
        let elapsed = g.add_node("time.elapsed", None, 0.0, 200.0);
        let loose = g.add_node("math.add", None, 900.0, 900.0);
        assert!(g.link(PinRef::new(update, "then"), PinRef::new(tick, "in")));
        assert!(g.link(PinRef::new(start, "then"), PinRef::new(hello, "in")));
        assert!(g.link(PinRef::new(elapsed, "seconds"), PinRef::new(tick, "value")));
        assert!(g.link(PinRef::new(elapsed, "seconds"), PinRef::new(hello, "value")));

        let module = vs_graph::lower(&g).module;
        let systems = vs_graph::access::systems(&module, &vs_graph::rust::names("scripts/door.lua"));
        let layouts: Vec<NodeLayout> = g.nodes.iter().map(|n| layout(&g, n)).collect();
        let groups = groups(&module.blocks, systems, &layouts);

        // Start systems come first, as the plugin adds them.
        assert_eq!(groups.iter().map(|g| g.system.name.as_str()).collect::<Vec<_>>(), ["setup_door", "update_door"]);
        assert_eq!(groups[0].trigger, "when Door is added");
        assert_eq!(groups[1].trigger, "every frame in Update");
        assert_eq!(groups[0].members, [start, hello]);
        assert_eq!(groups[1].members, [update, tick]);
        assert!(groups.iter().all(|g| !g.members.contains(&elapsed) && !g.members.contains(&loose)));

        // Each frame holds its members, with room for the header above them.
        let rect_of = |id: NodeId| layouts.iter().find(|l| l.id == id).unwrap().rect;
        for group in &groups {
            for member in &group.members {
                let r = rect_of(*member);
                assert!(group.rect.contains(r.min) && group.rect.contains(r.max));
                assert!(r.min.y >= group.rect.min.y + HEADER_H);
            }
        }
        // The frames follow the wires, not the positions: the shared node sits between them.
        assert!(!groups[0].rect.intersects(&groups[1].rect));
    }
}
