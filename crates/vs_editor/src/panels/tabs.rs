//! The tab bar over the canvas: one tab per open script, the edited one highlighted, each with
//! a dot while it has unsaved changes and a button that closes it.

use gpui_kit::{Context, Div, MouseButton, SharedString, div, prelude::*, px};
use gpui_kit::component::{Icon, IconName, Sizable};

use crate::editor::Editor;
use crate::theme;

impl Editor {
    pub(crate) fn tab_bar(&self, cx: &mut Context<Self>) -> Div {
        let active = self.tabs.active();
        let tabs = self.tabs.all(&self.doc).enumerate().map(|(i, doc)| {
            let current = i == active;
            let name = doc.path.as_ref().and_then(|p| p.file_stem()).map_or("untitled".into(), |s| s.to_string_lossy().into_owned());
            let close = div()
                .id(("tab-close", i))
                .rounded_sm()
                .p_0p5()
                .hover(|s| s.bg(theme::button_hover(cx)))
                .child(Icon::new(IconName::Close).xsmall())
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.close_tab(i, window);
                    cx.notify();
                }));
            div()
                .id(SharedString::from(format!("tab-{i}")))
                .h_full()
                .pl_3()
                .pr_1p5()
                .flex()
                .items_center()
                .gap_1p5()
                .border_r_1()
                .border_color(theme::border(cx))
                .text_color(if current { theme::text(cx) } else { theme::dim(cx) })
                .map(|d| if current { d.bg(theme::canvas_bg(cx)) } else { d.bg(theme::panel_bg(cx)) })
                // The edited tab is marked on top, where it meets the bar's edge.
                .when(current, |d| d.border_t_2().border_color(theme::selection(cx)))
                .hover(|s| s.text_color(theme::text(cx)))
                .cursor_pointer()
                .child(name)
                .when(doc.history.is_dirty(), |d| d.child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(theme::dim(cx))))
                .child(close)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.switch_tab(i, window);
                    cx.notify();
                }))
                // A middle click closes, as in most editors.
                .on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(move |this, _, window, cx| {
                        this.close_tab(i, window);
                        cx.notify();
                    }),
                )
        });
        div()
            .h(px(32.0))
            .flex_none()
            .flex()
            .bg(theme::panel_bg(cx))
            .border_b_1()
            .border_color(theme::border(cx))
            .overflow_hidden()
            .text_size(px(12.5))
            .children(tabs)
    }
}
