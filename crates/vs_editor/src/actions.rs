//! The editor's commands as GPUI actions, so the menu bar and the keyboard run the same code.
//!
//! Each action has at most one key binding here, and the menus read their shortcut hints from
//! these bindings, so the two can't disagree. [`register`] wires every action to the `Editor`
//! method that does the work.
//!
//! Keys still go through `Editor::key_down` first in spirit: while a dialog, a text edit or the
//! add-node menu has the keyboard, an action gives its keystroke back (`cx.propagate()`) and
//! `key_down` handles it as typing or navigation, exactly as before actions existed.

use gpui_kit::{App, Context, InteractiveElement, KeyBinding, Window, actions};

use crate::editor::Editor;

/// The key context the editor's root element declares; bindings only apply inside it.
pub const CONTEXT: &str = "Editor";

actions!(
    editor,
    [
        NewScript,
        OpenScript,
        Save,
        SaveAs,
        NewProject,
        OpenProject,
        Settings,
        Undo,
        Redo,
        Duplicate,
        Delete,
        SelectAll,
        AddNode,
        FrameAll,
        ToggleBottomPanel,
        Preview,
        Build,
        Stop,
        RefreshCatalog,
        EditCatalog,
    ]
);

/// A keystroke with the platform's command modifier: Cmd on macOS, Ctrl elsewhere. (GPUI's own
/// `secondary-` needs the OS, which `KeyBinding::new` doesn't pass, so it fails to parse there.)
macro_rules! cmd {
    ($keys:literal) => {
        if cfg!(target_os = "macos") { concat!("cmd-", $keys) } else { concat!("ctrl-", $keys) }
    };
}

pub fn bind_keys(cx: &mut App) {
    let c = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new(cmd!("n"), NewScript, c),
        KeyBinding::new(cmd!("o"), OpenScript, c),
        KeyBinding::new(cmd!("s"), Save, c),
        KeyBinding::new(cmd!("shift-s"), SaveAs, c),
        KeyBinding::new(cmd!("z"), Undo, c),
        KeyBinding::new(cmd!("shift-z"), Redo, c),
        KeyBinding::new(cmd!("y"), Redo, c),
        KeyBinding::new(cmd!("d"), Duplicate, c),
        KeyBinding::new("delete", Delete, c),
        KeyBinding::new(cmd!("a"), SelectAll, c),
        KeyBinding::new("tab", AddNode, c),
        KeyBinding::new("f", FrameAll, c),
        KeyBinding::new(cmd!("j"), ToggleBottomPanel, c),
        KeyBinding::new("f5", Preview, c),
        KeyBinding::new(cmd!("b"), Build, c),
        KeyBinding::new("shift-f5", Stop, c),
    ]);
}

impl Editor {
    /// Whether something other than the canvas owns the keyboard right now.
    fn keyboard_taken(&self) -> bool {
        self.chrome.dialog.is_some() || self.editing.is_some() || self.canvas.menu.is_some()
    }
}

/// Wraps an editor method as an action handler that steps aside while the keyboard is taken.
fn on<A: gpui_kit::Action, E: InteractiveElement>(
    el: E,
    cx: &mut Context<Editor>,
    run: impl Fn(&mut Editor, &mut Window, &mut Context<Editor>) + 'static,
) -> E {
    el.on_action(cx.listener(move |this, _: &A, window, cx| {
        if this.keyboard_taken() {
            cx.propagate();
            return;
        }
        run(this, window, cx);
        cx.notify();
    }))
}

/// Connects every action to the editor. Call on the editor's root element.
pub fn register<E: InteractiveElement>(el: E, cx: &mut Context<Editor>) -> E {
    let el = on::<NewScript, _>(el, cx, |e, w, _| e.new_graph(w));
    let el = on::<OpenScript, _>(el, cx, |e, w, _| e.open(w));
    let el = on::<Save, _>(el, cx, |e, w, _| e.save(w));
    let el = on::<SaveAs, _>(el, cx, |e, w, _| e.save_as(w));
    let el = on::<NewProject, _>(el, cx, |e, _, _| e.new_project_dialog());
    let el = on::<OpenProject, _>(el, cx, |e, w, _| e.open_project(w));
    let el = on::<Settings, _>(el, cx, |e, _, _| e.open_settings());
    let el = on::<Undo, _>(el, cx, |e, _, _| e.undo());
    let el = on::<Redo, _>(el, cx, |e, _, _| e.redo());
    let el = on::<Duplicate, _>(el, cx, |e, _, _| e.duplicate_selected());
    let el = on::<Delete, _>(el, cx, |e, _, _| e.delete_selected());
    let el = on::<SelectAll, _>(el, cx, |e, _, _| e.doc.selected = e.doc.graph.nodes.iter().map(|n| n.id).collect());
    let el = on::<AddNode, _>(el, cx, |e, _, _| e.open_menu(e.canvas.mouse, None));
    let el = on::<FrameAll, _>(el, cx, |e, _, _| e.frame());
    let el = on::<ToggleBottomPanel, _>(el, cx, |e, _, _| e.chrome.show_bottom_panel = !e.chrome.show_bottom_panel);
    // The project commands do nothing without a project, as their menu items are disabled then.
    let el = on::<Preview, _>(el, cx, |e, w, cx| {
        if e.workspace.project.is_some() {
            e.run_preview(w, cx)
        }
    });
    let el = on::<Build, _>(el, cx, |e, w, cx| {
        if e.workspace.project.is_some() {
            e.build(w, cx)
        }
    });
    let el = on::<Stop, _>(el, cx, |e, _, _| e.stop_task());
    let el = on::<RefreshCatalog, _>(el, cx, |e, _, cx| {
        if e.workspace.project.is_some() {
            e.refresh_catalog(cx)
        }
    });
    on::<EditCatalog, _>(el, cx, |e, _, _| {
        if e.workspace.project.is_some() {
            e.edit_catalog()
        }
    })
}
