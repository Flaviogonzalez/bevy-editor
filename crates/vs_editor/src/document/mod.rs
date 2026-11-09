//! The open script: its graph, the file it lives in, its undo history, which nodes are selected
//! and where the canvas camera looks at it. Everything here belongs to one file, so when the
//! editor holds several files later, each gets its own `Document`.

pub mod history;

use std::collections::BTreeSet;
use std::path::PathBuf;

use vs_graph::{Graph, NodeId};

use crate::canvas::layout::V2;
use history::History;

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
