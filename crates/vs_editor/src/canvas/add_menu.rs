//! The add-node menu: a filter, then the matching nodes, scrolled to keep the highlight visible.
//! Opening it, which nodes it lists (only those that can take the wire that opened it, if
//! any), its keys, and placing the chosen node.


use gpui_kit::{
    Context, IntoElement, MouseButton, MouseDownEvent, ScrollEvent, div, prelude::*, px,
};

use crate::editor::{EditTarget, Editor};
use crate::theme;
use super::MENU_ROWS;
use super::layout::{self, V2};

use vs_graph::{PinRef, Side, library};

impl Editor {
    pub(super) fn menu_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.canvas.menu.as_ref().expect("checked by the caller");
        let entries = self.menu_entries();
        let start = menu.highlight.saturating_sub(MENU_ROWS - 1);
        let rows = entries.iter().enumerate().skip(start).take(MENU_ROWS).map(|(i, entry)| {
            let entry = entry.clone();
            let highlighted = i == menu.highlight;
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .flex()
                .justify_between()
                .when(highlighted, |d| d.bg(theme::highlight(cx)))
                .hover(|s| s.bg(theme::button_hover(cx)))
                .cursor_pointer()
                .child(entry.label.clone())
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(if highlighted { theme::text(cx) } else { theme::dim(cx) })
                        .child(entry.category),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.place(&entry);
                        cx.notify();
                    }),
                )
        });
        let title = if menu.wire.is_some() { "Add connected node" } else { "Add node" };

        div()
            .absolute()
            .left(px(menu.at.x))
            .top(px(menu.at.y))
            .w(px(260.0))
            .p_1p5()
            .flex()
            .flex_col()
            .gap_0p5()
            .rounded_lg()
            .bg(theme::panel_bg(cx))
            .border_1()
            .border_color(theme::border(cx))
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_scroll(cx.listener(|this, e: &ScrollEvent, _, cx| {
                cx.stop_propagation();
                let count = this.menu_entries().len();
                if let Some(menu) = &mut this.canvas.menu {
                    let dy = e.delta.pixel_delta(px(20.0)).y.as_f32();
                    menu.highlight = if dy < 0.0 {
                        (menu.highlight + 1).min(count.saturating_sub(1))
                    } else {
                        menu.highlight.saturating_sub(1)
                    };
                }
                cx.notify();
            }))
            .child(div().px_2().pt_1().text_size(px(11.0)).text_color(theme::dim(cx)).child(title))
            .child(
                div()
                    .mx_1()
                    .my_1()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(theme::field_bg(cx))
                    .border_1()
                    .border_color(theme::selection(cx))
                    .child(if menu.filter.is_empty() {
                        div().text_color(theme::dim(cx)).child("Type to search…")
                    } else {
                        div().child(format!("{}|", menu.filter))
                    }),
            )
            .children(rows)
            .when(entries.is_empty(), |d| {
                d.child(div().px_2().py_1().text_color(theme::dim(cx)).child("No matching nodes"))
            })
    }
}

/// One row in the add-node menu.
#[derive(Clone, Debug)]
pub struct MenuEntry {
    pub kind: &'static str,
    pub var: Option<String>,
    /// `Component.path`, for Get / Set field nodes; the component's name, for a copy.
    pub field: Option<String>,
    pub label: String,
    pub category: &'static str,
}

pub struct Menu {
    /// Canvas-local position of the menu's corner.
    pub at: V2,
    /// Where the new node goes, in graph units.
    pub graph_at: V2,
    pub filter: String,
    pub highlight: usize,
    /// The wire dropped on empty canvas that opened the menu, if any.
    pub wire: Option<(PinRef, Side)>,
}

impl Editor {
    pub fn open_menu(&mut self, local: V2, wire: Option<(PinRef, Side)>) {
        // A preview run may have taught the project new components since.
        if let Some(project) = &self.workspace.project {
            let _ = vs_graph::components::load_project(&project.root);
        }
        let (_, size) = self.canvas.bounds.get();
        let at = V2::new(local.x.min(size.x - 270.0).max(4.0), local.y.min(size.y - 420.0).max(4.0));
        self.canvas.menu =
            Some(Menu { at, graph_at: self.doc.to_graph(local), filter: String::new(), highlight: 0, wire });
    }

    /// The menu rows that match the filter and, when a wire opened the menu, can take it.
    pub fn menu_entries(&self) -> Vec<MenuEntry> {
        let Some(menu) = &self.canvas.menu else { return Vec::new() };
        let filter = menu.filter.to_lowercase();
        let mut entries = Vec::new();
        for def in library::all() {
            if def.is_definition() {
                entries.push(MenuEntry {
                    kind: def.kind,
                    var: None,
                    field: None,
                    label: format!("New {}", def.title),
                    category: def.category,
                });
            } else if def.is_copy() {
                // Components other scripts declare. The open script's own come from its cards.
                let file = self.script_name();
                for c in vs_graph::components::all() {
                    let Some(source) = &c.source else { continue };
                    if *source == file || self.doc.graph.defined_components().any(|d| d.name == c.name) {
                        continue;
                    }
                    entries.push(MenuEntry {
                        kind: def.kind,
                        var: None,
                        label: format!("Copy of {} ({source})", c.name),
                        field: Some(c.name),
                        category: def.category,
                    });
                }
            } else if def.is_variable() {
                for var in &self.doc.graph.variables {
                    entries.push(MenuEntry {
                        kind: def.kind,
                        var: Some(var.name.clone()),
                        field: None,
                        label: format!("{} {}", def.title, var.name),
                        category: def.category,
                    });
                }
            } else if def.is_field() {
                // The script's own cards come first: they may be unsaved, or renamed since the
                // catalog last read this file.
                let own: Vec<_> = self.doc.graph.defined_components().map(vs_graph::components::declared_info).collect();
                let catalog = vs_graph::components::all().into_iter().filter(|c| !own.iter().any(|o| o.name == c.name));
                for component in own.iter().cloned().chain(catalog) {
                    for f in &component.fields {
                        let field = format!("{}.{}", component.name, f.path);
                        entries.push(MenuEntry {
                            kind: def.kind,
                            var: None,
                            label: format!("{} {field}", def.title),
                            field: Some(field),
                            category: def.category,
                        });
                    }
                }
            } else {
                entries.push(MenuEntry {
                    kind: def.kind,
                    var: None,
                    field: None,
                    label: def.title.to_string(),
                    category: def.category,
                });
            }
        }
        entries.retain(|e| {
            (filter.is_empty()
                || e.label.to_lowercase().contains(&filter)
                || e.category.to_lowercase().contains(&filter))
                && menu.wire.as_ref().is_none_or(|(pin, side)| self.fits(e, pin, *side).is_some())
        });
        entries
    }

    /// The pin on a would-be node of `entry` that a wire from `pin` would plug into.
    fn fits(&self, entry: &MenuEntry, pin: &PinRef, side: Side) -> Option<String> {
        let ty = self.doc.graph.pin(pin, side)?.ty;
        let probe = vs_graph::Node {
            id: 0,
            kind: entry.kind.to_string(),
            x: 0.0,
            y: 0.0,
            values: Default::default(),
            var: entry.var.clone(),
            field: entry.field.clone(),
            name: None,
            component: None,
            copy_of: None,
        };
        let pins = self.doc.graph.pins_of(&probe);
        let candidates = match side {
            Side::Output => pins.inputs,
            Side::Input => pins.outputs,
        };
        candidates
            .into_iter()
            .find(|p| match side {
                Side::Output => ty.connects_to(p.ty),
                Side::Input => p.ty.connects_to(ty),
            })
            .map(|p| p.name)
    }

    pub fn place(&mut self, entry: &MenuEntry) {
        let Some(menu) = self.canvas.menu.take() else { return };
        if library::find(entry.kind).is_some_and(|d| d.is_definition()) {
            // No pins, so no wire opened the menu for it (see `fits`).
            let id = self.doc.add_component(menu.graph_at);
            self.name_new(EditTarget::ComponentName(id));
            return;
        }
        if let (Some(true), Some(name)) = (library::find(entry.kind).map(|d| d.is_copy()), &entry.field) {
            self.doc.add_component_copy(name, menu.graph_at);
            return;
        }
        self.doc.checkpoint();
        let mut at = menu.graph_at;
        if let Some((_, Side::Input)) = &menu.wire {
            // Dragged out of an input: the new node sits to the left so its output faces it.
            at.x -= layout::NODE_W;
        }
        let id = match &entry.field {
            Some(field) => self.doc.graph.add_field_node(entry.kind, field, at.x, at.y),
            None => self.doc.graph.add_node(entry.kind, entry.var.clone(), at.x, at.y),
        };
        if let Some((pin, side)) = menu.wire
            && let Some(target) = self.fits(entry, &pin, side)
        {
            let new_pin = PinRef::new(id, target);
            match side {
                Side::Output => self.doc.graph.link(pin, new_pin),
                Side::Input => self.doc.graph.link(new_pin, pin),
            };
        }
        self.doc.selected.clear();
        self.doc.selected.insert(id);
    }

    /// A key while the menu is open: Escape closes it, the arrows move the highlight, Enter adds
    /// the highlighted node, and typing filters.
    pub fn menu_key(&mut self, key: &str, typed: Option<String>) {
        let count = self.menu_entries().len();
        let menu = self.canvas.menu.as_mut().unwrap();
        match key {
            "escape" => self.canvas.menu = None,
            "up" => menu.highlight = menu.highlight.saturating_sub(1),
            "down" => menu.highlight = (menu.highlight + 1).min(count.saturating_sub(1)),
            "enter" => {
                let entries = self.menu_entries();
                let pick = self.canvas.menu.as_ref().map_or(0, |m| m.highlight);
                if let Some(entry) = entries.get(pick) {
                    self.place(&entry.clone());
                }
            }
            "backspace" => {
                menu.filter.pop();
                menu.highlight = 0;
            }
            _ => {
                if let Some(t) = typed {
                    menu.filter.push_str(&t);
                    menu.highlight = 0;
                }
            }
        }
    }
}
