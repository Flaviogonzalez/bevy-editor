//! The open script: its graph, the file it lives in, its undo history, which nodes are selected
//! and where the canvas camera looks at it. Everything here belongs to one file, so when the
//! editor holds several files later, each gets its own `Document`.
//!
//! The operations here only touch the document, never the window or the panels, so they can be
//! tested without GPUI. Whatever also needs the rest of the editor (a text edit, the status
//! line, the project) lives on `Editor` and calls into these.
//!
//! Depends on `canvas::layout` for node geometry, which framing and hit tests share.

pub mod history;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vs_graph::{Graph, NodeId, PinType, Value, Variable};

use crate::canvas::layout::{self, NodeLayout, V2};
use history::History;

pub const MIN_ZOOM: f32 = 0.3;
pub const MAX_ZOOM: f32 = 2.0;

pub struct Document {
    pub graph: Graph,
    /// Where the script is saved; `None` until the first save.
    pub path: Option<PathBuf>,
    /// Undo, redo and unsaved changes.
    pub history: History,
    pub selected: BTreeSet<NodeId>,
    /// The camera: the canvas-local position of the graph's origin, and the scale.
    pub pan: V2,
    pub zoom: f32,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            graph: Graph::default(),
            path: None,
            history: History::default(),
            selected: BTreeSet::new(),
            pan: V2::new(60.0, 60.0),
            zoom: 1.0,
        }
    }
}

impl Document {
    // File -------------------------------------------------------------------------------------

    /// Reads and parses the script at `path`.
    pub fn load(path: &Path) -> Result<Self, String> {
        let graph = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| vs_graph::load_from_str(&s).map_err(|e| e.to_string()))?;
        Ok(Self { graph, path: Some(path.to_path_buf()), ..Self::default() })
    }

    /// Writes the script to `path`, which becomes its file, and marks it saved.
    pub fn write_to(&mut self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, vs_graph::save_to_string(&self.graph))?;
        self.path = Some(path.to_path_buf());
        self.history.mark_saved();
        Ok(())
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("untitled.lua".into(), |n| n.to_string_lossy().into_owned())
    }

    // Camera -----------------------------------------------------------------------------------

    pub fn to_graph(&self, local: V2) -> V2 {
        (local - self.pan) * (1.0 / self.zoom)
    }

    pub fn to_screen(&self, graph: V2) -> V2 {
        graph * self.zoom + self.pan
    }

    pub fn layouts(&self) -> Vec<NodeLayout> {
        self.graph.nodes.iter().map(|n| layout::layout(&self.graph, n)).collect()
    }

    /// Zooms by a wheel delta, keeping the graph point under `local` where it is.
    pub fn zoom_at(&mut self, local: V2, dy: f32) {
        let before = self.to_graph(local);
        self.zoom = (self.zoom * 1.0015f32.powf(dy)).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = local - before * self.zoom;
    }

    /// Fits the selection, or the whole graph, in a canvas of `size`.
    pub fn frame(&mut self, size: V2) {
        let layouts = self.layouts();
        let pick: Vec<&NodeLayout> = if self.selected.is_empty() {
            layouts.iter().collect()
        } else {
            layouts.iter().filter(|l| self.selected.contains(&l.id)).collect()
        };
        let Some(first) = pick.first() else { return };
        let bounds = pick.iter().fold(first.rect, |acc, l| acc.union(&l.rect));
        let margin = 60.0;
        let zoom = ((size.x - margin * 2.0) / bounds.w())
            .min((size.y - margin * 2.0) / bounds.h())
            .clamp(MIN_ZOOM, 1.2);
        self.zoom = zoom;
        let center = V2::new(bounds.min.x + bounds.w() * 0.5, bounds.min.y + bounds.h() * 0.5);
        self.pan = size * 0.5 - center * zoom;
    }

    /// Selects `id` and centers it in a canvas of `size`, keeping the zoom.
    pub fn focus_node(&mut self, id: NodeId, size: V2) {
        self.select_only(id);
        if let Some(node) = self.graph.node(id) {
            let center = V2::new(node.x + layout::NODE_W * 0.5, node.y + 40.0);
            self.pan = size * 0.5 - center * self.zoom;
        }
    }

    // History ----------------------------------------------------------------------------------

    /// Call before every change to the graph.
    pub fn checkpoint(&mut self) {
        self.history.record(self.graph.clone());
    }

    /// Returns whether there was something to undo.
    pub fn undo(&mut self) -> bool {
        let changed = self.history.undo(&mut self.graph);
        if changed {
            self.forget_missing();
        }
        changed
    }

    /// Returns whether there was something to redo.
    pub fn redo(&mut self) -> bool {
        let changed = self.history.redo(&mut self.graph);
        if changed {
            self.forget_missing();
        }
        changed
    }

    /// Drops selected nodes that the graph no longer has.
    fn forget_missing(&mut self) {
        self.selected.retain(|id| self.graph.node(*id).is_some());
    }

    // Selection --------------------------------------------------------------------------------

    pub fn select_only(&mut self, id: NodeId) {
        self.selected.clear();
        self.selected.insert(id);
    }

    pub fn select_all(&mut self) {
        self.selected = self.graph.nodes.iter().map(|n| n.id).collect();
    }

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

    /// Draws `id` above the other nodes (the last node is drawn last).
    pub fn bring_to_front(&mut self, id: NodeId) {
        if let Some(i) = self.graph.nodes.iter().position(|n| n.id == id) {
            let node = self.graph.nodes.remove(i);
            self.graph.nodes.push(node);
        }
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

    /// Removes variable `i` and every node that reads or writes it. Returns whether it existed.
    pub fn remove_variable(&mut self, i: usize) -> bool {
        let Some(name) = self.graph.variables.get(i).map(|v| v.name.clone()) else { return false };
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
        true
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

    /// Adds a Get or Set node (`kind`) for variable `i` at `at`, in graph units, and selects it.
    pub fn add_variable_node(&mut self, i: usize, kind: &'static str, at: V2) {
        let Some(name) = self.graph.variables.get(i).map(|v| v.name.clone()) else { return };
        self.checkpoint();
        let id = self.graph.add_node(kind, Some(name), at.x, at.y);
        self.select_only(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_variable() -> Document {
        let mut doc = Document::default();
        doc.add_variable();
        doc.add_variable_node(0, "var.get", V2::new(0.0, 0.0));
        doc.add_variable_node(0, "var.set", V2::new(0.0, 100.0));
        doc
    }

    #[test]
    fn removing_a_variable_removes_its_nodes_and_undo_brings_them_back() {
        let mut doc = doc_with_variable();
        assert_eq!(doc.graph.nodes.len(), 2);
        assert!(doc.remove_variable(0));
        assert!(doc.graph.variables.is_empty());
        assert!(doc.graph.nodes.is_empty());
        assert!(doc.undo());
        assert_eq!(doc.graph.variables.len(), 1);
        assert_eq!(doc.graph.nodes.len(), 2);
        assert!(!doc.remove_variable(5));
    }

    #[test]
    fn undo_drops_selected_nodes_that_no_longer_exist() {
        let mut doc = doc_with_variable();
        let last = doc.graph.nodes[1].id;
        assert_eq!(doc.selected, BTreeSet::from([last]));
        assert!(doc.undo());
        assert!(doc.selected.is_empty());
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer() {
        let mut doc = Document::default();
        let pointer = V2::new(300.0, 200.0);
        let before = doc.to_graph(pointer);
        doc.zoom_at(pointer, 200.0);
        assert!(doc.zoom > 1.0);
        let after = doc.to_graph(pointer);
        assert!((before.x - after.x).abs() < 1e-3 && (before.y - after.y).abs() < 1e-3);
        doc.zoom_at(pointer, 1e6);
        assert_eq!(doc.zoom, MAX_ZOOM);
    }

    #[test]
    fn frame_centers_the_graph_in_the_canvas() {
        let mut doc = doc_with_variable();
        doc.selected.clear();
        let size = V2::new(1000.0, 800.0);
        doc.frame(size);
        let layouts = doc.layouts();
        let bounds = layouts.iter().skip(1).fold(layouts[0].rect, |acc, l| acc.union(&l.rect));
        let center = doc.to_screen(V2::new(bounds.min.x + bounds.w() * 0.5, bounds.min.y + bounds.h() * 0.5));
        assert!((center.x - 500.0).abs() < 1e-3 && (center.y - 400.0).abs() < 1e-3);
    }
}
