//! The query commandbox: a floating box, like shadcn's Command, over the editor. It shows a
//! system's query as its two halves, each in its own row and color, the same color they have in
//! the Rust signature at the bottom:
//!
//! - what it reads and writes on each entity (the query's data, taken from the nodes);
//! - which entities it runs on (the query's filters), each one read as a phrase (`has ▾
//!   SpotLight ▾`) and editable;
//!
//! with a big button at the right that runs the query, and under it, read-only, the entities the
//! query matches.
//!
//! Input: the event node it's open for (`Dialog::Query`) and that event's `System`. Output: the
//! box, and edits to the node's filters through `Document`. The Inspector only has the button
//! that opens it ([`Editor::query_trigger`]).

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::{
    Anchor, AnyElement, App, Context, Div, FontWeight, Hsla, IntoElement, SharedString, Window, div, prelude::*, px, rgba,
};
use vs_graph::access::{Filter, Subject, System};
use vs_graph::{Compiled, FilterOp, NodeId};

use crate::editor::Editor;
use crate::panels::Dialog;
use crate::theme;
use crate::ui::MONO;

/// The color of the query's data, wherever it shows.
fn data_color() -> Hsla {
    theme::system(0).into()
}

/// The color of the query's filters, wherever they show.
fn filter_color() -> Hsla {
    theme::system(2).into()
}

/// A filter as the start of a phrase about the entity: "has", "doesn't have"…
fn phrase(op: FilterOp) -> &'static str {
    match op {
        FilterOp::With => "has",
        FilterOp::Without => "doesn't have",
        FilterOp::Added => "just got",
        FilterOp::Changed => "changed",
    }
}

/// A big button with a label and a chevron: a query is what a system is about, so the buttons
/// that open and run it aren't small icons.
fn big_button(id: &'static str, label: impl IntoElement, cx: &App) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .px_4()
        .min_h(px(44.0))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded_md()
        .bg(theme::selection(cx).opacity(0.22))
        .border_1()
        .border_color(theme::selection(cx))
        .hover(|s| s.bg(theme::selection(cx).opacity(0.4)))
        .cursor_pointer()
        .font_weight(FontWeight::SEMIBOLD)
        .child(label)
        .child(Icon::new(IconName::ChevronRight).small())
}

/// `Query<data, filters>` with each half in its color.
fn colored_query(system: &System, cx: &App) -> Div {
    let part = |text: String, color: Hsla| div().text_color(color).child(text);
    let mut out = div()
        .flex()
        .flex_wrap()
        .font_family(MONO)
        .text_size(px(11.5))
        .text_color(theme::dim(cx))
        .child("Query<")
        .child(part(system.data(), data_color()));
    if let Some(filters) = system.filter_type() {
        out = out.child(", ").child(part(filters, filter_color()));
    }
    out.child(">")
}

impl Editor {
    /// The Inspector's part: the button that opens the commandbox, showing the query it runs.
    pub(crate) fn query_trigger(&self, node: NodeId, system: &System, cx: &mut Context<Self>) -> Div {
        // Two short lines, in the colors the commandbox uses: what it reads and writes, then
        // which entities it runs on, as phrases.
        let data: Vec<String> = system
            .items
            .iter()
            .map(|i| if i.subject == Subject::Entity { "Entity".to_string() } else { i.name.clone() })
            .collect();
        let own = match system.filter {
            Some(Filter::Added) => Some(format!("{} {}", phrase(FilterOp::Added), system.own)),
            Some(Filter::With) => Some(format!("{} {}", phrase(FilterOp::With), system.own)),
            None => None,
        };
        let filters: Vec<String> =
            own.into_iter().chain(system.filters.iter().map(|f| format!("{} {}", phrase(f.op), f.component.name))).collect();
        let line = |dot: Hsla, text: String| {
            div()
                .flex()
                .items_start()
                .gap_1p5()
                .font_weight(FontWeight::NORMAL)
                .text_size(px(11.5))
                .child(div().mt(px(5.0)).w(px(6.0)).h(px(6.0)).flex_none().rounded_full().bg(dot))
                .child(div().flex_1().min_w_0().text_color(theme::text(cx)).child(text))
        };
        let label = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .child("Query")
            .child(line(data_color(), if data.is_empty() { "reads nothing".into() } else { data.join(", ") }))
            // With no filters, the data alone picks the entities: those that have all of it.
            .child(line(
                filter_color(),
                match (filters.is_empty(), data.iter().any(|d| d != "Entity")) {
                    (false, _) => filters.join(", "),
                    (true, true) => "has everything it reads & writes".into(),
                    (true, false) => "every entity".into(),
                },
            ));
        crate::ui::group("Query", None, cx).child(
            big_button("query-open", label, cx).py_2().justify_start().on_click(cx.listener(move |this, _, _, cx| {
                this.chrome.dialog = Some(Dialog::Query { node, ran: false });
                cx.notify();
            })),
        )
    }

    /// The commandbox for event `node`, drawn over the window. `ran` is whether its big button
    /// was pressed, so the list can say why it's empty.
    pub(crate) fn query_command(&self, node: NodeId, ran: bool, compiled: &Compiled, cx: &mut Context<Self>) -> AnyElement {
        let system = self.systems(compiled).remove(&node);
        let event = self.doc.graph.node(node);
        let body = match (event, &system) {
            (Some(event), Some(system)) => self.query_body(event, system, ran, cx),
            _ => div().p_4().text_color(theme::dim(cx)).child("This event isn't a system anymore."),
        };
        div()
            .id("query-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            .items_start()
            .pt(px(90.0))
            .bg(rgba(0x0000_0099))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.chrome.dialog = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("query-command")
                    .w(px(700.0))
                    .flex()
                    .flex_col()
                    .rounded_lg()
                    .bg(theme::panel_bg(cx))
                    .border_1()
                    .border_color(theme::border(cx))
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(body),
            )
            .into_any_element()
    }

    fn query_body(&self, event: &vs_graph::Node, system: &System, ran: bool, cx: &mut Context<Self>) -> Div {
        let node = event.id;
        let header = div()
            .px_4()
            .py_3()
            .flex()
            .items_start()
            .justify_between()
            .border_b_1()
            .border_color(theme::border(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(div().font_family(MONO).font_weight(FontWeight::SEMIBOLD).child(format!("fn {}", system.name)))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme::dim(cx))
                            .child("Its query: which entities it runs on, and what it reads and writes on each one."),
                    ),
            )
            .child(div().text_size(px(11.0)).text_color(theme::dim(cx)).child("Esc to close"));

        let data = self.data_row(system, cx);
        let filters = self.filter_row(event, system, cx);
        let run = big_button("query-run", "Query", cx).flex_none().on_click(cx.listener(move |this, _, _, cx| {
            this.chrome.dialog = Some(Dialog::Query { node, ran: true });
            cx.notify();
        }));
        let bar = div()
            .px_4()
            .py_3()
            .flex()
            .items_stretch()
            .gap_3()
            .border_b_1()
            .border_color(theme::border(cx))
            .child(div().flex_1().min_w_0().flex().flex_col().gap_3().child(data).child(filters))
            .child(div().flex().items_center().child(run));

        let empty = if ran {
            "No preview is running. Start Preview, and the entities this query matches show here."
        } else {
            "Press Query to list the entities this query matches."
        };
        let results = div()
            .min_h(px(150.0))
            .px_4()
            .py_3()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::dim(cx))
                    .child("MATCHING ENTITIES"),
            )
            .child(div().flex_1().flex().items_center().justify_center().text_color(theme::dim(cx)).child(empty));
        let footer = div()
            .px_4()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .border_t_1()
            .border_color(theme::border(cx))
            .bg(theme::field_bg(cx))
            .rounded_b_lg()
            .child(colored_query(system, cx));
        div().flex().flex_col().child(header).child(bar).child(results).child(footer)
    }

    /// A row of the bar: its name and what it means at the left, in `color`, and its pills.
    fn bar_row(label: &str, hint: &str, color: Hsla, pills: Vec<AnyElement>, cx: &App) -> Div {
        div()
            .flex()
            .items_start()
            .gap_3()
            .child(
                div()
                    .w(px(140.0))
                    .flex_none()
                    .pt(px(3.0))
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(color))
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(color).child(label.to_string())),
                    )
                    .child(div().pl(px(14.0)).text_size(px(11.0)).text_color(theme::dim(cx)).child(hint.to_string())),
            )
            .child(div().flex_1().min_w_0().flex().flex_wrap().items_center().gap_1p5().children(pills))
    }

    /// What the system reads and writes on each entity: one pill per query item, each with
    /// whether it's read or written. They follow from the nodes.
    fn data_row(&self, system: &System, cx: &App) -> Div {
        let color = data_color();
        let pills: Vec<AnyElement> = system
            .items
            .iter()
            .map(|item| {
                let (name, access, access_color) = match (item.subject, item.write) {
                    (Subject::Entity, _) => ("Entity".to_string(), "id", theme::dim(cx)),
                    (_, true) => (item.name.clone(), "write", Hsla::from(theme::system(1))),
                    (_, false) => (item.name.clone(), "read", color),
                };
                div()
                    .h(px(26.0))
                    .flex()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(color.opacity(0.45))
                    .bg(color.opacity(0.08))
                    .overflow_hidden()
                    .child(div().px_2().font_family(MONO).text_size(px(12.0)).child(name))
                    .child(
                        div()
                            .h_full()
                            .px_1p5()
                            .flex()
                            .items_center()
                            .bg(access_color.opacity(0.18))
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(access_color)
                            .child(access),
                    )
                    .into_any_element()
            })
            .collect();
        let pills = if pills.is_empty() {
            vec![div().pt(px(3.0)).text_color(theme::dim(cx)).child("Nothing: no node reads or writes a component.").into_any_element()]
        } else {
            pills
        };
        Self::bar_row("Reads & writes", "from the nodes", color, pills, cx)
    }

    /// Which entities the system runs on: the filter on the script's own component, then the
    /// event's own filters, each a phrase of two menus (`has ▾` `SpotLight ▾`) and a remove
    /// button, and the button that adds one.
    fn filter_row(&self, node: &vs_graph::Node, system: &System, cx: &mut Context<Self>) -> Div {
        let color = filter_color();
        let id = node.id;
        let editor = cx.entity().downgrade();
        // A click handler that edits the document through `f`.
        let run = move |f: Box<dyn Fn(&mut Editor)>| {
            let editor = editor.clone();
            move |_: &gpui_kit::ClickEvent, _: &mut Window, cx: &mut App| {
                editor
                    .update(cx, |e, cx| {
                        f(e);
                        cx.notify();
                    })
                    .ok();
            }
        };
        let shell = |strong: bool| {
            div()
                .h(px(26.0))
                .flex()
                .items_center()
                .rounded_md()
                .border_1()
                .border_color(color.opacity(if strong { 0.55 } else { 0.25 }))
                .bg(color.opacity(if strong { 0.08 } else { 0.03 }))
                .overflow_hidden()
        };

        let mut pills: Vec<AnyElement> = Vec::new();
        // Asking for a component as data also requires it: an entity without `Transform` never
        // matches `Query<&mut Transform>`.
        if system.items.iter().any(|i| i.subject != Subject::Entity) {
            pills.push(
                shell(false)
                    .child(div().px_2().text_color(theme::dim(cx)).child("has everything it reads & writes"))
                    .child(
                        div()
                            .h_full()
                            .px_1p5()
                            .flex()
                            .items_center()
                            .bg(theme::dim(cx).opacity(0.15))
                            .text_size(px(10.5))
                            .text_color(theme::dim(cx))
                            .child("implied"),
                    )
                    .into_any_element(),
            );
        }
        // The own component's filter: fixed until it's a regular component.
        let own = match system.filter {
            Some(Filter::Added) => Some(FilterOp::Added),
            Some(Filter::With) => Some(FilterOp::With),
            None => None,
        };
        if let Some(op) = own {
            pills.push(
                shell(false)
                    .child(div().pl_2().pr_1().text_color(theme::dim(cx)).child(phrase(op)))
                    .child(div().pr_2().font_family(MONO).text_size(px(12.0)).child(system.own.clone()))
                    .child(
                        div()
                            .h_full()
                            .px_1p5()
                            .flex()
                            .items_center()
                            .bg(theme::dim(cx).opacity(0.15))
                            .text_size(px(10.5))
                            .text_color(theme::dim(cx))
                            .child("always"),
                    )
                    .into_any_element(),
            );
        }

        let choices = filter_components(&self.doc.graph, &system.own);
        for (i, f) in node.filters.iter().enumerate() {
            let op_menu = {
                let (run, current, component) = (run.clone(), f.op, f.component.clone());
                Button::new(SharedString::from(format!("filter-op-{i}")))
                    .ghost()
                    .xsmall()
                    .tab_stop(false)
                    .label(phrase(f.op))
                    .dropdown_caret(true)
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |mut menu, _, _| {
                        for op in FilterOp::ALL {
                            let set = run(Box::new(move |e: &mut Editor| e.doc.set_filter(id, i, Some(op), None)));
                            let label = format!("{}    {}<{}>", phrase(op), op.rust(), component);
                            menu = menu.item(PopupMenuItem::new(label).checked(op == current).on_click(set));
                        }
                        menu
                    })
            };
            let component_menu = {
                let (run, choices) = (run.clone(), choices.clone());
                Button::new(SharedString::from(format!("filter-component-{i}")))
                    .ghost()
                    .xsmall()
                    .tab_stop(false)
                    .label(f.component.clone())
                    .dropdown_caret(true)
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
                        component_menu(menu, &choices, |name| {
                            run(Box::new(move |e: &mut Editor| e.doc.set_filter(id, i, None, Some(&name))))
                        })
                    })
            };
            let remove = Button::new(SharedString::from(format!("filter-remove-{i}")))
                .ghost()
                .xsmall()
                .tab_stop(false)
                .icon(IconName::Close)
                .tooltip("Remove this filter")
                .on_click(run(Box::new(move |e: &mut Editor| e.doc.remove_filter(id, i))));
            pills.push(
                shell(true)
                    .child(op_menu)
                    .child(div().w(px(1.0)).h(px(14.0)).bg(color.opacity(0.35)))
                    .child(component_menu)
                    .child(remove)
                    .into_any_element(),
            );
        }
        let add = Button::new("filter-add")
            .outline()
            .xsmall()
            .tab_stop(false)
            .icon(IconName::Plus)
            .label("Add filter")
            .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
                component_menu(menu, &choices, |name| run(Box::new(move |e: &mut Editor| e.doc.add_filter(id, &name))))
            });
        pills.push(add.into_any_element());
        Self::bar_row("Only entities that", "the filters", color, pills, cx)
    }
}

/// The components a query can filter on, as (heading, names): Bevy's, then the project's (its
/// Rust components and the ones its scripts declare). The script's own component is left out,
/// since its filter is always there.
fn filter_components(graph: &vs_graph::Graph, own: &str) -> Vec<(&'static str, Vec<String>)> {
    let mut bevy = Vec::new();
    let mut project: Vec<String> = graph.defined_components().map(|d| d.name.clone()).collect();
    for c in vs_graph::components::all() {
        if c.name == own || bevy.contains(&c.name) || project.contains(&c.name) {
            continue;
        }
        if vs_graph::components::is_bundled(&c.name) { bevy.push(c.name) } else { project.push(c.name) }
    }
    project.sort();
    vec![("Bevy", bevy), ("This project", project)]
}

/// A menu of `choices` under their headings, each item running `pick(name)`.
fn component_menu<H>(mut menu: PopupMenu, choices: &[(&'static str, Vec<String>)], pick: impl Fn(String) -> H) -> PopupMenu
where
    H: Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
{
    menu = menu.scrollable(true).max_h(px(320.0));
    for (heading, names) in choices.iter().filter(|(_, names)| !names.is_empty()) {
        menu = menu.label(*heading);
        for name in names {
            menu = menu.item(PopupMenuItem::new(name.clone()).on_click(pick(name.clone())));
        }
    }
    menu
}
