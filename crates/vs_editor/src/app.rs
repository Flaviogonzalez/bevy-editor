//! The editor's root view: lays out the toolbar, the resizable Explorer, canvas, bottom panel and
//! Inspector, and the dialog on top. Each region draws itself from its own module (`panels`,
//! `canvas`); this file only decides where they go.


use gpui_kit::{
    Context, IntoElement, KeyDownEvent,
    Window, div, prelude::*, px,
};
use gpui_kit::component::resizable::{h_resizable, resizable_panel, v_resizable};

use crate::actions;
use crate::editor::Editor;
use crate::theme;

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = window;
        let compiled = vs_graph::compile(&self.doc.graph);
        let layouts = self.doc.layouts();

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
                                            .visible(self.chrome.show_bottom_panel)
                                            .child(self.bottom_panel(&compiled, cx)),
                                    ),
                            ),
                        )
                        .child(
                            resizable_panel()
                                .size(px(300.0))
                                .size_range(px(220.0)..px(520.0))
                                .fixed()
                                .child(self.inspector(&compiled, cx)),
                        ),
                ),
            )
            .when(self.chrome.dialog.is_some(), |d| d.child(self.dialog_element(cx)))
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
                    .child(self.chrome.status.clone()),
            )
    }
}
