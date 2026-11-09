//! Small interface building blocks shared by the panels and the toolbar: buttons, text fields,
//! side panels and their groups, and code listings. They only read colors from `theme`, so they
//! know nothing about the editor's state, and each can be swapped for a gpui-component widget
//! without touching the panels that use it.


use gpui_kit::{
    App, Div, FontWeight, SharedString,
    Window, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::Sizable;
use gpui_kit::component::IconName;

use crate::actions;
use crate::theme;

pub const MONO: &str = "Consolas";

pub(crate) fn button(id: &'static str, label: &str, cx: &App) -> gpui_kit::Stateful<Div> {
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
pub(crate) fn icon_button(id: &'static str, icon: IconName, tooltip: &'static str, action: &dyn gpui_kit::Action) -> Button {
    let boxed = action.boxed_clone();
    Button::new(id)
        .ghost()
        .small()
        .tab_stop(false)
        .icon(icon)
        .tooltip_with_action(tooltip, action, Some(actions::CONTEXT))
        .on_click(move |_, window, cx| window.dispatch_action(boxed.boxed_clone(), cx))
}

pub(crate) fn code_block(code: &str, comment: &'static str, cx: &App) -> Div {
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
pub(crate) fn panel(id: &'static str, cx: &App) -> gpui_kit::Stateful<Div> {
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
pub(crate) fn panel_title(title: &str, cx: &App) -> Div {
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
pub(crate) fn group(title: impl Into<SharedString>, action: Option<gpui_kit::AnyElement>, cx: &App) -> Div {
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
pub(crate) fn dispatch<A: gpui_kit::Action>(action: A) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx)
}

pub(crate) fn field(id: SharedString, text: String, active: bool, cx: &App) -> gpui_kit::Stateful<Div> {
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
