//! Visual scripting core: the graph model, the node library, the IR and its backends, and the
//! file format.
//!
//! A graph lowers to an [`ir::Module`] once ([`lower`]); the Lua backend ([`lua`]) writes the
//! preview from it, and the Rust backend ([`rust`]) the code shipped builds compile.
//!
//! A visual script is a `.lua` file with the graph embedded as JSON inside a Lua block comment,
//! followed by the code compiled from it. Lua ignores the comment, so the file runs as-is; the
//! editor reads the comment back. The graph is the source of truth: saving rewrites the code.

pub mod components;
pub mod graph;
pub mod ir;
pub mod library;
pub mod lower;
pub mod lua;
pub mod rust;

pub use lower::{Lowered, Problem, Severity, lower};
pub use vs_nodes;
pub use graph::{Graph, Link, Node, NodeId, PinRef, Side, Value, Variable};
pub use library::{NodeDef, PinDef, PinType};

const MARKER: &str = "bevy-visual-script";

/// A graph compiled for the preview.
#[derive(Clone, Debug, Default)]
pub struct Compiled {
    pub lua: String,
    pub problems: Vec<Problem>,
    /// The IR both backends read, for showing the Rust a script exports to.
    pub module: ir::Module,
}

impl Compiled {
    pub fn errors(&self) -> usize {
        self.problems.iter().filter(|p| p.severity == Severity::Error).count()
    }
}

pub fn compile(graph: &Graph) -> Compiled {
    let lowered = lower(graph);
    Compiled { lua: lua::emit(&lowered.module), problems: lowered.problems, module: lowered.module }
}

/// Serializes a graph into a runnable script file: the embedded graph, then the compiled Lua.
pub fn save_to_string(graph: &Graph) -> String {
    let json = serde_json::to_string_pretty(graph).expect("graphs always serialize");
    // Pick a long-bracket level the JSON can't close by accident.
    let mut level = 2;
    while json.contains(&format!("]{}]", "=".repeat(level))) {
        level += 1;
    }
    let eq = "=".repeat(level);
    let compiled = compile(graph);
    format!(
        "-- A visual script. bevy-editor generated this file from the graph below.\n\
         -- Edit the graph in the editor: changes to the code are replaced on save.\n\
         --[{eq}[{MARKER}\n{json}\n]{eq}]\n\n{}",
        compiled.lua
    )
}

/// A small script to start from: spins its entity at `speed` degrees per second.
pub fn starter() -> Graph {
    let mut g = Graph::default();
    g.variables.push(Variable { name: "speed".into(), value: Value::Number(90.0) });
    let update = g.add_node("event.update", None, 0.0, 0.0);
    let speed = g.add_node("var.get", Some("speed".into()), 0.0, 130.0);
    let mul = g.add_node("math.mul", None, 260.0, 110.0);
    let rotate = g.add_node("transform.rotate", None, 520.0, 0.0);
    g.link(PinRef::new(update, "then"), PinRef::new(rotate, "in"));
    g.link(PinRef::new(speed, "value"), PinRef::new(mul, "a"));
    g.link(PinRef::new(update, "delta"), PinRef::new(mul, "b"));
    g.link(PinRef::new(mul, "result"), PinRef::new(rotate, "y"));
    g
}

#[derive(Debug)]
pub enum LoadError {
    NotAVisualScript,
    Json(serde_json::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotAVisualScript => write!(f, "the file has no embedded visual script graph"),
            LoadError::Json(e) => write!(f, "the embedded graph is invalid: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Reads the graph embedded in a script file.
pub fn load_from_str(source: &str) -> Result<Graph, LoadError> {
    // Git on Windows may have checked the file out with CRLF line endings.
    let (start, newline) = match source.find(&format!("[{MARKER}\n")) {
        Some(i) => (i, 1),
        None => (source.find(&format!("[{MARKER}\r\n")).ok_or(LoadError::NotAVisualScript)?, 2),
    };
    let level = source[..start].chars().rev().take_while(|c| *c == '=').count();
    let body_start = start + MARKER.len() + 1 + newline;
    let closer = format!("]{}]", "=".repeat(level));
    let end = source[body_start..].find(&closer).ok_or(LoadError::NotAVisualScript)?;
    let mut graph: Graph =
        serde_json::from_str(&source[body_start..body_start + end]).map_err(LoadError::Json)?;
    graph.prune_links();
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spinner() -> Graph {
        let mut g = Graph::default();
        g.variables.push(Variable { name: "speed".into(), value: Value::Number(90.0) });
        let update = g.add_node("event.update", None, 0.0, 0.0);
        let rotate = g.add_node("transform.rotate", None, 300.0, 0.0);
        let speed = g.add_node("var.get", Some("speed".into()), 0.0, 150.0);
        let mul = g.add_node("math.mul", None, 150.0, 150.0);
        assert!(g.link(PinRef::new(update, "then"), PinRef::new(rotate, "in")));
        assert!(g.link(PinRef::new(speed, "value"), PinRef::new(mul, "a")));
        assert!(g.link(PinRef::new(update, "delta"), PinRef::new(mul, "b")));
        assert!(g.link(PinRef::new(mul, "result"), PinRef::new(rotate, "y")));
        g
    }

    #[test]
    fn compiles_a_spinner() {
        let out = compile(&spinner());
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        assert!(out.lua.contains("function on_update()"));
        assert!(out.lua.contains("local t0 = node.math.mul(vars[1], vs.delta())"), "{}", out.lua);
        assert!(out.lua.contains("vs.rotate(entity, 0, t0, 0)"), "{}", out.lua);
    }

    #[test]
    fn reads_files_with_crlf_line_endings() {
        let g = spinner();
        let file = save_to_string(&g).replace('\n', "\r\n");
        assert_eq!(load_from_str(&file).unwrap(), g);
    }

    #[test]
    fn round_trips_through_a_file() {
        let g = spinner();
        let file = save_to_string(&g);
        assert_eq!(load_from_str(&file).unwrap(), g);
    }

    #[test]
    fn rejects_mismatched_pins() {
        let mut g = Graph::default();
        let start = g.add_node("event.start", None, 0.0, 0.0);
        let add = g.add_node("math.add", None, 0.0, 0.0);
        assert!(!g.link(PinRef::new(start, "then"), PinRef::new(add, "a")));
    }

    #[test]
    fn reports_unreachable_nodes_and_loops() {
        let mut g = Graph::default();
        let print = g.add_node("action.print", None, 0.0, 0.0);
        let a = g.add_node("math.add", None, 0.0, 0.0);
        let b = g.add_node("math.add", None, 0.0, 0.0);
        g.link(PinRef::new(a, "result"), PinRef::new(b, "a"));
        g.link(PinRef::new(b, "result"), PinRef::new(a, "a"));
        let start = g.add_node("event.start", None, 0.0, 0.0);
        let p2 = g.add_node("action.print", None, 0.0, 0.0);
        g.link(PinRef::new(start, "then"), PinRef::new(p2, "in"));
        g.link(PinRef::new(a, "result"), PinRef::new(p2, "value"));
        let out = compile(&g);
        assert!(
            out.problems
                .iter()
                .any(|p| p.node == Some(print) && p.severity == Severity::Warning)
        );
        assert!(out.problems.iter().any(|p| p.message.contains("loop")));
    }
}
