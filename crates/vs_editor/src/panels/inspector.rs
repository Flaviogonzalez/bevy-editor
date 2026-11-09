//! The right panel, what you edit: the selected node, what it does, its name and its pins.


use gpui_kit::{
    Context, Div, FontWeight, IntoElement, div, prelude::*, px,
};
use vs_graph::library;

use crate::editor::{EditTarget, Editor};
use crate::theme;
use crate::ui::{field, group, panel, panel_title};

impl Editor {
    /// The right panel: what you edit. The selected node, what it does, its name and its pins.
    pub(crate) fn inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel("inspector", cx)
            .border_l_1()
            .child(panel_title("Inspector", cx))
            .child(self.node_details(cx))
    }

    fn node_details(&self, cx: &mut Context<Self>) -> Div {
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
        let name = match (editing, &node.name) {
            (Some(e), _) => format!("{}|", e.buffer),
            (None, Some(name)) => name.clone(),
            (None, None) => "(unnamed)".into(),
        };
        let declared = node.component.as_ref().map(|c| &c.name);
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
            // A node's name names its value in exported Rust; a card is named on the canvas.
            .when(!def.is_definition(), |d| {
                d.child(group("Name", None, cx).child(field("node-name".into(), name, editing.is_some(), cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.start_edit(EditTarget::NodeName(id));
                        cx.notify();
                    }),
                )))
            })
            .when(!pins.inputs.is_empty(), |d| {
                d.child(group("Inputs", None, cx).children(pins.inputs.iter().map(pin_row)))
            })
            .when(!pins.outputs.is_empty(), |d| {
                d.child(group("Outputs", None, cx).children(pins.outputs.iter().map(pin_row)))
            })
    }
}
