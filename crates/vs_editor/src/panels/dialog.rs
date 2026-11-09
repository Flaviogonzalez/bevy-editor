//! The modal dialogs (settings, new project, catalog) drawn over the whole window.


use gpui_kit::{
    Context, FontWeight, IntoElement, div, prelude::*, px, rgba,
};

use crate::editor::{Dialog, Editor};
use crate::theme;
use crate::ui::{button, field};

impl Editor {
    pub(crate) fn dialog_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
