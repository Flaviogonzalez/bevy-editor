//! The add-node menu: a filter, then the matching nodes, scrolled to keep the highlight visible.


use gpui_kit::{
    Context, IntoElement, MouseButton, MouseDownEvent, ScrollEvent, div, prelude::*, px,
};

use crate::editor::Editor;
use crate::theme;
use super::MENU_ROWS;

impl Editor {
    pub(super) fn menu_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
}
