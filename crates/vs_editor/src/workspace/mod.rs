//! The project around the open script: the settings, creating and opening projects, the catalog
//! of components, and the cargo runs (preview, build) started from the editor.

mod project;
pub mod tasks;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use vs_graph::ir::Module;
use vs_graph::{Graph, NodeId};

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
    /// What each of the project's other scripts holds, by path, read with `others`. The
    /// Explorer lists it under each plugin; the open script's comes from the graph on screen.
    pub outlines: BTreeMap<PathBuf, Outline>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            config: vs_project::Config::load(),
            project: None,
            task: Rc::new(RefCell::new(None)),
            others: Vec::new(),
            outlines: BTreeMap::new(),
        }
    }
}

/// What a script's plugin holds, as the Explorer lists it under the plugin: one list per kind of
/// Bevy piece. Each entry carries the node it comes from, so a click can show it on the canvas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    /// The plugin's type name, such as `LightBehaviorPlugin`.
    pub plugin: String,
    /// Its systems, in the order the plugin adds them, each with the event it compiles from.
    pub systems: Vec<(String, NodeId)>,
    /// Its components: the script's own first, which has no card on the canvas, then the ones
    /// its Define Component cards declare.
    pub components: Vec<(String, Option<NodeId>)>,
}

impl Outline {
    /// The outline of the script at `file` (its path in the scripts folder, or its file name),
    /// from its graph and the module lowered from it.
    pub fn of(graph: &Graph, module: &Module, file: &str) -> Self {
        let names = vs_graph::rust::names(file);
        let systems = vs_graph::access::systems(module, &names)
            .into_iter()
            .map(|s| {
                let event = module.blocks[s.block].node;
                (s.name, event)
            })
            .collect();
        let declared = graph.nodes.iter().filter_map(|n| n.component.as_ref().map(|c| (c.name.clone(), Some(n.id))));
        let components = std::iter::once((names.ty, None)).chain(declared).collect();
        Self { plugin: names.plugin, systems, components }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vs_graph::ComponentDef;

    #[test]
    fn a_plugin_lists_its_systems_then_its_own_and_declared_components() {
        let mut graph = vs_graph::starter();
        let card = graph.add_component_def(ComponentDef { name: "Health".into(), fields: Vec::new() }, 0.0, 400.0);
        let start = graph.add_node("event.start", None, 0.0, 600.0);
        let module = vs_graph::lower(&graph).module;
        let outline = Outline::of(&graph, &module, "player/spinner.lua");
        assert_eq!(outline.plugin, "SpinnerPlugin");
        // The Start system comes first, as the plugin adds it.
        let names: Vec<&str> = outline.systems.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["setup_spinner", "update_spinner"]);
        assert_eq!(outline.systems[0].1, start);
        assert_eq!(outline.components, [("Spinner".to_string(), None), ("Health".to_string(), Some(card))]);
    }
}
