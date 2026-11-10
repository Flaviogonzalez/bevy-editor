//! The left panel, what you navigate and declare: the open project (its plugins, in a tree of
//! folders that each become a plugin too, each plugin unfolding into its systems, components and
//! resources; and its catalog of components), then the graph's variables.


use std::path::{Path, PathBuf};

use gpui_kit::{
    Anchor, AnyElement, App, Context, Div, IntoElement, Hsla, SharedString,
    Window, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Disableable, Sizable};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{Compiled, NodeId, PinType, Value};

use crate::actions;
use crate::editor::{EditTarget, Editor};
use crate::theme;
use crate::panels::dialog::folder_plugin;
use crate::ui::{MONO, dispatch, field, group, panel, panel_title};
use crate::workspace::Outline;
use crate::canvas::layout::{self, V2};

impl Editor {
    /// The left panel: what you navigate and declare. The open project (its scripts and its
    /// catalog of components), then the graph's variables.
    pub(crate) fn explorer(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        panel("explorer", cx)
            .border_r_1()
            .child(panel_title("Explorer", cx))
            .child(self.project_group(compiled, cx))
            .child(self.variables_group(cx))
    }

    fn project_group(&self, compiled: &Compiled, cx: &mut Context<Self>) -> Div {
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
        let current = Outline::of(&self.doc.graph, &compiled.module, &self.script_name());
        let mut rows = Vec::new();
        self.tree_rows(&project.scripts_dir(), "", 0, &current, &mut rows, cx);
        let new_script = self.add_menu_button("script-add".into(), "", "New script or folder", cx);
        let catalog = vs_graph::components::project().len();
        group(format!("Project · {}", project.name), Some(new_script), cx)
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

/// What's in a folder of scripts, as the Explorer lists it: its folders, then its scripts, each
/// sorted by name. Read from disk on every render, like the script list was, so files added
/// outside the editor show up.
fn entries(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut all: Vec<PathBuf> =
        std::fs::read_dir(dir).map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
    all.sort();
    let (folders, files): (Vec<_>, Vec<_>) = all.into_iter().partition(|p| p.is_dir());
    (folders, files.into_iter().filter(|p| p.extension().is_some_and(|e| e == "lua")).collect())
}

impl Editor {
    /// The rows for the folder at `dir` (`rel` in the scripts folder, `""` for the top), at
    /// `depth`: each folder, with the plugin it becomes and, unless folded, what's in it; then
    /// each script.
    ///
    /// Each script is listed as the plugin it becomes, and unfolds into what that plugin holds.
    /// `current` is the open script's outline, read from the graph on screen; the others' come
    /// from disk (see [`Editor::scan_systems`]).
    fn tree_rows(
        &self,
        dir: &Path,
        rel: &str,
        depth: usize,
        current: &Outline,
        rows: &mut Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) {
        let indent = px(8.0 + depth as f32 * 14.0);
        let (folders, files) = entries(dir);
        for folder in folders {
            let name = folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let open = !self.chrome.collapsed.contains(&path);
            let toggle = path.clone();
            let row = div()
                .id(SharedString::from(format!("folder-{path}")))
                .h(px(26.0))
                .pl(indent)
                .pr_1()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_md()
                .text_color(theme::dim(cx))
                .hover(|s| s.bg(theme::button_hover(cx)).text_color(theme::text(cx)))
                .cursor_pointer()
                .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).xsmall())
                .child(Icon::new(if open { IconName::FolderOpen } else { IconName::Folder }).small())
                .child(div().text_color(theme::text(cx)).child(name))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(px(11.0))
                        .child(folder_plugin(&path)),
                )
                .child(self.add_menu_button(SharedString::from(format!("folder-add-{path}")), &path, "Add in this folder", cx))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.chrome.collapsed.remove(&toggle) {
                        this.chrome.collapsed.insert(toggle.clone());
                    }
                    cx.notify();
                }));
            rows.push(row.into_any_element());
            if open {
                self.tree_rows(&folder, &path, depth + 1, current, rows, cx);
            }
        }
        for path in files {
            let open = self.doc.path.as_ref() == Some(&path);
            let outline = if open { Some(current) } else { self.workspace.outlines.get(&path) };
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            // The open plugin starts unfolded and the rest folded; `collapsed` holds the ones
            // toggled away from that.
            let key = format!("plugin:{}", path.display());
            let expanded = open != self.chrome.collapsed.contains(&key);
            let chevron = div()
                .id(SharedString::from(format!("plugin-fold-{}", path.display())))
                .flex()
                .child(Icon::new(if expanded { IconName::ChevronDown } else { IconName::ChevronRight }).xsmall())
                .on_click(cx.listener(move |this, _, _, cx| {
                    // Folding shouldn't also open the plugin.
                    cx.stop_propagation();
                    if !this.chrome.collapsed.remove(&key) {
                        this.chrome.collapsed.insert(key.clone());
                    }
                    cx.notify();
                }));
            let to_open = path.clone();
            let row = div()
                .id(SharedString::from(format!("plugin-{}", path.display())))
                .h(px(26.0))
                .pl(indent)
                .pr_1()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_md()
                .text_color(if open { theme::text(cx) } else { theme::dim(cx) })
                .when(open, |d| d.bg(theme::highlight(cx)))
                .hover(|s| s.bg(theme::button_hover(cx)).text_color(theme::text(cx)))
                .cursor_pointer()
                .child(chevron)
                .child(Icon::new(IconName::Network).small())
                .child(div().text_color(theme::text(cx)).child(name))
                .when_some(outline, |d, outline| {
                    d.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(11.0))
                            .child(outline.plugin.clone()),
                    )
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_path(to_open.clone(), window);
                    cx.notify();
                }));
            rows.push(row.into_any_element());
            if expanded && let Some(outline) = outline {
                self.outline_rows(&path, outline, depth + 1, rows, cx);
            }
        }
    }

    /// What a plugin holds, under its row: a heading per kind of Bevy piece and a row per piece.
    /// Pressing a piece opens its plugin, if it isn't the open one, and centers the canvas on the
    /// node it comes from: a system's event, a component's card.
    fn outline_rows(&self, path: &Path, outline: &Outline, depth: usize, rows: &mut Vec<AnyElement>, cx: &mut Context<Self>) {
        let heading = |label: &str, count: usize, cx: &mut Context<Self>| {
            div()
                .h(px(22.0))
                .pl(px(8.0 + depth as f32 * 14.0 + 18.0))
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(px(11.0))
                .text_color(theme::dim(cx))
                .child(Icon::new(IconName::Folder).xsmall())
                .child(format!("{label} · {count}"))
                .into_any_element()
        };
        let item = |id: String, dot: Option<Hsla>, label: String, note: Option<&str>, target: Option<NodeId>, cx: &mut Context<Self>| {
            let path = path.to_path_buf();
            div()
                .id(SharedString::from(id))
                .h(px(22.0))
                .pl(px(8.0 + (depth + 1) as f32 * 14.0 + 18.0))
                .pr_2()
                .flex()
                .items_center()
                .gap_1p5()
                .rounded_md()
                .text_size(px(11.5))
                .text_color(theme::text(cx))
                .when_some(dot, |d, color| d.child(div().flex_none().w(px(7.0)).h(px(7.0)).rounded_full().bg(color)))
                .child(div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(MONO).child(label))
                .when_some(note, |d, note| {
                    d.child(div().flex_none().text_size(px(10.5)).text_color(theme::dim(cx)).child(note.to_string()))
                })
                .when_some(target, |d, node| {
                    d.cursor_pointer().hover(|s| s.bg(theme::button_hover(cx))).on_click(cx.listener(move |this, _, window, cx| {
                        this.show_in_plugin(&path, node, window);
                        cx.notify();
                    }))
                })
                .into_any_element()
        };
        let key = path.display().to_string();
        rows.push(heading("Systems", outline.systems.len(), cx));
        for (i, (name, event)) in outline.systems.iter().enumerate() {
            let dot = Some(Hsla::from(theme::system(i)));
            rows.push(item(format!("system-{key}-{i}"), dot, name.clone(), None, Some(*event), cx));
        }
        rows.push(heading("Components", outline.components.len(), cx));
        for (i, (name, card)) in outline.components.iter().enumerate() {
            // The script's own component has no card yet: every system's query filters by it.
            let note = card.is_none().then_some("in every query");
            rows.push(item(format!("component-{key}-{i}"), None, name.clone(), note, *card, cx));
        }
        rows.push(heading("Resources", 0, cx));
    }

    /// Opens the plugin at `path` unless it's the open one, then selects `node` and centers the
    /// canvas on it.
    fn show_in_plugin(&mut self, path: &Path, node: NodeId, window: &mut Window) {
        if self.doc.path.as_deref() != Some(path) {
            self.open_path(path.to_path_buf(), window);
            if self.doc.path.as_deref() != Some(path) {
                // It didn't open; `open_path` already said why.
                return;
            }
        }
        if self.doc.graph.node(node).is_some() {
            self.doc.focus_node(node, self.canvas.size());
        }
    }

    /// A `+` that offers a new script or a new folder in `folder` (`""` for the top).
    fn add_menu_button(&self, id: SharedString, folder: &str, tooltip: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let editor = cx.entity().downgrade();
        let folder = folder.to_string();
        let button = Button::new(id.clone()).ghost().xsmall().tab_stop(false).icon(IconName::Plus).tooltip(tooltip).dropdown_menu_with_anchor(
            Anchor::TopRight,
            move |menu, _, _| {
                let run = |f: fn(&mut Editor, &str)| {
                    let (editor, folder) = (editor.clone(), folder.clone());
                    move |_: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut App| {
                        editor
                            .update(cx, |e, cx| {
                                f(e, &folder);
                                cx.notify();
                            })
                            .ok();
                    }
                };
                menu.item(PopupMenuItem::new("New script").on_click(run(Editor::new_script_dialog)))
                    .item(PopupMenuItem::new("New folder").on_click(run(Editor::new_folder_dialog)))
            },
        );
        // On a folder's row, opening the menu shouldn't also fold the folder.
        div().id(SharedString::from(format!("{id}-wrap"))).on_click(|_, _, cx| cx.stop_propagation()).child(button).into_any_element()
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
