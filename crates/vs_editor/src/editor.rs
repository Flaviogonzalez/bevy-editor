//! The editor window's state, and what coordinates its parts: the text edit that can start on
//! the canvas or in a panel, replacing the open document, opening and saving files (which
//! touch the document, the project and the window title), and routing keys that aren't
//! actions. Each part's own logic lives with it: `document/`, `canvas/`, `panels/`,
//! `workspace/`.

use std::path::PathBuf;

use gpui_kit::{Context, FocusHandle, KeyDownEvent, Window};
use vs_graph::{NodeId, Value};

use crate::canvas::{CanvasState, Drag};
use crate::document::Document;
use crate::panels::Chrome;
use crate::workspace::Workspace;
use crate::canvas::layout::Hit;

#[derive(Clone, Debug, PartialEq)]
pub enum EditTarget {
    Field(NodeId, String),
    /// The node's optional name, used for its value or state in exported Rust.
    NodeName(NodeId),
    VarName(usize),
    VarValue(usize),
}

pub struct Editing {
    pub target: EditTarget,
    pub buffer: String,
}

/// The editor window's state, split by who owns it. Every panel and the canvas still render
/// from `&Editor`, but each part changes for its own reasons:
///
/// - `doc`: the open script and its undo history; later there will be one per open file.
/// - `canvas`: the pointer's interaction with the canvas (drags, the add-node menu, hover).
/// - `chrome`: the panels' own state (bottom tab, dialog, status line).
/// - `workspace`: the project, the settings and the cargo run, which outlive the open script.
pub struct Editor {
    pub doc: Document,
    pub canvas: CanvasState,
    pub chrome: Chrome,
    pub workspace: Workspace,
    /// The text edit that has the keyboard, in a node's field on the canvas or in a panel. It
    /// lives here, not in `canvas` or `chrome`, because either side can start one.
    pub editing: Option<Editing>,
    pub focus: FocusHandle,
}

impl Editor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            doc: Document::default(),
            canvas: CanvasState::default(),
            chrome: Chrome::default(),
            workspace: Workspace::default(),
            editing: None,
            focus,
        }
    }

    // History ----------------------------------------------------------------------------------

    /// Undoes the last change. A text edit in progress is dropped, since it may point at a node
    /// or variable the undo removed.
    pub fn undo(&mut self) {
        if self.doc.undo() {
            self.editing = None;
        }
    }

    pub fn redo(&mut self) {
        if self.doc.redo() {
            self.editing = None;
        }
    }

    // Inline editing ---------------------------------------------------------------------------

    pub fn commit_edit(&mut self) {
        let Some(edit) = self.editing.take() else { return };
        match edit.target {
            EditTarget::Field(node, pin) => {
                let current = self.doc.graph.node(node).and_then(|n| n.values.get(&pin)).cloned();
                let Some(value) = current.and_then(|c| c.parse_like(&edit.buffer)) else {
                    self.chrome.status = format!("`{}` isn't a valid value", edit.buffer);
                    return;
                };
                if self.doc.graph.node(node).and_then(|n| n.values.get(&pin)) != Some(&value) {
                    self.doc.checkpoint();
                    if let Some(n) = self.doc.graph.node_mut(node) {
                        n.values.insert(pin, value);
                    }
                }
            }
            EditTarget::NodeName(id) => {
                let name = vs_graph::lower::snake_case(edit.buffer.trim());
                let name = (!name.is_empty()).then_some(name);
                if self.doc.graph.node(id).is_some_and(|n| n.name != name) {
                    self.doc.checkpoint();
                    if let Some(n) = self.doc.graph.node_mut(id) {
                        n.name = name;
                    }
                }
            }
            EditTarget::VarName(i) => {
                let name = edit.buffer.trim().to_string();
                let Some(old) = self.doc.graph.variables.get(i).map(|v| v.name.clone()) else { return };
                if name == old {
                    return;
                }
                if name.is_empty() || self.doc.graph.variable(&name).is_some() {
                    self.chrome.status = format!("A variable needs a unique name, `{name}` isn't one");
                    return;
                }
                self.doc.checkpoint();
                self.doc.graph.variables[i].name = name.clone();
                for n in &mut self.doc.graph.nodes {
                    if n.var.as_deref() == Some(old.as_str()) {
                        n.var = Some(name.clone());
                    }
                }
            }
            EditTarget::VarValue(i) => {
                let Some(var) = self.doc.graph.variables.get(i) else { return };
                match var.value.parse_like(&edit.buffer) {
                    Some(value) if value != var.value => {
                        self.doc.checkpoint();
                        self.doc.graph.variables[i].value = value;
                    }
                    Some(_) => {}
                    None => self.chrome.status = format!("`{}` isn't a valid value", edit.buffer),
                }
            }
        }
    }

    pub fn start_edit(&mut self, target: EditTarget) {
        self.commit_edit();
        let buffer = match &target {
            EditTarget::Field(node, pin) => self
                .doc.graph
                .node(*node)
                .and_then(|n| n.values.get(pin))
                .map(Value::display)
                .unwrap_or_default(),
            EditTarget::NodeName(id) => self.doc.graph.node(*id).and_then(|n| n.name.clone()).unwrap_or_default(),
            EditTarget::VarName(i) => {
                self.doc.graph.variables.get(*i).map(|v| v.name.clone()).unwrap_or_default()
            }
            EditTarget::VarValue(i) => {
                self.doc.graph.variables.get(*i).map(|v| v.value.display()).unwrap_or_default()
            }
        };
        self.editing = Some(Editing { target, buffer });
    }

    // Files ------------------------------------------------------------------------------------

    pub fn confirm_discard(&self) -> bool {
        !self.doc.history.is_dirty()
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description("The current graph has unsaved changes. Discard them?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                == rfd::MessageDialogResult::Yes
    }

    pub fn new_graph(&mut self, window: &mut Window) {
        if !self.confirm_discard() {
            return;
        }
        self.reset(Document::default());
        self.update_title(window);
    }

    /// Replaces the open document with `doc` and clears everything that pointed into the old
    /// one: the drag, the menu, the hover, the text edit, the dialog and the status line. The
    /// workspace, the panels' layout and the canvas bounds stay.
    pub fn reset(&mut self, doc: Document) {
        self.doc = doc;
        self.canvas.drag = Drag::None;
        self.canvas.menu = None;
        self.canvas.hover = Hit::Empty;
        self.editing = None;
        self.chrome.status = String::new();
        self.chrome.dialog = None;
    }

    pub fn open(&mut self, window: &mut Window) {
        if !self.confirm_discard() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter("Visual script", &["lua"]).set_title("Open visual script");
        if let Some(project) = &self.workspace.project {
            dialog = dialog.set_directory(project.scripts_dir());
        }
        if let Some(path) = dialog.pick_file() {
            self.open_path(path, window);
        }
    }

    /// Opens a script without asking about unsaved changes. The project it belongs to, if any,
    /// becomes the current project.
    pub fn open_path(&mut self, path: PathBuf, window: &mut Window) {
        match Document::load(&path) {
            Ok(doc) => {
                let project = vs_project::Project::containing(&path).or_else(|| self.workspace.project.clone());
                self.reset(doc);
                // Always reload, even for the same project: a preview may have rewritten its catalog.
                let catalog = match project {
                    Some(project) => self.set_project(project),
                    None => Ok(()),
                };
                self.doc.frame(self.canvas.size());
                self.chrome.status = match catalog {
                    Ok(()) => format!("Opened {}", path.display()),
                    Err(e) => format!("Opened {}, but {e}", path.display()),
                };
                self.update_title(window);
            }
            Err(e) => self.chrome.status = format!("Couldn't open {}: {e}", path.display()),
        }
    }

    pub fn save(&mut self, window: &mut Window) {
        match self.doc.path.clone() {
            Some(path) => self.write(path, window),
            None => self.save_as(window),
        }
    }

    pub fn save_as(&mut self, window: &mut Window) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Visual script", &["lua"])
            .set_title("Save visual script")
            .set_file_name("script.lua")
            .save_file()
        else {
            return;
        };
        self.write(path, window);
    }

    fn write(&mut self, path: PathBuf, window: &mut Window) {
        self.commit_edit();
        match self.doc.write_to(&path) {
            Ok(()) => {
                self.chrome.status = format!("Saved {}", path.display());
                self.update_title(window);
            }
            Err(e) => self.chrome.status = format!("Couldn't save {}: {e}", path.display()),
        }
    }

    pub fn update_title(&self, window: &mut Window) {
        let project = self.workspace.project.as_ref().map(|p| format!(" ({})", p.name)).unwrap_or_default();
        window.set_window_title(&format!("{}{project} - Bevy Visual Script Editor", self.doc.file_name()));
    }

    // Keyboard ---------------------------------------------------------------------------------

    pub fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window) {
        let ks = &event.keystroke;
        let ctrl = ks.modifiers.control || ks.modifiers.platform;
        let key = ks.key.as_str();
        let typed = ks.key_char.clone().filter(|_| !ctrl);

        if self.chrome.dialog.is_some() {
            self.dialog_key(key, typed, window);
            return;
        }

        if let Some(edit) = &mut self.editing {
            match key {
                "enter" => self.commit_edit(),
                "escape" => self.editing = None,
                "backspace" => {
                    edit.buffer.pop();
                }
                _ => {
                    if let Some(t) = typed {
                        edit.buffer.push_str(&t);
                    }
                }
            }
            return;
        }

        if self.canvas.menu.is_some() {
            self.menu_key(key, typed);
            return;
        }

        // Shortcuts with a menu item are actions (see actions.rs); these are the extra keys.
        match (ctrl, key) {
            (false, "backspace") => self.doc.delete_selected(),
            (false, "space") => self.open_menu(self.canvas.mouse, None),
            _ => {}
        }
    }
}
