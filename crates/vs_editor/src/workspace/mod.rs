//! The project around the open script: the settings, creating and opening projects, the catalog
//! of components, and the cargo runs (preview, build) started from the editor.

mod project;
pub mod tasks;

use std::cell::RefCell;
use std::rc::Rc;

use tasks::Task;

/// What outlives the open script: the settings, the project it belongs to, and the cargo run.
pub struct Workspace {
    pub config: vs_project::Config,
    /// The project the open script belongs to, if any. Change it with [`Editor::set_project`],
    /// which also loads that project's catalog of components.
    ///
    /// [`Editor::set_project`]: crate::editor::Editor::set_project
    pub project: Option<vs_project::Project>,
    /// The cargo run in progress or last finished (preview or build). Opening another script
    /// keeps it, so the run isn't stopped.
    pub task: Rc<RefCell<Option<Task>>>,
    /// The systems of the project's other scripts, by file name, as last read from disk (see
    /// [`Editor::scan_systems`]). The open script's own come from the graph on screen.
    ///
    /// [`Editor::scan_systems`]: crate::editor::Editor::scan_systems
    pub others: Vec<(String, Vec<vs_graph::access::System>)>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self { config: vs_project::Config::load(), project: None, task: Rc::new(RefCell::new(None)), others: Vec::new() }
    }
}
