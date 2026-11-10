//! The modal dialogs (settings, new project, new script): how they look, drawn over the whole
//! window, and what they do with the keyboard and on confirm.


use gpui_kit::{
    Context, FontWeight, IntoElement, div, prelude::*, px, rgba,
};

use crate::editor::Editor;
use crate::panels::Dialog;
use gpui_kit::Window;
use crate::theme;
use crate::ui::{button, field};

/// The plugin a folder of scripts becomes: `player/input` → `InputPlugin`.
pub(crate) fn folder_plugin(folder: &str) -> String {
    vs_graph::rust::folder_plugin_name(folder.rsplit('/').next().unwrap_or(folder))
}

impl Editor {
    pub(crate) fn dialog_element(&self, compiled: &vs_graph::Compiled, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let dialog = self.chrome.dialog.as_ref().expect("checked by the caller");
        // The query commandbox has a layout of its own (see `query`).
        if let Dialog::Query { node, ran } = dialog {
            return self.query_command(*node, *ran, compiled, cx);
        }
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
                            field("projects-dir".into(), self.workspace.config.projects_dir.display().to_string(), false, cx)
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
            Dialog::Query { .. } => unreachable!("drawn above"),
            Dialog::NewProject { buffer } | Dialog::NewScript { buffer, .. } | Dialog::NewFolder { buffer, .. } => {
                let (title, hint) = match dialog {
                    Dialog::NewProject { .. } => (
                        "New project",
                        format!("It goes in {}", self.workspace.config.projects_dir.display()),
                    ),
                    Dialog::NewScript { folder, .. } if !folder.is_empty() => (
                        "New script",
                        format!("In {folder}/: a component you can put on entities, added by {}", folder_plugin(folder)),
                    ),
                    Dialog::NewScript { .. } => ("New script", "It becomes a component you can put on entities".to_string()),
                    Dialog::NewFolder { parent, .. } if !parent.is_empty() => {
                        ("New folder", format!("In {parent}/: a plugin of the scripts you put in it"))
                    }
                    _ => ("New folder", "It becomes a plugin of the scripts you put in it".to_string()),
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
                this.chrome.dialog = None;
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
                                    this.chrome.dialog = None;
                                    cx.notify();
                                })))
                            })
                            .child(button("dialog-ok", confirm, cx).bg(theme::primary(cx).0).text_color(theme::primary(cx).1).on_click(cx.listener(
                                |this, _, window, cx| {
                                    match this.chrome.dialog {
                                        Some(Dialog::Settings) => this.chrome.dialog = None,
                                        _ => this.confirm_dialog(window),
                                    }
                                    cx.notify();
                                },
                            ))),
                    ),
            )
            .into_any_element()
    }
}

impl Editor {
    pub fn open_settings(&mut self) {
        self.chrome.dialog = Some(Dialog::Settings);
    }

    /// Asks for a new projects folder and saves it.
    pub fn change_projects_dir(&mut self) {
        let mut picker = rfd::FileDialog::new().set_title("Folder for new projects");
        if self.workspace.config.projects_dir.exists() {
            picker = picker.set_directory(&self.workspace.config.projects_dir);
        }
        let Some(dir) = picker.pick_folder() else { return };
        self.workspace.config.projects_dir = dir;
        match self.workspace.config.save() {
            Ok(()) => self.chrome.status = format!("New projects go in {}", self.workspace.config.projects_dir.display()),
            Err(e) => self.chrome.status = format!("Couldn't save the settings: {e}"),
        }
    }

    pub fn new_project_dialog(&mut self) {
        self.chrome.dialog = Some(Dialog::NewProject { buffer: String::new() });
    }

    /// Asks for the name of a script to add in `folder` (`""` for the top).
    pub fn new_script_dialog(&mut self, folder: &str) {
        if self.workspace.project.is_some() {
            self.chrome.dialog = Some(Dialog::NewScript { buffer: String::new(), folder: folder.to_string() });
        }
    }

    /// Asks for the name of a folder to add in `parent` (`""` for the top).
    pub fn new_folder_dialog(&mut self, parent: &str) {
        if self.workspace.project.is_some() {
            self.chrome.dialog = Some(Dialog::NewFolder { buffer: String::new(), parent: parent.to_string() });
        }
    }

    pub fn dialog_key(&mut self, key: &str, typed: Option<String>, window: &mut Window) {
        match key {
            "escape" => self.chrome.dialog = None,
            "enter" => self.confirm_dialog(window),
            "backspace" => {
                if let Some(Dialog::NewProject { buffer } | Dialog::NewScript { buffer, .. } | Dialog::NewFolder { buffer, .. }) =
                    &mut self.chrome.dialog
                {
                    buffer.pop();
                }
            }
            _ => {
                if let (Some(t), Some(Dialog::NewProject { buffer } | Dialog::NewScript { buffer, .. } | Dialog::NewFolder { buffer, .. })) =
                    (typed, &mut self.chrome.dialog)
                {
                    buffer.push_str(&t);
                }
            }
        }
    }

    pub fn confirm_dialog(&mut self, window: &mut Window) {
        match self.chrome.dialog.take() {
            Some(Dialog::NewProject { buffer }) => {
                if !self.confirm_discard_all() {
                    return;
                }
                match vs_project::create(&self.workspace.config.projects_dir, &buffer) {
                    Ok(project) => {
                        self.enter_project(project, window);
                        self.chrome.status = format!("Created {}. Press Preview to run it", self.status_path());
                    }
                    Err(e) => {
                        self.chrome.status = format!("Couldn't create the project: {e}");
                        self.chrome.dialog = Some(Dialog::NewProject { buffer });
                    }
                }
            }
            Some(Dialog::NewScript { buffer, folder }) => {
                let Some(project) = self.workspace.project.clone() else { return };
                match project.new_script(&folder, &buffer) {
                    Ok(path) => {
                        self.open_path(path, window);
                        if let Some(stem) = self.doc.path.as_ref().and_then(|p| p.file_stem()) {
                            let ty = vs_graph::rust::names(&stem.to_string_lossy()).ty;
                            self.chrome.status = format!("Spawn `scripts::{ty}::default()` on an entity in src/main.rs to use it");
                        }
                    }
                    Err(e) => {
                        self.chrome.status = format!("Couldn't add the script: {e}");
                        self.chrome.dialog = Some(Dialog::NewScript { buffer, folder });
                    }
                }
            }
            Some(Dialog::NewFolder { buffer, parent }) => {
                let Some(project) = self.workspace.project.clone() else { return };
                match project.new_folder(&parent, &buffer) {
                    Ok(folder) => {
                        self.chrome.collapsed.remove(&parent);
                        self.chrome.status = format!("Added {folder}/. Scripts you add in it make up {}", folder_plugin(&folder));
                    }
                    Err(e) => {
                        self.chrome.status = format!("Couldn't add the folder: {e}");
                        self.chrome.dialog = Some(Dialog::NewFolder { buffer, parent });
                    }
                }
            }
            // Enter runs the query, like its big button.
            Some(Dialog::Query { node, .. }) => self.chrome.dialog = Some(Dialog::Query { node, ran: true }),
            Some(Dialog::Settings) | None => {}
        }
    }

    fn status_path(&self) -> String {
        self.workspace.project.as_ref().map(|p| p.root.display().to_string()).unwrap_or_default()
    }
}
