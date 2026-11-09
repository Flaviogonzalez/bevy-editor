//! The top bar: the menus, the most used commands as icon buttons, the run controls and the
//! file's status. Every control dispatches an action from actions.rs.


use gpui_kit::{
    Context, IntoElement, MouseButton, div, prelude::*, px,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu};
use gpui_kit::component::{Disableable, Selectable, Sizable};
use gpui_kit::component::IconName;
use vs_graph::Compiled;

use crate::actions;
use crate::editor::Editor;
use crate::theme;
use crate::ui::icon_button;

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
    pub(crate) fn toolbar(&self, compiled: &Compiled, cx: &mut Context<Self>) -> impl IntoElement {
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
}
