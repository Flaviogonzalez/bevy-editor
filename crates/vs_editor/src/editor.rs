//! Editor state and everything that changes it. Drawing lives in `render.rs`.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{Context, FocusHandle, KeyDownEvent, Window};
use vs_graph::{Graph, NodeId, PinRef, PinType, Side, Value, Variable, library};

use crate::layout::{self, Hit, NodeLayout, Rect, V2};
use crate::tasks::Task;

pub const MIN_ZOOM: f32 = 0.3;
pub const MAX_ZOOM: f32 = 2.0;

/// What the left or right mouse button is doing right now.
pub enum Drag {
    None,
    Pan { last: V2, moved: bool, opens_menu: bool },
    Nodes { last: V2, before: Graph, moved: bool },
    /// A wire being pulled from `anchor`. `side` is the side of the anchor pin.
    Wire { anchor: PinRef, side: Side, cursor: V2 },
    Select { start: V2, cursor: V2, additive: bool },
}

/// One row in the add-node menu.
#[derive(Clone, Debug)]
pub struct MenuEntry {
    pub kind: &'static str,
    pub var: Option<String>,
    /// `Component.path`, for Get / Set field nodes.
    pub field: Option<String>,
    pub label: String,
    pub category: &'static str,
}

pub struct Menu {
    /// Canvas-local position of the menu's corner.
    pub at: V2,
    /// Where the new node goes, in graph units.
    pub graph_at: V2,
    pub filter: String,
    pub highlight: usize,
    /// The wire dropped on empty canvas that opened the menu, if any.
    pub wire: Option<(PinRef, Side)>,
}

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

/// A modal dialog over the editor.
pub enum Dialog {
    Settings,
    /// Asks for a name; `buffer` is what's typed so far.
    NewProject { buffer: String },
    NewScript { buffer: String },
}

/// Which generated code the side panel shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CodeView {
    Lua,
    Rust,
}

pub struct Editor {
    pub graph: Graph,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub pan: V2,
    pub zoom: f32,
    pub selected: BTreeSet<NodeId>,
    pub drag: Drag,
    pub menu: Option<Menu>,
    pub editing: Option<Editing>,
    pub undo: Vec<Graph>,
    pub redo: Vec<Graph>,
    pub focus: FocusHandle,
    pub show_code: bool,
    pub status: String,
    /// Last pointer position, canvas-local.
    pub mouse: V2,
    /// The canvas origin in window coordinates and its size, written while painting.
    pub canvas: Rc<Cell<(V2, V2)>>,
    pub config: vs_project::Config,
    /// The project the open script belongs to, if any.
    pub project: Option<vs_project::Project>,
    pub dialog: Option<Dialog>,
    pub code_view: CodeView,
    /// The cargo run in progress or last finished (preview or build). Shared across resets so
    /// opening another script doesn't stop it.
    pub task: Rc<std::cell::RefCell<Option<Task>>>,
}

impl Editor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            graph: starter_graph(),
            path: None,
            dirty: false,
            pan: V2::new(60.0, 60.0),
            zoom: 1.0,
            selected: BTreeSet::new(),
            drag: Drag::None,
            menu: None,
            editing: None,
            undo: Vec::new(),
            redo: Vec::new(),
            focus,
            show_code: true,
            status: "Right-click or press Tab on the canvas to add a node".into(),
            mouse: V2::default(),
            canvas: Rc::new(Cell::new((V2::default(), V2::new(800.0, 600.0)))),
            config: vs_project::Config::load(),
            project: None,
            dialog: None,
            code_view: CodeView::Lua,
            task: Rc::new(std::cell::RefCell::new(None)),
        }
    }

    // Coordinates ------------------------------------------------------------------------------

    pub fn to_local(&self, window_pos: gpui::Point<gpui::Pixels>) -> V2 {
        let origin = self.canvas.get().0;
        V2::new(window_pos.x.as_f32() - origin.x, window_pos.y.as_f32() - origin.y)
    }

    pub fn to_graph(&self, local: V2) -> V2 {
        (local - self.pan) * (1.0 / self.zoom)
    }

    pub fn to_screen(&self, graph: V2) -> V2 {
        graph * self.zoom + self.pan
    }

    pub fn layouts(&self) -> Vec<NodeLayout> {
        self.graph.nodes.iter().map(|n| layout::layout(&self.graph, n)).collect()
    }

    // History ----------------------------------------------------------------------------------

    /// Call before every change to the graph.
    pub fn checkpoint(&mut self) {
        self.undo.push(self.graph.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.graph, prev));
            self.after_history();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.graph, next));
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.selected.retain(|id| self.graph.node(*id).is_some());
        self.editing = None;
        self.dirty = true;
    }

    // Mouse ------------------------------------------------------------------------------------

    pub fn left_down(&mut self, local: V2, additive: bool) {
        self.commit_edit();
        self.menu = None;
        let g = self.to_graph(local);
        let layouts = self.layouts();
        match layout::hit_test(&layouts, g) {
            Hit::Pin(pin, side) => {
                // Grabbing a connected input picks its wire back up from the source.
                let existing = match side {
                    Side::Input => self.graph.links_to(&pin).next().cloned(),
                    Side::Output => None,
                };
                if let Some(link) = existing {
                    self.checkpoint();
                    self.graph.links.retain(|l| l != &link);
                    self.drag = Drag::Wire { anchor: link.from, side: Side::Output, cursor: g };
                } else {
                    self.drag = Drag::Wire { anchor: pin, side, cursor: g };
                }
            }
            Hit::Field(node, pin) => {
                let Some(value) = self.graph.node(node).and_then(|n| n.values.get(&pin)).cloned()
                else {
                    return;
                };
                if let Value::Bool(b) = value {
                    self.checkpoint();
                    if let Some(n) = self.graph.node_mut(node) {
                        n.values.insert(pin, Value::Bool(!b));
                    }
                } else {
                    self.editing = Some(Editing {
                        target: EditTarget::Field(node, pin),
                        buffer: value.display(),
                    });
                }
            }
            Hit::Header(id) | Hit::Body(id) => {
                if additive {
                    if !self.selected.remove(&id) {
                        self.selected.insert(id);
                    }
                } else if !self.selected.contains(&id) {
                    self.selected.clear();
                    self.selected.insert(id);
                }
                self.bring_to_front(id);
                self.drag = Drag::Nodes { last: g, before: self.graph.clone(), moved: false };
            }
            Hit::Empty => {
                if !additive {
                    self.selected.clear();
                }
                self.drag = Drag::Select { start: g, cursor: g, additive };
            }
        }
    }

    pub fn right_down(&mut self, local: V2) {
        self.commit_edit();
        self.menu = None;
        let g = self.to_graph(local);
        let layouts = self.layouts();
        let on_node = match layout::hit_test(&layouts, g) {
            Hit::Header(id) | Hit::Body(id) | Hit::Field(id, _) => Some(id),
            Hit::Pin(..) | Hit::Empty => None,
        };
        if let Some(id) = on_node
            && !self.selected.contains(&id)
        {
            self.selected.clear();
            self.selected.insert(id);
        }
        self.drag = Drag::Pan { last: local, moved: false, opens_menu: on_node.is_none() };
    }

    pub fn middle_down(&mut self, local: V2) {
        self.drag = Drag::Pan { last: local, moved: false, opens_menu: false };
    }

    pub fn mouse_move(&mut self, local: V2, any_button_held: bool) -> bool {
        self.mouse = local;
        if !any_button_held && !matches!(self.drag, Drag::None) {
            // The button went up somewhere we didn't see, such as outside the window.
            self.drag = Drag::None;
            return true;
        }
        let g = self.to_graph(local);
        match &mut self.drag {
            Drag::None => return false,
            Drag::Pan { last, moved, .. } => {
                let delta = local - *last;
                if delta.x.abs() + delta.y.abs() > 0.0 {
                    *moved = true;
                }
                *last = local;
                self.pan = self.pan + delta;
            }
            Drag::Nodes { last, moved, .. } => {
                let delta = g - *last;
                *last = g;
                *moved = true;
                let selected = self.selected.clone();
                for node in self.graph.nodes.iter_mut().filter(|n| selected.contains(&n.id)) {
                    node.x += delta.x;
                    node.y += delta.y;
                }
            }
            Drag::Wire { cursor, .. } | Drag::Select { cursor, .. } => *cursor = g,
        }
        true
    }

    pub fn mouse_up(&mut self, local: V2) {
        let g = self.to_graph(local);
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::None => {}
            Drag::Pan { moved, opens_menu, .. } => {
                if !moved && opens_menu {
                    self.open_menu(local, None);
                }
            }
            Drag::Nodes { before, moved, .. } => {
                if moved {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                }
            }
            Drag::Wire { anchor, side, .. } => {
                let layouts = self.layouts();
                match layout::hit_test(&layouts, g) {
                    Hit::Pin(target, target_side) if target_side != side => {
                        let (from, to) = match side {
                            Side::Output => (anchor, target),
                            Side::Input => (target, anchor),
                        };
                        if self.graph.can_link(&from, &to) {
                            self.checkpoint();
                            self.graph.link(from, to);
                        } else {
                            self.status = "Those pins don't fit together".into();
                        }
                    }
                    Hit::Empty => self.open_menu(local, Some((anchor, side))),
                    _ => {}
                }
            }
            Drag::Select { start, cursor, additive } => {
                let area = Rect::from_points(start, cursor);
                if area.w() > 2.0 || area.h() > 2.0 {
                    if !additive {
                        self.selected.clear();
                    }
                    for l in self.layouts() {
                        if l.rect.intersects(&area) {
                            self.selected.insert(l.id);
                        }
                    }
                }
            }
        }
    }

    pub fn scroll(&mut self, local: V2, dy: f32) {
        let before = self.to_graph(local);
        self.zoom = (self.zoom * 1.0015f32.powf(dy)).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = local - before * self.zoom;
    }

    fn bring_to_front(&mut self, id: NodeId) {
        if let Some(i) = self.graph.nodes.iter().position(|n| n.id == id) {
            let node = self.graph.nodes.remove(i);
            self.graph.nodes.push(node);
        }
    }

    // Add-node menu ----------------------------------------------------------------------------

    pub fn open_menu(&mut self, local: V2, wire: Option<(PinRef, Side)>) {
        // A preview run may have taught the project new components since.
        if let Some(project) = &self.project {
            let _ = vs_graph::components::load_project(&project.root);
        }
        let (_, size) = self.canvas.get();
        let at = V2::new(local.x.min(size.x - 270.0).max(4.0), local.y.min(size.y - 420.0).max(4.0));
        self.menu =
            Some(Menu { at, graph_at: self.to_graph(local), filter: String::new(), highlight: 0, wire });
    }

    /// The menu rows that match the filter and, when a wire opened the menu, can take it.
    pub fn menu_entries(&self) -> Vec<MenuEntry> {
        let Some(menu) = &self.menu else { return Vec::new() };
        let filter = menu.filter.to_lowercase();
        let mut entries = Vec::new();
        for def in library::all() {
            if def.is_variable() {
                for var in &self.graph.variables {
                    entries.push(MenuEntry {
                        kind: def.kind,
                        var: Some(var.name.clone()),
                        field: None,
                        label: format!("{} {}", def.title, var.name),
                        category: def.category,
                    });
                }
            } else if def.is_field() {
                for component in vs_graph::components::all() {
                    for f in &component.fields {
                        let field = format!("{}.{}", component.name, f.path);
                        entries.push(MenuEntry {
                            kind: def.kind,
                            var: None,
                            label: format!("{} {field}", def.title),
                            field: Some(field),
                            category: def.category,
                        });
                    }
                }
            } else {
                entries.push(MenuEntry {
                    kind: def.kind,
                    var: None,
                    field: None,
                    label: def.title.to_string(),
                    category: def.category,
                });
            }
        }
        entries.retain(|e| {
            (filter.is_empty()
                || e.label.to_lowercase().contains(&filter)
                || e.category.to_lowercase().contains(&filter))
                && menu.wire.as_ref().is_none_or(|(pin, side)| self.fits(e, pin, *side).is_some())
        });
        entries
    }

    /// The pin on a would-be node of `entry` that a wire from `pin` would plug into.
    fn fits(&self, entry: &MenuEntry, pin: &PinRef, side: Side) -> Option<String> {
        let ty = self.graph.pin(pin, side)?.ty;
        let probe = vs_graph::Node {
            id: 0,
            kind: entry.kind.to_string(),
            x: 0.0,
            y: 0.0,
            values: Default::default(),
            var: entry.var.clone(),
            field: entry.field.clone(),
            name: None,
        };
        let pins = self.graph.pins_of(&probe);
        let candidates = match side {
            Side::Output => pins.inputs,
            Side::Input => pins.outputs,
        };
        candidates
            .into_iter()
            .find(|p| match side {
                Side::Output => ty.connects_to(p.ty),
                Side::Input => p.ty.connects_to(ty),
            })
            .map(|p| p.name)
    }

    pub fn place(&mut self, entry: &MenuEntry) {
        let Some(menu) = self.menu.take() else { return };
        self.checkpoint();
        let mut at = menu.graph_at;
        if let Some((_, Side::Input)) = &menu.wire {
            // Dragged out of an input: the new node sits to the left so its output faces it.
            at.x -= layout::NODE_W;
        }
        let id = match &entry.field {
            Some(field) => self.graph.add_field_node(entry.kind, field, at.x, at.y),
            None => self.graph.add_node(entry.kind, entry.var.clone(), at.x, at.y),
        };
        if let Some((pin, side)) = menu.wire
            && let Some(target) = self.fits(entry, &pin, side)
        {
            let new_pin = PinRef::new(id, target);
            match side {
                Side::Output => self.graph.link(pin, new_pin),
                Side::Input => self.graph.link(new_pin, pin),
            };
        }
        self.selected.clear();
        self.selected.insert(id);
    }

    // Inline editing ---------------------------------------------------------------------------

    pub fn commit_edit(&mut self) {
        let Some(edit) = self.editing.take() else { return };
        match edit.target {
            EditTarget::Field(node, pin) => {
                let current = self.graph.node(node).and_then(|n| n.values.get(&pin)).cloned();
                let Some(value) = current.and_then(|c| c.parse_like(&edit.buffer)) else {
                    self.status = format!("`{}` isn't a valid value", edit.buffer);
                    return;
                };
                if self.graph.node(node).and_then(|n| n.values.get(&pin)) != Some(&value) {
                    self.checkpoint();
                    if let Some(n) = self.graph.node_mut(node) {
                        n.values.insert(pin, value);
                    }
                }
            }
            EditTarget::NodeName(id) => {
                let name = vs_graph::lower::snake_case(edit.buffer.trim());
                let name = (!name.is_empty()).then_some(name);
                if self.graph.node(id).is_some_and(|n| n.name != name) {
                    self.checkpoint();
                    if let Some(n) = self.graph.node_mut(id) {
                        n.name = name;
                    }
                }
            }
            EditTarget::VarName(i) => {
                let name = edit.buffer.trim().to_string();
                let Some(old) = self.graph.variables.get(i).map(|v| v.name.clone()) else { return };
                if name == old {
                    return;
                }
                if name.is_empty() || self.graph.variable(&name).is_some() {
                    self.status = format!("A variable needs a unique name, `{name}` isn't one");
                    return;
                }
                self.checkpoint();
                self.graph.variables[i].name = name.clone();
                for n in &mut self.graph.nodes {
                    if n.var.as_deref() == Some(old.as_str()) {
                        n.var = Some(name.clone());
                    }
                }
            }
            EditTarget::VarValue(i) => {
                let Some(var) = self.graph.variables.get(i) else { return };
                match var.value.parse_like(&edit.buffer) {
                    Some(value) if value != var.value => {
                        self.checkpoint();
                        self.graph.variables[i].value = value;
                    }
                    Some(_) => {}
                    None => self.status = format!("`{}` isn't a valid value", edit.buffer),
                }
            }
        }
    }

    pub fn start_edit(&mut self, target: EditTarget) {
        self.commit_edit();
        let buffer = match &target {
            EditTarget::Field(node, pin) => self
                .graph
                .node(*node)
                .and_then(|n| n.values.get(pin))
                .map(Value::display)
                .unwrap_or_default(),
            EditTarget::NodeName(id) => self.graph.node(*id).and_then(|n| n.name.clone()).unwrap_or_default(),
            EditTarget::VarName(i) => {
                self.graph.variables.get(*i).map(|v| v.name.clone()).unwrap_or_default()
            }
            EditTarget::VarValue(i) => {
                self.graph.variables.get(*i).map(|v| v.value.display()).unwrap_or_default()
            }
        };
        self.editing = Some(Editing { target, buffer });
    }

    // Variables --------------------------------------------------------------------------------

    pub fn add_variable(&mut self) {
        self.checkpoint();
        let mut n = self.graph.variables.len() + 1;
        while self.graph.variable(&format!("var{n}")).is_some() {
            n += 1;
        }
        self.graph.variables.push(Variable { name: format!("var{n}"), value: Value::Number(0.0) });
    }

    pub fn remove_variable(&mut self, i: usize) {
        let Some(name) = self.graph.variables.get(i).map(|v| v.name.clone()) else { return };
        self.checkpoint();
        self.graph.variables.remove(i);
        let ids: Vec<NodeId> = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.var.as_deref() == Some(name.as_str()))
            .map(|n| n.id)
            .collect();
        self.graph.remove_nodes(&ids);
        self.editing = None;
    }

    /// Number → Bool → String → Vector → Rotation → Color → Number. Set nodes get a fresh inline
    /// value of the new type.
    pub fn cycle_variable_type(&mut self, i: usize) {
        let Some(var) = self.graph.variables.get(i) else { return };
        let ty = match var.value.ty() {
            PinType::Number => PinType::Bool,
            PinType::Bool => PinType::String,
            PinType::String => PinType::Vec3,
            PinType::Vec3 => PinType::Quat,
            PinType::Quat => PinType::Color,
            _ => PinType::Number,
        };
        let value = Value::default_for(ty).expect("data types have defaults");
        let name = var.name.clone();
        self.checkpoint();
        self.graph.variables[i].value = value;
        for n in &mut self.graph.nodes {
            if n.var.as_deref() == Some(name.as_str()) && n.kind == "var.set" {
                if let Some(v) = Value::default_for(ty) {
                    n.values.insert("value".into(), v);
                }
            }
        }
        self.graph.prune_links();
    }

    pub fn add_variable_node(&mut self, i: usize, kind: &'static str) {
        let Some(name) = self.graph.variables.get(i).map(|v| v.name.clone()) else { return };
        let (_, size) = self.canvas.get();
        let at = self.to_graph(size * 0.5) - V2::new(layout::NODE_W * 0.5, 30.0);
        self.checkpoint();
        let id = self.graph.add_node(kind, Some(name), at.x, at.y);
        self.selected.clear();
        self.selected.insert(id);
    }

    // Selection --------------------------------------------------------------------------------

    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.checkpoint();
        let ids: Vec<NodeId> = self.selected.iter().copied().collect();
        self.graph.remove_nodes(&ids);
        self.selected.clear();
    }

    pub fn duplicate_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.checkpoint();
        let ids: Vec<NodeId> = self.selected.iter().copied().collect();
        let copies = self.graph.duplicate(&ids, 30.0, 30.0);
        self.selected = copies.into_iter().collect();
    }

    /// Fits the selection, or the whole graph, in the canvas.
    pub fn frame(&mut self) {
        let layouts = self.layouts();
        let pick: Vec<&NodeLayout> = if self.selected.is_empty() {
            layouts.iter().collect()
        } else {
            layouts.iter().filter(|l| self.selected.contains(&l.id)).collect()
        };
        let Some(first) = pick.first() else { return };
        let bounds = pick.iter().fold(first.rect, |acc, l| acc.union(&l.rect));
        let (_, size) = self.canvas.get();
        let margin = 60.0;
        let zoom = ((size.x - margin * 2.0) / bounds.w())
            .min((size.y - margin * 2.0) / bounds.h())
            .clamp(MIN_ZOOM, 1.2);
        self.zoom = zoom;
        let center = V2::new(bounds.min.x + bounds.w() * 0.5, bounds.min.y + bounds.h() * 0.5);
        self.pan = size * 0.5 - center * zoom;
    }

    pub fn focus_node(&mut self, id: NodeId) {
        self.selected.clear();
        self.selected.insert(id);
        if let Some(node) = self.graph.node(id) {
            let (_, size) = self.canvas.get();
            let center = V2::new(node.x + layout::NODE_W * 0.5, node.y + 40.0);
            self.pan = size * 0.5 - center * self.zoom;
        }
    }

    // Files ------------------------------------------------------------------------------------

    pub fn confirm_discard(&self) -> bool {
        !self.dirty
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
        *self = Self { graph: Graph::default(), path: None, ..self.reset() };
        self.update_title(window);
    }

    /// A fresh state that keeps the window's handles.
    pub fn reset(&self) -> Self {
        Self {
            graph: Graph::default(),
            path: None,
            dirty: false,
            pan: V2::new(60.0, 60.0),
            zoom: 1.0,
            selected: BTreeSet::new(),
            drag: Drag::None,
            menu: None,
            editing: None,
            undo: Vec::new(),
            redo: Vec::new(),
            focus: self.focus.clone(),
            show_code: self.show_code,
            status: String::new(),
            mouse: self.mouse,
            canvas: self.canvas.clone(),
            config: self.config.clone(),
            project: self.project.clone(),
            dialog: None,
            code_view: self.code_view,
            task: self.task.clone(),
        }
    }

    pub fn open(&mut self, window: &mut Window) {
        if !self.confirm_discard() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter("Visual script", &["lua"]).set_title("Open visual script");
        if let Some(project) = &self.project {
            dialog = dialog.set_directory(project.scripts_dir());
        }
        if let Some(path) = dialog.pick_file() {
            self.open_path(path, window);
        }
    }

    /// Opens a script without asking about unsaved changes. The project it belongs to, if any,
    /// becomes the current project.
    pub fn open_path(&mut self, path: PathBuf, window: &mut Window) {
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| vs_graph::load_from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(graph) => {
                let project = vs_project::Project::containing(&path).or_else(|| self.project.clone());
                *self = Self { graph, path: Some(path.clone()), project, ..self.reset() };
                self.frame();
                self.status = format!("Opened {}", path.display());
                self.update_title(window);
            }
            Err(e) => self.status = format!("Couldn't open {}: {e}", path.display()),
        }
    }

    pub fn save(&mut self, window: &mut Window) {
        match self.path.clone() {
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
        match std::fs::write(&path, vs_graph::save_to_string(&self.graph)) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.path = Some(path);
                self.dirty = false;
                self.update_title(window);
            }
            Err(e) => self.status = format!("Couldn't save {}: {e}", path.display()),
        }
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("untitled.lua".into(), |n| n.to_string_lossy().into_owned())
    }

    pub fn update_title(&self, window: &mut Window) {
        let project = self.project.as_ref().map(|p| format!(" ({})", p.name)).unwrap_or_default();
        window.set_window_title(&format!("{}{project} - Bevy Visual Script Editor", self.file_name()));
    }

    // Keyboard ---------------------------------------------------------------------------------

    pub fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window) {
        let ks = &event.keystroke;
        let ctrl = ks.modifiers.control || ks.modifiers.platform;
        let key = ks.key.as_str();
        let typed = ks.key_char.clone().filter(|_| !ctrl);

        if self.dialog.is_some() {
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

        if self.menu.is_some() {
            let count = self.menu_entries().len();
            let menu = self.menu.as_mut().unwrap();
            match key {
                "escape" => self.menu = None,
                "up" => menu.highlight = menu.highlight.saturating_sub(1),
                "down" => menu.highlight = (menu.highlight + 1).min(count.saturating_sub(1)),
                "enter" => {
                    let entries = self.menu_entries();
                    let pick = self.menu.as_ref().map_or(0, |m| m.highlight);
                    if let Some(entry) = entries.get(pick) {
                        self.place(&entry.clone());
                    }
                }
                "backspace" => {
                    menu.filter.pop();
                    menu.highlight = 0;
                }
                _ => {
                    if let Some(t) = typed {
                        menu.filter.push_str(&t);
                        menu.highlight = 0;
                    }
                }
            }
            return;
        }

        match (ctrl, ks.modifiers.shift, key) {
            (false, _, "delete" | "backspace") => self.delete_selected(),
            (true, false, "z") => self.undo(),
            (true, true, "z") | (true, _, "y") => self.redo(),
            (true, false, "s") => self.save(window),
            (true, true, "s") => self.save_as(window),
            (true, _, "o") => self.open(window),
            (true, _, "n") => self.new_graph(window),
            (true, _, "d") => self.duplicate_selected(),
            (true, _, "a") => self.selected = self.graph.nodes.iter().map(|n| n.id).collect(),
            (false, _, "f") => self.frame(),
            (false, _, "tab" | "space") => self.open_menu(self.mouse, None),
            _ => {}
        }
    }
}

/// The graph a new window opens with: a cube spinner, so there's something to look at.
fn starter_graph() -> Graph {
    vs_graph::starter()
}
