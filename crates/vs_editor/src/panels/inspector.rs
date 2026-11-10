//! The right panel, what you edit: the selected node, what it does, its name and its pins.


use gpui_kit::{Context, Div, FontWeight, IntoElement, div, prelude::*, px};
use vs_graph::{Compiled, library};

use crate::editor::{EditTarget, Editor};
use crate::theme;
use crate::ui::{MONO, field, group, panel, panel_title};

impl Editor {
    /// The right panel: what you edit. The selected node, what it does, its name and its pins.
    pub(crate) fn inspector(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        panel("inspector", cx)
            .border_l_1()
            .child(panel_title("Inspector", cx))
            .child(self.node_details(compiled, cx))
    }

    fn node_details(&self, compiled: &Compiled, cx: &mut Context<Self>) -> Div {
        let hint = |text: String| div().px_3().py_3().text_color(theme::dim(cx)).child(text);
        let id = match self.doc.selected.iter().next() {
            Some(id) if self.doc.selected.len() == 1 => *id,
            Some(_) => return hint(format!("{} nodes selected", self.doc.selected.len())),
            None => return hint("Select a node to see what it does.".into()),
        };
        let Some((node, def)) = self.doc.graph.node(id).and_then(|n| library::find(&n.kind).map(|d| (n, d))) else {
            return hint("Unknown node".into());
        };
        let pins = self.doc.graph.pins_of(node);
        let system = self.systems(compiled).remove(&id);
        let query = system.as_ref().map(|s| self.query_trigger(id, s, cx));
        let pin_row = |p: &vs_graph::library::PinDef| {
            div()
                .h(px(24.0))
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(theme::pin(p.ty)))
                .child(div().flex_1().child(p.name.clone()))
                .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(p.ty.label()))
        };
        let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::NodeName(id));
        let name = match (editing, &node.name, &system) {
            (Some(e), _, _) => format!("{}|", e.buffer),
            (None, Some(name), _) => name.clone(),
            // An unnamed event still names its system, after the event and the script.
            (None, None, Some(system)) => format!("(automatic: {})", system.name),
            (None, None, None) => "(unnamed)".into(),
        };
        let declared = node.component.as_ref().map(|c| &c.name).or(node.copy_of.as_ref());
        let title = match node.var.as_ref().or(node.field.as_ref()).or(declared) {
            Some(subject) => format!("{} {subject}", def.title),
            None => def.title.to_string(),
        };
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3()
                    .py_3()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .border_b_1()
                    .border_color(theme::border(cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(10.0)).h(px(10.0)).rounded_sm().bg(theme::category(def.category)))
                            .child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child(title))
                            .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(format!("#{}", node.id))),
                    )
                    .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(def.category))
                    .child(div().text_color(theme::dim(cx)).child(def.description)),
            )
            // A node's name names its value in exported Rust; a card is named on the canvas, and
            // a copy has the name of what it copies.
            .when(!def.is_definition() && !def.is_copy(), |d| {
                d.child(group("Name", None, cx).child(field("node-name".into(), name, editing.is_some(), cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.start_edit(EditTarget::NodeName(id));
                        cx.notify();
                    }),
                )))
            })
            // An event's system queries entities: the button that opens its commandbox.
            .children(query)
            // An event compiles to a system: its signature, as the exported Rust has it.
            .when_some(system, |d, system| {
                let param = |text: String| div().pl_4().child(format!("{text},"));
                d.child(
                    group("System", None, cx)
                        .font_family(MONO)
                        .text_size(px(11.5))
                        .child(div().child(format!("fn {}(", system.name)))
                        .children(system.params().into_iter().map(param))
                        .child(div().child(")")),
                )
            })
            .when(!pins.inputs.is_empty(), |d| {
                d.child(group("Inputs", None, cx).children(pins.inputs.iter().map(pin_row)))
            })
            .when(!pins.outputs.is_empty(), |d| {
                d.child(group("Outputs", None, cx).children(pins.outputs.iter().map(pin_row)))
            })
    }
}
