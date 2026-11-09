//! The left panel, what you navigate and declare: the open project (its scripts and its catalog
//! of components), then the graph's variables.


use gpui_kit::{
    Anchor, App, Context, Div, IntoElement, Hsla, SharedString,
    Window, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Disableable, Sizable};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{PinType, Value};

use crate::actions;
use crate::editor::{EditTarget, Editor};
use crate::theme;
use crate::ui::{dispatch, field, group, panel, panel_title};
use crate::canvas::layout::{self, V2};

impl Editor {
    /// The left panel: what you navigate and declare. The open project (its scripts and its
    /// catalog of components), then the graph's variables.
    pub(crate) fn explorer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel("explorer", cx)
            .border_r_1()
            .child(panel_title("Explorer", cx))
            .child(self.project_group(cx))
            .child(self.variables_group(cx))
    }

    fn project_group(&self, cx: &mut Context<Self>) -> Div {
        let Some(project) = self.workspace.project.as_ref() else {
            return group("Project", None, cx)
                .child(
                    div()
                        .text_color(theme::dim(cx))
                        .child("No project open. A project is a Bevy game whose scripts you edit here."),
                )
                .child(
                    div()
                        .pt_1()
                        .flex()
                        .gap_1()
                        .child(
                            Button::new("explorer-open-project")
                                .outline()
                                .xsmall()
                                .tab_stop(false)
                                .icon(IconName::FolderOpen)
                                .label("Open project")
                                .on_click(dispatch(actions::OpenProject)),
                        )
                        .child(
                            Button::new("explorer-new-project")
                                .ghost()
                                .xsmall()
                                .tab_stop(false)
                                .icon(IconName::Plus)
                                .label("New project")
                                .on_click(dispatch(actions::NewProject)),
                        ),
                );
        };
        let rows = project.scripts().into_iter().enumerate().map(|(i, path)| {
            let open = self.doc.path.as_ref() == Some(&path);
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            div()
                .id(("script", i))
                .h(px(26.0))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .text_color(if open { theme::text(cx) } else { theme::dim(cx) })
                .when(open, |d| d.bg(theme::highlight(cx)))
                .hover(|s| s.bg(theme::button_hover(cx)).text_color(theme::text(cx)))
                .cursor_pointer()
                .child(Icon::new(IconName::FileText).small())
                .child(name)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.doc.path.as_ref() != Some(&path) && this.confirm_discard() {
                        this.open_path(path.clone(), window);
                    }
                    cx.notify();
                }))
        });
        let new_script = Button::new("script-add")
            .ghost()
            .xsmall()
            .tab_stop(false)
            .icon(IconName::Plus)
            .tooltip("New script")
            .on_click(cx.listener(|this, _, _, cx| {
                this.new_script_dialog();
                cx.notify();
            }));
        let catalog = vs_graph::components::project().len();
        group(format!("Project · {}", project.name), Some(new_script.into_any_element()), cx)
            .child(
                div()
                    .pb_1()
                    .text_size(px(11.0))
                    .text_color(theme::dim(cx))
                    .child(project.root.display().to_string()),
            )
            .children(rows)
            .child(
                div()
                    .mt_2()
                    .pt_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .border_t_1()
                    .border_color(theme::border(cx))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.0))
                            .text_color(theme::dim(cx))
                            .child(format!("Catalog · {catalog} component{}", if catalog == 1 { "" } else { "s" })),
                    )
                    .child(
                        Button::new("catalog-refresh")
                            .ghost()
                            .xsmall()
                            .tab_stop(false)
                            .icon(IconName::RefreshCw)
                            .tooltip("Refresh catalog: build the game to read its components")
                            .disabled(self.task_running())
                            .on_click(dispatch(actions::RefreshCatalog)),
                    )
                    .child(
                        Button::new("catalog-edit")
                            .ghost()
                            .xsmall()
                            .tab_stop(false)
                            .icon(IconName::Settings)
                            .tooltip("Edit catalog: choose which components the editor offers")
                            .on_click(dispatch(actions::EditCatalog)),
                    ),
            )
    }

    fn variables_group(&self, cx: &mut Context<Self>) -> Div {
        let editor = cx.entity().downgrade();
        let rows = self.doc.graph.variables.iter().enumerate().map(|(i, var)| {
            let editing_name = self.editing.as_ref().filter(|e| e.target == EditTarget::VarName(i));
            let editing_value = self.editing.as_ref().filter(|e| e.target == EditTarget::VarValue(i));
            let ty = var.value.ty();
            let type_label = match ty {
                PinType::Number => "num",
                PinType::Bool => "bool",
                PinType::String => "text",
                PinType::Vec3 => "vec3",
                PinType::Quat => "rot",
                PinType::Color => "color",
                _ => "?",
            };
            let wide = matches!(ty, PinType::Vec3 | PinType::Quat | PinType::Color);
            let editor = editor.clone();
            // Get, Set and Delete live in a menu, so the row keeps its width for the name.
            let more = Button::new(("var-more", i))
                .ghost()
                .xsmall()
                .tab_stop(false)
                .icon(IconName::Ellipsis)
                .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                    let run = |f: fn(&mut Editor, usize)| {
                        let editor = editor.clone();
                        move |_: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut App| {
                            editor
                                .update(cx, |e, cx| {
                                    f(e, i);
                                    cx.notify();
                                })
                                .ok();
                        }
                    };
                    menu.item(PopupMenuItem::new("Add Get node").on_click(run(|e, i| e.add_variable_node(i, "var.get"))))
                        .item(PopupMenuItem::new("Add Set node").on_click(run(|e, i| e.add_variable_node(i, "var.set"))))
                        .item(PopupMenuItem::new("Change type").on_click(run(|e, i| e.doc.cycle_variable_type(i))))
                        .separator()
                        .item(PopupMenuItem::new("Delete").on_click(run(Editor::remove_variable)))
                });
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    field(
                        SharedString::from(format!("var-name-{i}")),
                        editing_name.map_or(var.name.clone(), |e| format!("{}|", e.buffer)),
                        editing_name.is_some(),
                        cx,
                    )
                    .flex_1()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_edit(EditTarget::VarName(i));
                        cx.notify();
                    })),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("var-type-{i}")))
                        .w(px(38.0))
                        .py_0p5()
                        .flex()
                        .justify_center()
                        .rounded_sm()
                        .text_size(px(11.0))
                        .text_color(theme::pin(ty))
                        .bg(Hsla::from(theme::pin(ty)).opacity(0.12))
                        .cursor_pointer()
                        .child(type_label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.doc.cycle_variable_type(i);
                            cx.notify();
                        })),
                )
                .child(
                    field(
                        SharedString::from(format!("var-value-{i}")),
                        editing_value.map_or(var.value.display(), |e| format!("{}|", e.buffer)),
                        editing_value.is_some(),
                        cx,
                    )
                    .w(px(if wide { 96.0 } else { 56.0 }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Value::Bool(b)) = this.doc.graph.variables.get(i).map(|v| v.value.clone()) {
                            this.doc.checkpoint();
                            this.doc.graph.variables[i].value = Value::Bool(!b);
                        } else {
                            this.start_edit(EditTarget::VarValue(i));
                        }
                        cx.notify();
                    })),
                )
                .child(more)
        });
        let add = Button::new("var-add")
            .ghost()
            .xsmall()
            .tab_stop(false)
            .icon(IconName::Plus)
            .tooltip("Add variable")
            .on_click(cx.listener(|this, _, _, cx| {
                this.doc.add_variable();
                cx.notify();
            }));
        group("Variables", Some(add.into_any_element()), cx)
            .when(self.doc.graph.variables.is_empty(), |d| {
                d.child(div().text_color(theme::dim(cx)).child("No variables. A variable keeps a value between frames."))
            })
            .children(rows)
    }
}

impl Editor {
    /// Deletes variable `i` and its nodes. Any text edit is dropped, since edits point at
    /// variables by index and the indices after `i` shift.
    fn remove_variable(&mut self, i: usize) {
        if self.doc.remove_variable(i) {
            self.editing = None;
        }
    }

    /// Adds a Get or Set node for variable `i` in the middle of the canvas.
    fn add_variable_node(&mut self, i: usize, kind: &'static str) {
        let at = self.doc.to_graph(self.canvas.size() * 0.5) - V2::new(layout::NODE_W * 0.5, 30.0);
        self.doc.add_variable_node(i, kind, at);
    }
}
