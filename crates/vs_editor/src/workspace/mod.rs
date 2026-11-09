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
}

impl Default for Workspace {
    fn default() -> Self {
        Self { config: vs_project::Config::load(), project: None, task: Rc::new(RefCell::new(None)) }
    }
}
