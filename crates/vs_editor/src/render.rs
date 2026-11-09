//! Drawing the editor: toolbar, canvas (grid, wires, nodes, menu) and side panel.

use std::collections::BTreeMap;

use gpui_kit::{
    Bounds, Context, Div, FontWeight, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Rgba, ScrollEvent, SharedString,
    Window, canvas, div, point, prelude::*, px, rgba, size,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{Compiled, NodeId, PinType, Severity, Side, Value, library};

use crate::editor::{CodeView, Dialog, Drag, EditTarget, Editor};
use crate::layout::{self, NodeLayout, Rect, V2};
use crate::tasks::Outcome;
use crate::theme;

const MONO: &str = "Consolas";
pub const MENU_ROWS: usize = 10;

struct WireSeg {
    a: V2,
    b: V2,
    color: Rgba,
    width: f32,
}

/// Everything the canvas paint closure needs, captured by value.
struct Scene {
    pan: V2,
    zoom: f32,
    wires: Vec<WireSeg>,
    selection: Option<Rect>,
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = window;
        let compiled = vs_graph::compile(&self.graph);
        let layouts = self.layouts();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::canvas_bg())
            .text_color(theme::text())
            .text_size(px(13.0))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.key_down(e, window);
                cx.notify();
            }))
            .relative()
            .child(self.toolbar(&compiled, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.canvas_area(&layouts, &compiled, cx))
                    .child(self.side_panel(&compiled, cx)),
            )
            .when(self.dialog.is_some(), |d| d.child(self.dialog_element(cx)))
            .child(
                div()
                    .h(px(24.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .bg(theme::toolbar_bg())
                    .border_t_1()
                    .border_color(theme::border())
                    .text_size(px(12.0))
                    .text_color(theme::dim())
                    .child(self.status.clone()),
            )
    }
}

fn button(id: &'static str, label: &str) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .px_2p5()
        .py_1()
        .rounded_md()
        .bg(theme::button_bg())
        .hover(|s| s.bg(theme::border()))
        .cursor_pointer()
        .child(label.to_string())
}

impl Editor {
    fn toolbar(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = compiled.errors();
        let warnings = compiled.problems.len() - errors;
        let (pill, pill_color) = match (errors, warnings) {
            (0, 0) => ("Compiles".to_string(), theme::ok()),
            (0, w) => (format!("{w} warning{}", if w == 1 { "" } else { "s" }), theme::warning()),
            (e, _) => (format!("{e} error{}", if e == 1 { "" } else { "s" }), theme::error()),
        };
        let title = format!("{}{}", self.file_name(), if self.history.is_dirty() { " •" } else { "" });

        div()
            .h(px(40.0))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .bg(theme::toolbar_bg())
            .border_b_1()
            .border_color(theme::border())
            .child(button("new", "New").on_click(cx.listener(|this, _, window, cx| {
                this.new_graph(window);
                cx.notify();
            })))
            .child(button("open", "Open").on_click(cx.listener(|this, _, window, cx| {
                this.open(window);
                cx.notify();
            })))
            .child(button("save", "Save").on_click(cx.listener(|this, _, window, cx| {
                this.save(window);
                cx.notify();
            })))
            .child(button("save-as", "Save as").on_click(cx.listener(|this, _, window, cx| {
                this.save_as(window);
                cx.notify();
            })))
            .child(div().w(px(8.0)))
            .child(button("undo", "Undo").on_click(cx.listener(|this, _, _, cx| {
                this.undo();
                cx.notify();
            })))
            .child(button("redo", "Redo").on_click(cx.listener(|this, _, _, cx| {
                this.redo();
                cx.notify();
            })))
            .child(button("frame", "Frame").on_click(cx.listener(|this, _, _, cx| {
                this.frame();
                cx.notify();
            })))
            .child(
                button("code", if self.show_code { "Hide code" } else { "Show code" }).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.show_code = !this.show_code;
                        cx.notify();
                    }),
                ),
            )
            .child(div().w(px(8.0)))
            .child(button("new-project", "New project").on_click(cx.listener(|this, _, _, cx| {
                this.new_project_dialog();
                cx.notify();
            })))
            .child(button("open-project", "Open project").on_click(cx.listener(|this, _, window, cx| {
                this.open_project(window);
                cx.notify();
            })))
            .when(self.project.is_some(), |d| {
                let running = self.task_running();
                d.child(button("preview", "▶ Preview").when(running, |b| b.opacity(0.5)).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.run_preview(window, cx);
                        cx.notify();
                    },
                )))
                .child(button("build", "Build").when(running, |b| b.opacity(0.5)).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.build(window, cx);
                        cx.notify();
                    },
                )))
                .when(running, |d| {
                    d.child(button("stop", "■ Stop").text_color(theme::error()).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.stop_task();
                            cx.notify();
                        },
                    )))
                })
            })
            .child(button("settings", "Settings").on_click(cx.listener(|this, _, _, cx| {
                this.open_settings();
                cx.notify();
            })))
            .child(div().flex_1())
            .child(div().text_color(theme::dim()).child(title))
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

        let scene = self.scene(layouts);
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
            .h_full()
            .relative()
            .overflow_hidden()
            .bg(theme::canvas_bg())
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
                area = area.child(self.node_element(node, l, flagged.get(&l.id).copied()));
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
            .bg(theme::panel_bg())
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

    fn scene(&self, layouts: &[NodeLayout]) -> Scene {
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
                color: if lit { theme::text() } else { theme::pin(ty) },
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
                color: theme::pin(ty),
                width: 2.0 * self.zoom.max(0.6),
            });
        }
        let selection = match &self.drag {
            Drag::Select { start, cursor, .. } => {
                Some(Rect::from_points(self.to_screen(*start), self.to_screen(*cursor)))
            }
            _ => None,
        };
        Scene { pan: self.pan, zoom: self.zoom, wires, selection }
    }

    fn node_element(&self, node: &vs_graph::Node, l: &NodeLayout, flag: Option<Severity>) -> Div {
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
            theme::accent()
        } else {
            match flag {
                Some(Severity::Error) => theme::error(),
                Some(Severity::Warning) => theme::warning(),
                None => theme::border(),
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
            .bg(theme::node_bg())
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
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, z));
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
                        .text_color(theme::dim())
                        .child(pin.def.name.clone()),
                );
            }
        }
        for pin in &l.outputs {
            let c = rel(pin.center);
            el = el.child(pin_dot(c, pin.def.ty, pin.connected, z));
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
                    .text_color(theme::dim())
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
                    .bg(theme::field_bg())
                    .border_1()
                    .border_color(if editing.is_some() { theme::accent() } else { theme::border() })
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
                                .border_color(theme::border())
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
                .when(highlighted, |d| d.bg(theme::accent()))
                .hover(|s| s.bg(theme::border()))
                .cursor_pointer()
                .child(entry.label.clone())
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(if highlighted { theme::text() } else { theme::dim() })
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
            .bg(theme::panel_bg())
            .border_1()
            .border_color(theme::border())
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
            .child(div().px_2().pt_1().text_size(px(11.0)).text_color(theme::dim()).child(title))
            .child(
                div()
                    .mx_1()
                    .my_1()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(theme::field_bg())
                    .border_1()
                    .border_color(theme::accent())
                    .child(if menu.filter.is_empty() {
                        div().text_color(theme::dim()).child("Type to search…")
                    } else {
                        div().child(format!("{}|", menu.filter))
                    }),
            )
            .children(rows)
            .when(entries.is_empty(), |d| {
                d.child(div().px_2().py_1().text_color(theme::dim()).child("No matching nodes"))
            })
    }

    // Side panel -------------------------------------------------------------------------------

    fn side_panel(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("side-panel")
            .w(px(340.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::panel_bg())
            .border_l_1()
            .border_color(theme::border())
            .overflow_y_scroll()
            .when(self.project.is_some(), |d| d.child(self.project_section(cx)))
            .when(self.task.borrow().is_some(), |d| d.child(self.output_section()))
            .child(self.selection_section(cx))
            .child(self.variables_section(cx))
            .child(self.problems_section(compiled, cx))
            .when(self.show_code, |d| d.child(self.code_section(compiled, cx)))
    }

    fn project_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project = self.project.as_ref().expect("checked by the caller");
        let rows = project.scripts().into_iter().enumerate().map(|(i, path)| {
            let open = self.path.as_ref() == Some(&path);
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            div()
                .id(("script", i))
                .px_1()
                .rounded_sm()
                .when(open, |d| d.bg(theme::button_bg()).font_weight(FontWeight::SEMIBOLD))
                .hover(|s| s.bg(theme::border()))
                .cursor_pointer()
                .child(name)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.path.as_ref() != Some(&path) && this.confirm_discard() {
                        this.open_path(path.clone(), window);
                    }
                    cx.notify();
                }))
        });
        let catalog = vs_graph::components::project().len();
        let running = self.task_running();
        section(&format!("Project · {}", project.name))
            .child(div().text_size(px(11.0)).text_color(theme::dim()).child(project.root.display().to_string()))
            .children(rows)
            .child(small_button("script-add".into(), "+ New script").mt_1().on_click(cx.listener(
                |this, _, _, cx| {
                    this.new_script_dialog();
                    cx.notify();
                },
            )))
            .child(
                div()
                    .mt_2()
                    .text_size(px(11.0))
                    .text_color(theme::dim())
                    .child(format!("Catalog · {catalog} component(s) of the game, besides Bevy's common ones")),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        small_button("catalog-refresh".into(), "Refresh catalog")
                            .when(running, |b| b.opacity(0.5))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_catalog(cx);
                                cx.notify();
                            })),
                    )
                    .child(small_button("catalog-edit".into(), "Edit catalog").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.edit_catalog();
                            cx.notify();
                        },
                    ))),
            )
    }

    fn output_section(&self) -> impl IntoElement {
        let task = self.task.borrow();
        let task = task.as_ref().expect("checked by the caller");
        let state = match task.finished() {
            None => "running",
            Some(Outcome::Succeeded) => "done",
            Some(Outcome::Failed) => "failed",
            Some(Outcome::Stopped) => "stopped",
        };
        section(&format!("Output · {} · {state}", task.label)).child(
            div()
                .p_2()
                .rounded_md()
                .bg(theme::field_bg())
                .font_family(MONO)
                .text_size(px(11.0))
                .children(task.tail(14).into_iter().map(|line| {
                    let color = if line.trim_start().starts_with("error") {
                        theme::error()
                    } else if line.trim_start().starts_with("warning") {
                        theme::warning()
                    } else {
                        theme::dim()
                    };
                    div().text_color(color).child(if line.is_empty() { " ".to_string() } else { line })
                })),
        )
    }

    fn code_section(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        let code = match self.code_view {
            CodeView::Lua => compiled.lua.clone(),
            CodeView::Rust => self.rust_code(compiled),
        };
        let tab = |id: &'static str, label: &'static str, view: CodeView| {
            small_button(id.into(), label)
                .when(self.code_view == view, |b| b.bg(theme::accent()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.code_view = view;
                    cx.notify();
                }))
        };
        section("Generated code")
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(tab("view-lua", "Lua (preview)", CodeView::Lua))
                    .child(tab("view-rust", "Rust (build)", CodeView::Rust)),
            )
            .child(code_block(&code, if self.code_view == CodeView::Lua { "--" } else { "//" }))
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
                .child(div().text_color(theme::dim()).child("New projects are created in"))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            field("projects-dir".into(), self.config.projects_dir.display().to_string(), false)
                                .flex_1(),
                        )
                        .child(button("change-dir", "Change…").on_click(cx.listener(|this, _, _, cx| {
                            this.change_projects_dir();
                            cx.notify();
                        }))),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme::dim())
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
                    .child(field("dialog-name".into(), format!("{buffer}|"), true))
                    .child(div().text_size(px(11.0)).text_color(theme::dim()).child(hint))
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
                    .bg(theme::panel_bg())
                    .border_1()
                    .border_color(theme::border())
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .when(confirm != "Close", |d| {
                                d.child(button("dialog-cancel", "Cancel").on_click(cx.listener(|this, _, _, cx| {
                                    this.dialog = None;
                                    cx.notify();
                                })))
                            })
                            .child(button("dialog-ok", confirm).bg(theme::accent()).on_click(cx.listener(
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

    fn selection_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body: Div = match self.selected.iter().next() {
            Some(id) if self.selected.len() == 1 => {
                let node = self.graph.node(*id);
                match node.and_then(|n| library::find(&n.kind).map(|d| (n, d))) {
                    Some((node, def)) => {
                        let pins = self.graph.pins_of(node);
                        let pin_rows = pins
                            .inputs
                            .iter()
                            .map(|p| ("in", p))
                            .chain(pins.outputs.iter().map(|p| ("out", p)))
                            .map(|(dir, p)| {
                                div()
                                    .flex()
                                    .gap_2()
                                    .text_size(px(12.0))
                                    .child(div().w(px(28.0)).text_color(theme::dim()).child(dir))
                                    .child(div().w(px(10.0)).h(px(10.0)).mt(px(3.0)).rounded_full().bg(theme::pin(p.ty)))
                                    .child(p.name.clone())
                                    .child(div().text_color(theme::dim()).child(p.ty.label()))
                            });
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(match node.var.as_ref().or(node.field.as_ref()) {
                                        Some(subject) => format!("{} {subject} [{}]", def.title, node.id),
                                        None => format!("{} [{}]", def.title, node.id),
                                    }),
                            )
                            .child(div().text_color(theme::dim()).child(def.description))
                            .child({
                                let id = node.id;
                                let editing = self.editing.as_ref().filter(|e| e.target == EditTarget::NodeName(id));
                                let text = match (editing, &node.name) {
                                    (Some(e), _) => format!("{}|", e.buffer),
                                    (None, Some(name)) => name.clone(),
                                    (None, None) => "(unnamed)".into(),
                                };
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().w(px(40.0)).text_color(theme::dim()).child("Name"))
                                    .child(field("node-name".into(), text, editing.is_some()).flex_1().on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.start_edit(EditTarget::NodeName(id));
                                            cx.notify();
                                        }),
                                    ))
                            })
                            .child(div().h(px(4.0)))
                            .children(pin_rows)
                    }
                    None => div().text_color(theme::dim()).child("Unknown node"),
                }
            }
            Some(_) => div().text_color(theme::dim()).child(format!("{} nodes selected", self.selected.len())),
            None => div().text_color(theme::dim()).child("Select a node to see what it does."),
        };
        section("Node").child(body)
    }

    fn variables_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    field(
                        SharedString::from(format!("var-name-{i}")),
                        editing_name.map_or(var.name.clone(), |e| format!("{}|", e.buffer)),
                        editing_name.is_some(),
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
                        .w(px(40.0))
                        .px_1()
                        .rounded_sm()
                        .text_size(px(11.0))
                        .text_color(theme::pin(ty))
                        .border_1()
                        .border_color(theme::border())
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
                    )
                    .w(px(if wide { 104.0 } else { 64.0 }))
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
                .child(small_button(format!("var-get-{i}"), "Get").on_click(cx.listener(move |this, _, _, cx| {
                    this.add_variable_node(i, "var.get");
                    cx.notify();
                })))
                .child(small_button(format!("var-set-{i}"), "Set").on_click(cx.listener(move |this, _, _, cx| {
                    this.add_variable_node(i, "var.set");
                    cx.notify();
                })))
                .child(small_button(format!("var-del-{i}"), "×").on_click(cx.listener(move |this, _, _, cx| {
                    this.remove_variable(i);
                    cx.notify();
                })))
        });
        section("Variables").children(rows).child(
            small_button("var-add".into(), "+ Add variable")
                .mt_1()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.add_variable();
                    cx.notify();
                })),
        )
    }

    fn problems_section(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = compiled.problems.iter().enumerate().map(|(i, p)| {
            let color = if p.severity == Severity::Error { theme::error() } else { theme::warning() };
            let node = p.node;
            div()
                .id(("problem", i))
                .flex()
                .gap_2()
                .px_1()
                .rounded_sm()
                .hover(|s| s.bg(theme::border()))
                .cursor_pointer()
                .child(div().w(px(8.0)).h(px(8.0)).mt(px(5.0)).rounded_full().bg(color))
                .child(div().flex_1().child(p.message.clone()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(id) = node {
                        this.focus_node(id);
                    }
                    cx.notify();
                }))
        });
        section("Problems")
            .when(compiled.problems.is_empty(), |d| {
                d.child(div().text_color(theme::dim()).child("No problems."))
            })
            .children(rows)
    }
}

fn code_block(code: &str, comment: &'static str) -> Div {
    div()
        .p_2()
        .rounded_md()
        .bg(theme::field_bg())
        .font_family(MONO)
        .text_size(px(11.5))
        .children(code.lines().map(|line| {
            let color = if line.trim_start().starts_with(comment) { theme::dim() } else { theme::text() };
            div().text_color(color).child(if line.is_empty() {
                " ".to_string()
            } else {
                line.replace('\t', "    ")
            })
        }))
}

fn section(title: &str) -> Div {
    div()
        .p_3()
        .flex()
        .flex_col()
        .gap_1p5()
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::dim())
                .child(title.to_uppercase()),
        )
}

fn field(id: SharedString, text: String, active: bool) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .bg(theme::field_bg())
        .border_1()
        .border_color(if active { theme::accent() } else { theme::border() })
        .font_family(MONO)
        .text_size(px(12.0))
        .whitespace_nowrap()
        .overflow_hidden()
        .cursor_text()
        .child(text)
}

fn small_button(id: String, label: &str) -> gpui_kit::Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .bg(theme::button_bg())
        .hover(|s| s.bg(theme::border()))
        .text_size(px(11.0))
        .cursor_pointer()
        .child(label.to_string())
}

fn pin_dot(center: V2, ty: PinType, connected: bool, z: f32) -> Div {
    let r = layout::PIN_R * z;
    let color = theme::pin(ty);
    let dot = div()
        .absolute()
        .left(px(center.x - r))
        .top(px(center.y - r))
        .w(px(r * 2.0))
        .h(px(r * 2.0))
        .border(px(1.5 * z.max(0.7)))
        .border_color(color)
        .bg(if connected { color } else { theme::node_bg() });
    // Exec pins are square, data pins round.
    if ty == PinType::Exec { dot.rounded(px(1.5 * z)) } else { dot.rounded_full() }
}

fn paint_scene(scene: &Scene, bounds: Bounds<gpui_kit::Pixels>, window: &mut Window) {
    let o = V2::new(bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
    let w = bounds.size.width.as_f32();
    let h = bounds.size.height.as_f32();
    let quad = |x: f32, y: f32, qw: f32, qh: f32, color: Rgba, window: &mut Window| {
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
            let color = if i % 5 == 0 { theme::grid_major() } else { theme::grid_minor() };
            quad(o.x + x, o.y, 1.0, h, color, window);
        }
        for j in first_y..first_y + rows {
            let y = scene.pan.y + j as f32 * step;
            let color = if j % 5 == 0 { theme::grid_major() } else { theme::grid_minor() };
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
                rgba(0x4d8df722),
            )
            .border_widths(px(1.0))
            .border_color(theme::accent()),
        );
    }
}
