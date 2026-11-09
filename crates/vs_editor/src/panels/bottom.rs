//! The panel under the canvas, what the editor tells you back: the compiler's problems, the
//! output of the last preview or build, and the generated Lua and Rust code.


use gpui_kit::{
    App, Context, Div, IntoElement, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Selectable, Sizable};
use gpui_kit::component::{Icon, IconName};
use vs_graph::{Compiled, Severity};

use crate::actions;
use crate::editor::{BottomTab, CodeView, Editor};
use crate::workspace::tasks::Outcome;
use crate::theme;
use crate::ui::{MONO, code_block, icon_button};

impl Editor {
    /// The panel under the canvas: what the editor tells you back. The compiler's problems, the
    /// output of the last preview or build, and the generated code.
    pub(crate) fn bottom_panel(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
        const TABS: [BottomTab; 3] = [BottomTab::Problems, BottomTab::Output, BottomTab::Code];
        let selected = TABS.iter().position(|t| *t == self.chrome.bottom_tab).unwrap_or(0);
        let count = compiled.problems.len();
        let output = match self.workspace.task.borrow().as_ref().map(|t| t.finished()) {
            Some(None) => "Output · running",
            _ => "Output",
        };
        let code_toggle = |id: &'static str, label: &'static str, view: CodeView| {
            Button::new(id).ghost().xsmall().tab_stop(false).label(label).selected(self.chrome.code_view == view).on_click(
                cx.listener(move |this, _, _, cx| {
                    this.chrome.code_view = view;
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
                this.chrome.bottom_tab = TABS[*ix];
                cx.notify();
            }))
            .suffix(
                div()
                    .h_full()
                    .pr_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(self.chrome.bottom_tab == BottomTab::Code, |d| {
                        d.child(code_toggle("view-lua", "Lua (preview)", CodeView::Lua))
                            .child(code_toggle("view-rust", "Rust (build)", CodeView::Rust))
                    })
                    .child(icon_button("panel-hide", IconName::Close, "Hide panel", &actions::ToggleBottomPanel)),
            );
        let body = match self.chrome.bottom_tab {
            BottomTab::Problems => self.problems_list(compiled, cx).into_any_element(),
            BottomTab::Output => self.output_log(cx).into_any_element(),
            BottomTab::Code => {
                let code = match self.chrome.code_view {
                    CodeView::Lua => compiled.lua.clone(),
                    CodeView::Rust => self.rust_code(compiled),
                };
                code_block(&code, if self.chrome.code_view == CodeView::Lua { "--" } else { "//" }, cx).into_any_element()
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
        let task = self.workspace.task.borrow();
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
            .doc.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or("script".into(), |s| s.to_string_lossy().into_owned());
        let files = vs_graph::rust::emit_export(&[vs_graph::rust::Script { path: stem, ir: &compiled.module }]);
        files.into_iter().find(|(name, _)| name != "scripts/mod.rs").map(|(_, code)| code).unwrap_or_default()
    }
}
