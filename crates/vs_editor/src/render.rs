//! Drawing the editor: toolbar, canvas (grid, wires, nodes, menu) and side panel.

use std::collections::BTreeMap;

use gpui_kit::{
    Anchor, App, Bounds, Context, Div, FontWeight, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Hsla, ScrollEvent, SharedString,
    Window, canvas, div, point, prelude::*, px, rgba, size,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Disableable, Selectable, Sizable};
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{Compiled, NodeId, PinType, Severity, Side, Value, library};

use crate::actions;
use crate::editor::{BottomTab, CodeView, Dialog, Drag, EditTarget, Editor};
use crate::layout::{self, NodeLayout, Rect, V2};
use crate::tasks::Outcome;
use crate::theme;

const MONO: &str = "Consolas";
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

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = window;
        let compiled = vs_graph::compile(&self.graph);
        let layouts = self.layouts();

        actions::register(div(), cx)
            .key_context(actions::CONTEXT)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::canvas_bg(cx))
            .text_color(theme::text(cx))
            .text_size(px(13.0))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.key_down(e, window);
                cx.notify();
            }))
            .relative()
            .child(self.toolbar(&compiled, cx))
            // The panels around the canvas can be resized by dragging the lines between them.
            // gpui-component keeps their sizes in window state keyed by each group's id, so they
            // survive re-renders. The side panels are `fixed`: resizing the window grows or
            // shrinks the canvas, not them.
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("main-panels")
                        .child(
                            resizable_panel()
                                .size(px(272.0))
                                .size_range(px(200.0)..px(480.0))
                                .fixed()
                                .child(self.explorer(cx)),
                        )
                        .child(
                            resizable_panel().child(
                                v_resizable("center-panels")
                                    .child(resizable_panel().child(self.canvas_area(&layouts, &compiled, cx)))
                                    .child(
                                        resizable_panel()
                                            .size(px(240.0))
                                            .size_range(px(120.0)..px(600.0))
                                            .fixed()
                                            .visible(self.show_bottom_panel)
                                            .child(self.bottom_panel(&compiled, cx)),
                                    ),
                            ),
                        )
                        .child(
                            resizable_panel()
                                .size(px(300.0))
                                .size_range(px(220.0)..px(520.0))
                                .fixed()
                                .child(self.inspector(cx)),
                        ),
                ),
            )
            .when(self.dialog.is_some(), |d| d.child(self.dialog_element(cx)))
            .child(
                div()
                    .h(px(24.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .bg(theme::toolbar_bg(cx))
                    .border_t_1()
                    .border_color(theme::border(cx))
                    .text_size(px(12.0))
                    .text_color(theme::dim(cx))
                    .child(self.status.clone()),
            )
    }
}

fn button(id: &'static str, label: &str, cx: &App) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .px_2p5()
        .py_1()
        .rounded_md()
        .bg(theme::button_bg(cx))
        .hover(|s| s.bg(theme::button_hover(cx)))
        .cursor_pointer()
        .child(label.to_string())
}

/// A quiet toolbar button showing only an icon; its tooltip names the command and its shortcut.
fn icon_button(id: &'static str, icon: IconName, tooltip: &'static str, action: &dyn gpui_kit::Action) -> Button {
    let boxed = action.boxed_clone();
    Button::new(id)
        .ghost()
        .small()
        .tab_stop(false)
        .icon(icon)
        .tooltip_with_action(tooltip, action, Some(actions::CONTEXT))
        .on_click(move |_, window, cx| window.dispatch_action(boxed.boxed_clone(), cx))
}

/// What some menu items depend on, captured when the toolbar renders.
#[derive(Clone, Copy)]
struct MenuState {
    /// A project is open, so the project commands apply.
    project: bool,
    /// A preview or build is running.
    running: bool,
    /// The panel under the canvas is showing.
    panel: bool,
}

fn file_menu(menu: PopupMenu, _: MenuState) -> PopupMenu {
    menu.menu("New script", Box::new(actions::NewScript))
        .menu("Open script…", Box::new(actions::OpenScript))
        .separator()
        .menu("Save", Box::new(actions::Save))
        .menu("Save as…", Box::new(actions::SaveAs))
        .separator()
        .menu("New project…", Box::new(actions::NewProject))
        .menu("Open project…", Box::new(actions::OpenProject))
        .separator()
        .menu("Settings…", Box::new(actions::Settings))
}

fn edit_menu(menu: PopupMenu, _: MenuState) -> PopupMenu {
    menu.menu("Undo", Box::new(actions::Undo))
        .menu("Redo", Box::new(actions::Redo))
        .separator()
        .menu("Duplicate", Box::new(actions::Duplicate))
        .menu("Delete", Box::new(actions::Delete))
        .menu("Select all", Box::new(actions::SelectAll))
}

fn view_menu(menu: PopupMenu, state: MenuState) -> PopupMenu {
    menu.menu("Add node…", Box::new(actions::AddNode))
        .menu("Frame all", Box::new(actions::FrameAll))
        .separator()
        .menu_with_check("Bottom panel", state.panel, Box::new(actions::ToggleBottomPanel))
}

fn project_menu(menu: PopupMenu, state: MenuState) -> PopupMenu {
    let idle = state.project && !state.running;
    menu.menu_with_disabled("Preview", Box::new(actions::Preview), !idle)
        .menu_with_disabled("Build", Box::new(actions::Build), !idle)
        .menu_with_disabled("Stop", Box::new(actions::Stop), !state.running)
        .separator()
        .menu_with_disabled("Refresh catalog", Box::new(actions::RefreshCatalog), !state.project)
        .menu_with_disabled("Edit catalog", Box::new(actions::EditCatalog), !state.project)
}

impl Editor {
    /// The top bar, laid out like a desktop editor's: menus on the left, the most used commands
    /// as icon buttons, run controls in the middle, and the file and its status on the right.
    /// Every control dispatches an action from actions.rs, so it behaves like its shortcut.
    fn toolbar(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = compiled.errors();
        let warnings = compiled.problems.len() - errors;
        let (pill, pill_color) = match (errors, warnings) {
            (0, 0) => ("Compiles".to_string(), theme::ok(cx)),
            (0, w) => (format!("{w} warning{}", if w == 1 { "" } else { "s" }), theme::warning(cx)),
            (e, _) => (format!("{e} error{}", if e == 1 { "" } else { "s" }), theme::error(cx)),
        };
        let state =
            MenuState { project: self.project.is_some(), running: self.task_running(), panel: self.show_bottom_panel };
        let focus = self.focus.clone();
        let menu = move |id: &'static str, label: &'static str, items: fn(PopupMenu, MenuState) -> PopupMenu| {
            let focus = focus.clone();
            Button::new(id).ghost().small().tab_stop(false).label(label).dropdown_menu(move |menu, _, _| {
                items(menu.action_context(focus.clone()).min_w(px(220.0)), state)
            })
        };
        let separator = || div().w(px(1.0)).h(px(18.0)).mx_1().bg(theme::border(cx));

        div()
            .h(px(40.0))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .bg(theme::toolbar_bg(cx))
            .border_b_1()
            .border_color(theme::border(cx))
            // Using the bar dismisses the add-node menu, which would otherwise hold the keyboard.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.menu.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(menu("menu-file", "File", file_menu))
            .child(menu("menu-edit", "Edit", edit_menu))
            .child(menu("menu-view", "View", view_menu))
            .child(menu("menu-project", "Project", project_menu))
            .child(separator())
            .child(icon_button("tb-undo", IconName::Undo, "Undo", &actions::Undo))
            .child(icon_button("tb-redo", IconName::Redo, "Redo", &actions::Redo))
            .child(icon_button("tb-frame", IconName::Frame, "Frame all", &actions::FrameAll))
            .child(
                icon_button("tb-panel", IconName::PanelBottom, "Bottom panel", &actions::ToggleBottomPanel)
                    .selected(state.panel),
            )
            .child(div().flex_1())
            .when(state.project, |d| {
                d.child(
                    Button::new("tb-preview")
                        .primary()
                        .small()
                        .tab_stop(false)
                        .icon(IconName::Play)
                        .label("Preview")
                        .disabled(state.running)
                        .tooltip_with_action("Run the game with this script", &actions::Preview, Some(actions::CONTEXT))
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(actions::Preview), cx)),
                )
                .child(
                    Button::new("tb-build")
                        .outline()
                        .small()
                        .tab_stop(false)
                        .label("Build")
                        .disabled(state.running)
                        .tooltip_with_action("Compile a release build", &actions::Build, Some(actions::CONTEXT))
                        .on_click(|_, window, cx| window.dispatch_action(Box::new(actions::Build), cx)),
                )
                .when(state.running, |d| {
                    d.child(
                        Button::new("tb-stop")
                            .danger()
                            .small()
                            .tab_stop(false)
                            .icon(IconName::Square)
                            .label("Stop")
                            .tooltip_with_action("Stop", &actions::Stop, Some(actions::CONTEXT))
                            .on_click(|_, window, cx| window.dispatch_action(Box::new(actions::Stop), cx)),
                    )
                })
            })
            .child(div().flex_1())
            .child(
                div()
                    .px_2()
                    .text_color(theme::dim(cx))
                    .child(format!("{}{}", self.file_name(), if self.history.is_dirty() { " •" } else { "" })),
            )
            .child(
                div()
                    .px_2p5()
                    .py_0p5()
                    .rounded_full()
                    .border_1()
                    .border_color(pill_color)
                    .text_color(pill_color)
                    .text_size(px(12.0))
                    .child(pill),
            )
            .child(separator())
            .child(icon_button("tb-settings", IconName::Settings, "Settings", &actions::Settings))
    }

    // Canvas -----------------------------------------------------------------------------------

    fn canvas_area(
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

    fn node_element(&self, node: &vs_graph::Node, l: &NodeLayout, flag: Option<Severity>, cx: &App) -> Div {
        let z = self.zoom;
        let def = library::find(&node.kind);
        let title = match (node.var.as_ref().or(node.field.as_ref()), def) {
            (Some(subject), Some(d)) => format!("{} {subject}", d.title),
            (None, Some(d)) => d.title.to_string(),
            _ => format!("? {}", node.kind),
        };
        let category = def.map_or("", |d| d.category);
        let selected = self.selected.contains(&node.id);
        let border = if selected {
            theme::selection(cx)
        } else {
            match flag {
                Some(Severity::Error) => theme::error(cx),
                Some(Severity::Warning) => theme::warning(cx),
                None => theme::border(cx),
            }
        };
        let screen = self.to_screen(l.rect.min);
        let rel = |p: V2| (p - l.rect.min) * z;

        let mut el = div()
            .absolute()
            .left(px(screen.x))
            .top(px(screen.y))
            .w(px(l.rect.w() * z))
            .h(px(l.rect.h() * z))
            .rounded(px(6.0 * z))
            .bg(theme::node_bg(cx))
            .border(px(if selected { 2.0 } else { 1.0 }))
            .border_color(border)
            .text_size(px(12.0 * z))
            .shadow_md()
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w_full()
                    .h(px(layout::HEADER_H * z))
                    .rounded_t(px(6.0 * z))
                    .bg(theme::category(category))
                    .px(px(8.0 * z))
                    .flex()
                    .items_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(12.5 * z))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(title),
            );

        let row_text_h = layout::ROW_H * z;
        for pin in &l.inputs {
            let c = rel(pin.center);
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, z, cx));
            if pin.def.ty != PinType::Exec || l.inputs.iter().filter(|p| p.def.ty == PinType::Exec).count() > 1 {
                el = el.child(
                    div()
                        .absolute()
                        .left(px(10.0 * z))
                        .top(px(c.y - row_text_h * 0.5))
                        .h(px(row_text_h))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_color(theme::dim(cx))
                        .child(pin.def.name.clone()),
                );
            }
        }
        for pin in &l.outputs {
            let c = rel(pin.center);
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, z, cx));
            let label = if pin.def.ty == PinType::Exec && pin.def.name == "then" {
                String::new()
            } else {
                pin.def.name.clone()
            };
            el = el.child(
                div()
                    .absolute()
                    .right(px(10.0 * z))
                    .top(px(c.y - row_text_h * 0.5))
                    .h(px(row_text_h))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .text_color(theme::dim(cx))
                    .child(label),
            );
        }
        for (pin, rect) in &l.fields {
            let value = node.values.get(pin);
            let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::Field(node.id, pin.clone()));
            let text = match (editing, value) {
                (Some(e), _) => format!("{}|", e.buffer),
                (None, Some(Value::Str(s))) => format!("\"{s}\""),
                (None, Some(v)) => v.display(),
                (None, None) => String::new(),
            };
            let p = rel(rect.min);
            let swatch = match value {
                Some(Value::Color { rgba: c }) if editing.is_none() => {
                    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                    Some(rgba(byte(c[0]) << 24 | byte(c[1]) << 16 | byte(c[2]) << 8 | byte(c[3])))
                }
                _ => None,
            };
            el = el.child(
                div()
                    .absolute()
                    .left(px(p.x))
                    .top(px(p.y))
                    .w(px(rect.w() * z))
                    .h(px(rect.h() * z))
                    .px(px(4.0 * z))
                    .rounded(px(3.0 * z))
                    .bg(theme::field_bg(cx))
                    .border_1()
                    .border_color(if editing.is_some() { theme::selection(cx) } else { theme::border(cx) })
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.0 * z))
                    .font_family(MONO)
                    .gap(px(4.0 * z))
                    .when_some(swatch, |d, color| {
                        d.child(
                            div()
                                .w(px(10.0 * z))
                                .h(px(10.0 * z))
                                .flex_none()
                                .rounded(px(2.0 * z))
                                .border_1()
                                .border_color(theme::border(cx))
                                .bg(color),
                        )
                    })
                    .child(text),
            );
        }
        el
    }

    fn menu_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.menu.as_ref().expect("checked by the caller");
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
                if let Some(menu) = &mut this.menu {
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

    // Side panel -------------------------------------------------------------------------------

    // Panels -----------------------------------------------------------------------------------

    /// The left panel: what you navigate and declare. The open project (its scripts and its
    /// catalog of components), then the graph's variables.
    fn explorer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel("explorer", cx)
            .border_r_1()
            .child(panel_title("Explorer", cx))
            .child(self.project_group(cx))
            .child(self.variables_group(cx))
    }

    /// The right panel: what you edit. The selected node, what it does, its name and its pins.
    fn inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        panel("inspector", cx)
            .border_l_1()
            .child(panel_title("Inspector", cx))
            .child(self.node_details(cx))
    }

    fn project_group(&self, cx: &mut Context<Self>) -> Div {
        let Some(project) = self.project.as_ref() else {
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
            let open = self.path.as_ref() == Some(&path);
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
                    if this.path.as_ref() != Some(&path) && this.confirm_discard() {
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
        let rows = self.graph.variables.iter().enumerate().map(|(i, var)| {
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
                        .item(PopupMenuItem::new("Change type").on_click(run(Editor::cycle_variable_type)))
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
                            this.cycle_variable_type(i);
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
                        if let Some(Value::Bool(b)) = this.graph.variables.get(i).map(|v| v.value.clone()) {
                            this.checkpoint();
                            this.graph.variables[i].value = Value::Bool(!b);
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
                this.add_variable();
                cx.notify();
            }));
        group("Variables", Some(add.into_any_element()), cx)
            .when(self.graph.variables.is_empty(), |d| {
                d.child(div().text_color(theme::dim(cx)).child("No variables. A variable keeps a value between frames."))
            })
            .children(rows)
    }

    fn node_details(&self, cx: &mut Context<Self>) -> Div {
        let hint = |text: String| div().px_3().py_3().text_color(theme::dim(cx)).child(text);
        let id = match self.selected.iter().next() {
            Some(id) if self.selected.len() == 1 => *id,
            Some(_) => return hint(format!("{} nodes selected", self.selected.len())),
            None => return hint("Select a node to see what it does.".into()),
        };
        let Some((node, def)) = self.graph.node(id).and_then(|n| library::find(&n.kind).map(|d| (n, d))) else {
            return hint("Unknown node".into());
        };
        let pins = self.graph.pins_of(node);
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
        let title = match node.var.as_ref().or(node.field.as_ref()) {
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
            .child(
                group("Name", None, cx).child(field("node-name".into(), name, editing.is_some(), cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.start_edit(EditTarget::NodeName(id));
                        cx.notify();
                    }),
                )),
            )
            .when(!pins.inputs.is_empty(), |d| {
                d.child(group("Inputs", None, cx).children(pins.inputs.iter().map(pin_row)))
            })
            .when(!pins.outputs.is_empty(), |d| {
                d.child(group("Outputs", None, cx).children(pins.outputs.iter().map(pin_row)))
            })
    }

    /// The panel under the canvas: what the editor tells you back. The compiler's problems, the
    /// output of the last preview or build, and the generated code.
    fn bottom_panel(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        const TABS: [BottomTab; 3] = [BottomTab::Problems, BottomTab::Output, BottomTab::Code];
        let selected = TABS.iter().position(|t| *t == self.bottom_tab).unwrap_or(0);
        let count = compiled.problems.len();
        let output = match self.task.borrow().as_ref().map(|t| t.finished()) {
            Some(None) => "Output · running",
            _ => "Output",
        };
        let code_toggle = |id: &'static str, label: &'static str, view: CodeView| {
            Button::new(id).ghost().xsmall().tab_stop(false).label(label).selected(self.code_view == view).on_click(
                cx.listener(move |this, _, _, cx| {
                    this.code_view = view;
                    cx.notify();
                }),
            )
        };
        let tabs = TabBar::new("bottom-tabs")
            .underline()
            .small()
            .selected_index(selected)
            .prefix(div().w(px(6.0)))
            .child(Tab::new().label(if count == 0 { "Problems".to_string() } else { format!("Problems ({count})") }))
            .child(Tab::new().label(output))
            .child(Tab::new().label("Code"))
            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                this.bottom_tab = TABS[*ix];
                cx.notify();
            }))
            .suffix(
                div()
                    .h_full()
                    .pr_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(self.bottom_tab == BottomTab::Code, |d| {
                        d.child(code_toggle("view-lua", "Lua (preview)", CodeView::Lua))
                            .child(code_toggle("view-rust", "Rust (build)", CodeView::Rust))
                    })
                    .child(icon_button("panel-hide", IconName::Close, "Hide panel", &actions::ToggleBottomPanel)),
            );
        let body = match self.bottom_tab {
            BottomTab::Problems => self.problems_list(compiled, cx).into_any_element(),
            BottomTab::Output => self.output_log(cx).into_any_element(),
            BottomTab::Code => {
                let code = match self.code_view {
                    CodeView::Lua => compiled.lua.clone(),
                    CodeView::Rust => self.rust_code(compiled),
                };
                code_block(&code, if self.code_view == CodeView::Lua { "--" } else { "//" }, cx).into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::panel_bg(cx))
            .border_t_1()
            .border_color(theme::border(cx))
            .child(tabs)
            .child(div().id("bottom-body").flex_1().min_h_0().overflow_y_scroll().p_2().child(body))
    }

    fn problems_list(&self, compiled: &Compiled, cx: &mut Context<Self>) -> Div {
        if compiled.problems.is_empty() {
            return div().p_1().text_color(theme::dim(cx)).child("No problems. The graph compiles.");
        }
        div().flex().flex_col().children(compiled.problems.iter().enumerate().map(|(i, p)| {
            let (icon, color) = if p.severity == Severity::Error {
                (IconName::CircleX, theme::error(cx))
            } else {
                (IconName::TriangleAlert, theme::warning(cx))
            };
            let node = p.node;
            div()
                .id(("problem", i))
                .px_2()
                .py_1()
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .hover(|s| s.bg(theme::button_hover(cx)))
                .when(node.is_some(), |d| d.cursor_pointer())
                .child(Icon::new(icon).small().text_color(color))
                .child(div().flex_1().child(p.message.clone()))
                .when_some(node, |d, id| d.child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(format!("#{id}"))))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(id) = node {
                        this.focus_node(id);
                    }
                    cx.notify();
                }))
        }))
    }

    fn output_log(&self, cx: &App) -> Div {
        let task = self.task.borrow();
        let Some(task) = task.as_ref() else {
            return div()
                .p_1()
                .text_color(theme::dim(cx))
                .child("Nothing has run yet. Preview or build the game from the Project menu.");
        };
        let state = match task.finished() {
            None => "running",
            Some(Outcome::Succeeded) => "done",
            Some(Outcome::Failed) => "failed",
            Some(Outcome::Stopped) => "stopped",
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(format!("{} · {state}", task.label)))
            .child(div().font_family(MONO).text_size(px(11.5)).children(task.tail(12).into_iter().map(|line| {
                let color = if line.trim_start().starts_with("error") {
                    theme::error(cx)
                } else if line.trim_start().starts_with("warning") {
                    theme::warning(cx)
                } else {
                    theme::dim(cx)
                };
                div().text_color(color).child(if line.is_empty() { " ".to_string() } else { line })
            })))
    }

    /// The file `Build` writes for this script.
    fn rust_code(&self, compiled: &Compiled) -> String {
        if compiled.errors() > 0 {
            return "// Fix the errors to see the Rust code.".into();
        }
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or("script".into(), |s| s.to_string_lossy().into_owned());
        let files = vs_graph::rust::emit_export(&[vs_graph::rust::Script { path: stem, ir: &compiled.module }]);
        files.into_iter().find(|(name, _)| name != "scripts/mod.rs").map(|(_, code)| code).unwrap_or_default()
    }

    fn dialog_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog = self.dialog.as_ref().expect("checked by the caller");
        let body = match dialog {
            Dialog::Settings => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Settings"))
                .child(div().text_color(theme::dim(cx)).child("New projects are created in"))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            field("projects-dir".into(), self.config.projects_dir.display().to_string(), false, cx)
                                .flex_1(),
                        )
                        .child(button("change-dir", "Change…", cx).on_click(cx.listener(|this, _, _, cx| {
                            this.change_projects_dir();
                            cx.notify();
                        }))),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme::dim(cx))
                        .child(format!("Saved in {}", vs_project::Config::path().display())),
                ),
            Dialog::NewProject { buffer } | Dialog::NewScript { buffer } => {
                let (title, hint) = match dialog {
                    Dialog::NewProject { .. } => (
                        "New project",
                        format!("It goes in {}", self.config.projects_dir.display()),
                    ),
                    _ => ("New script", "It becomes a component you can put on entities".to_string()),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(field("dialog-name".into(), format!("{buffer}|"), true, cx))
                    .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child(hint))
            }
        };
        let confirm = match dialog {
            Dialog::Settings => "Close",
            _ => "Create",
        };
        div()
            .id("dialog-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0000_0088))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.dialog = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("dialog")
                    .w(px(460.0))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .rounded_lg()
                    .bg(theme::panel_bg(cx))
                    .border_1()
                    .border_color(theme::border(cx))
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .when(confirm != "Close", |d| {
                                d.child(button("dialog-cancel", "Cancel", cx).on_click(cx.listener(|this, _, _, cx| {
                                    this.dialog = None;
                                    cx.notify();
                                })))
                            })
                            .child(button("dialog-ok", confirm, cx).bg(theme::primary(cx).0).text_color(theme::primary(cx).1).on_click(cx.listener(
                                |this, _, window, cx| {
                                    match this.dialog {
                                        Some(Dialog::Settings) => this.dialog = None,
                                        _ => this.confirm_dialog(window),
                                    }
                                    cx.notify();
                                },
                            ))),
                    ),
            )
    }

}

fn code_block(code: &str, comment: &'static str, cx: &App) -> Div {
    div()
        .p_2()
        .rounded_md()
        .bg(theme::field_bg(cx))
        .font_family(MONO)
        .text_size(px(11.5))
        .children(code.lines().map(|line| {
            let color = if line.trim_start().starts_with(comment) { theme::dim(cx) } else { theme::text(cx) };
            div().text_color(color).child(if line.is_empty() {
                " ".to_string()
            } else {
                line.replace('\t', "    ")
            })
        }))
}

/// A side panel: a scrolling column on the panel background, filling the resizable slot it
/// sits in (the slot decides the width).
fn panel(id: &'static str, cx: &App) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::panel_bg(cx))
        .border_color(theme::border(cx))
        .overflow_y_scroll()
}

/// The name of a panel, in its header row.
fn panel_title(title: &str, cx: &App) -> Div {
    div()
        .h(px(34.0))
        .flex_none()
        .px_3()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(theme::border(cx))
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::dim(cx))
        .child(title.to_uppercase())
}

/// A titled group of rows inside a panel, with an optional action (such as an add button) at the
/// right of its title.
fn group(title: impl Into<SharedString>, action: Option<gpui_kit::AnyElement>, cx: &App) -> Div {
    let title: SharedString = title.into();
    div()
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .h(px(22.0))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::dim(cx))
                        .child(title.to_uppercase()),
                )
                .children(action),
        )
}

/// A click handler that runs `action`, as its menu item or shortcut would.
fn dispatch<A: gpui_kit::Action>(action: A) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx)
}

fn field(id: SharedString, text: String, active: bool, cx: &App) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .bg(theme::field_bg(cx))
        .border_1()
        .border_color(if active { theme::selection(cx) } else { theme::border(cx) })
        .font_family(MONO)
        .text_size(px(12.0))
        .whitespace_nowrap()
        .overflow_hidden()
        .cursor_text()
        .child(text)
}

fn pin_dot(center: V2, ty: PinType, connected: bool, z: f32, cx: &App) -> Div {
    let r = layout::PIN_R * z;
    let color: Hsla = theme::pin(ty).into();
    let dot = div()
        .absolute()
        .left(px(center.x - r))
        .top(px(center.y - r))
        .w(px(r * 2.0))
        .h(px(r * 2.0))
        .border(px(1.5 * z.max(0.7)))
        .border_color(color)
        .bg(if connected { color } else { theme::node_bg(cx) });
    // Exec pins are square, data pins round.
    if ty == PinType::Exec { dot.rounded(px(1.5 * z)) } else { dot.rounded_full() }
}

fn paint_scene(scene: &Scene, bounds: Bounds<gpui_kit::Pixels>, window: &mut Window) {
    let o = V2::new(bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
    let w = bounds.size.width.as_f32();
    let h = bounds.size.height.as_f32();
    let quad = |x: f32, y: f32, qw: f32, qh: f32, color: Hsla, window: &mut Window| {
        window.paint_quad(
            gpui_kit::fill(Bounds::new(point(px(x), px(y)), size(px(qw), px(qh))), color),
        );
    };

    // Grid: minor lines every 20 units, major every 100.
    let step = 20.0 * scene.zoom;
    if step >= 6.0 {
        let first_x = (-scene.pan.x / step).floor() as i64;
        let first_y = (-scene.pan.y / step).floor() as i64;
        let cols = (w / step).ceil() as i64 + 1;
        let rows = (h / step).ceil() as i64 + 1;
        for i in first_x..first_x + cols {
            let x = scene.pan.x + i as f32 * step;
            let color = if i % 5 == 0 { scene.grid_major } else { scene.grid_minor };
            quad(o.x + x, o.y, 1.0, h, color, window);
        }
        for j in first_y..first_y + rows {
            let y = scene.pan.y + j as f32 * step;
            let color = if j % 5 == 0 { scene.grid_major } else { scene.grid_minor };
            quad(o.x, o.y + y, w, 1.0, color, window);
        }
    }

    for wire in &scene.wires {
        let a = wire.a + o;
        let b = wire.b + o;
        let dx = ((b.x - a.x).abs() * 0.5).max(40.0 * scene.zoom);
        let mut path = PathBuilder::stroke(px(wire.width));
        path.move_to(point(px(a.x), px(a.y)));
        path.cubic_bezier_to(
            point(px(b.x), px(b.y)),
            point(px(a.x + dx), px(a.y)),
            point(px(b.x - dx), px(b.y)),
        );
        if let Ok(path) = path.build() {
            window.paint_path(path, wire.color);
        }
    }

    if let Some(sel) = &scene.selection {
        window.paint_quad(
            gpui_kit::fill(
                Bounds::new(point(px(o.x + sel.min.x), px(o.y + sel.min.y)), size(px(sel.w()), px(sel.h()))),
                scene.selection_color.opacity(0.08),
            )
            .border_widths(px(1.0))
            .border_color(scene.selection_color),
        );
    }
}
