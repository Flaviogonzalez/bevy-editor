//! Undo, redo and whether the graph has unsaved changes.
//!
//! The three move together, so they live behind one type: every change to the graph goes through
//! [`History::record`], which is the only place that caps the undo stack, clears redo and marks
//! the graph dirty.

use vs_graph::Graph;

/// How many undo steps are kept.
pub const MAX_UNDO: usize = 200;

#[derive(Default)]
pub struct History {
    undo: Vec<Graph>,
    redo: Vec<Graph>,
    dirty: bool,
}

impl History {
    /// Records the graph as it was before a change.
    pub fn record(&mut self, before: Graph) {
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    /// Steps `graph` back. Returns whether there was a step to undo.
    pub fn undo(&mut self, graph: &mut Graph) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(graph, prev));
        self.dirty = true;
        true
    }

    /// Steps `graph` forward again. Returns whether there was a step to redo.
    pub fn redo(&mut self, graph: &mut Graph) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(graph, next));
        self.dirty = true;
        true
    }

    /// The graph was just written to disk.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(n: usize) -> Graph {
        let mut g = Graph::default();
        for _ in 0..n {
            g.add_node("event.update", None, 0.0, 0.0);
        }
        g
    }

    #[test]
    fn undo_and_redo_swap_graphs() {
        let mut h = History::default();
        let mut g = graph(1);
        h.record(g.clone());
        g = graph(2);
        assert!(h.undo(&mut g));
        assert_eq!(g, graph(1));
        assert!(h.redo(&mut g));
        assert_eq!(g, graph(2));
    }

    #[test]
    fn empty_stacks_do_nothing() {
        let mut h = History::default();
        let mut g = graph(1);
        assert!(!h.undo(&mut g));
        assert!(!h.redo(&mut g));
        assert_eq!(g, graph(1));
        assert!(!h.is_dirty());
    }

    #[test]
    fn recording_clears_redo_and_marks_dirty() {
        let mut h = History::default();
        let mut g = graph(1);
        h.record(g.clone());
        h.undo(&mut g);
        h.mark_saved();
        h.record(g.clone());
        assert!(h.is_dirty());
        assert!(!h.redo(&mut g));
    }

    #[test]
    fn undo_is_capped() {
        let mut h = History::default();
        for i in 0..MAX_UNDO + 50 {
            h.record(graph(i % 3));
        }
        let mut g = Graph::default();
        let mut steps = 0;
        while h.undo(&mut g) {
            steps += 1;
        }
        assert_eq!(steps, MAX_UNDO);
    }
}
