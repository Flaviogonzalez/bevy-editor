//! The node library: every node type the editor can place.
//!
//! Two sources. Events, flow, effects, intrinsics and variables are the fixed set listed here;
//! [`crate::lower`] gives them their meaning. Pure nodes (math, logic, text) come from
//! `vs_nodes::catalog()`, which reads them from the Rust functions by reflection, so adding a
//! function there adds a node here with no other change.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::graph::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinType {
    Exec,
    Number,
    Bool,
    String,
    Vec3,
    /// A rotation. Literals are Euler angles in degrees.
    Quat,
    Color,
    /// Accepts any data value. Variable nodes resolve it to the variable's type.
    Any,
}

impl PinType {
    pub fn connects_to(self, input: PinType) -> bool {
        match (self, input) {
            (PinType::Exec, PinType::Exec) => true,
            (PinType::Exec, _) | (_, PinType::Exec) => false,
            (_, PinType::Any) | (PinType::Any, _) => true,
            // Every value prints fine where text is expected.
            (_, PinType::String) => true,
            (a, b) => a == b,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PinType::Exec => "exec",
            PinType::Number => "number",
            PinType::Bool => "bool",
            PinType::String => "string",
            PinType::Vec3 => "vector",
            PinType::Quat => "rotation",
            PinType::Color => "color",
            PinType::Any => "any",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PinDef {
    pub name: String,
    pub ty: PinType,
    pub default: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct NodeDef {
    pub kind: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
}

impl NodeDef {
    pub fn is_event(&self) -> bool {
        self.category == "Events"
    }

    pub fn is_variable(&self) -> bool {
        self.kind.starts_with("var.")
    }

    /// Get / Set Component: one menu entry per component field.
    pub fn is_field(&self) -> bool {
        self.kind.starts_with("component.")
    }
}

fn exec(name: &str) -> PinDef {
    PinDef { name: name.into(), ty: PinType::Exec, default: None }
}

fn num(name: &str, default: f64) -> PinDef {
    PinDef { name: name.into(), ty: PinType::Number, default: Some(Value::Number(default)) }
}

fn boolean(name: &str) -> PinDef {
    PinDef { name: name.into(), ty: PinType::Bool, default: Some(Value::Bool(false)) }
}

fn text(name: &str, default: &str) -> PinDef {
    PinDef { name: name.into(), ty: PinType::String, default: Some(Value::Str(default.into())) }
}

fn out(name: &str, ty: PinType) -> PinDef {
    PinDef { name: name.into(), ty, default: None }
}

fn any(name: &str) -> PinDef {
    PinDef { name: name.into(), ty: PinType::Any, default: None }
}

fn def(
    kind: &'static str,
    title: &'static str,
    category: &'static str,
    description: &'static str,
    inputs: Vec<PinDef>,
    outputs: Vec<PinDef>,
) -> NodeDef {
    NodeDef { kind, title, category, description, inputs, outputs }
}

/// The pin type a pure node's Rust type maps to.
pub fn pin_type(kind: vs_nodes::PinKind) -> PinType {
    match kind {
        vs_nodes::PinKind::Number => PinType::Number,
        vs_nodes::PinKind::Bool => PinType::Bool,
        vs_nodes::PinKind::Text => PinType::String,
        vs_nodes::PinKind::Vec3 => PinType::Vec3,
        vs_nodes::PinKind::Quat => PinType::Quat,
        vs_nodes::PinKind::Color => PinType::Color,
    }
}

fn pure(node: vs_nodes::PureNode) -> NodeDef {
    let pin = |(name, kind): &(String, vs_nodes::PinKind)| {
        let ty = pin_type(*kind);
        PinDef { name: name.clone(), ty, default: Value::default_for(ty) }
    };
    let inputs = node.inputs.iter().map(pin).collect();
    def(node.kind, node.title, node.category, node.description, inputs, vec![out("result", pin_type(node.output))])
}

fn build() -> Vec<NodeDef> {
    use PinType::*;
    let mut defs = vec![
        // Events: entry points. Each becomes (part of) a script callback.
        def("event.start", "On Start", "Events", "Runs once, when the script is loaded.", vec![], vec![exec("then")]),
        def(
            "event.update",
            "On Update",
            "Events",
            "Runs every frame. `delta` is the frame time in seconds: multiply speeds by it for frame-rate independent motion.",
            vec![],
            vec![exec("then"), out("delta", Number)],
        ),
        def(
            "event.key",
            "On Key Pressed",
            "Events",
            "Runs on the frame a key goes down. Key names follow Bevy's KeyCode, such as Space, KeyW or ArrowUp.",
            vec![text("key", "Space")],
            vec![exec("then")],
        ),
        // Flow control.
        def(
            "flow.branch",
            "Branch",
            "Flow",
            "If / else: runs `true` or `false` depending on the condition.",
            vec![exec("in"), boolean("condition")],
            vec![exec("true"), exec("false")],
        ),
        def(
            "flow.sequence",
            "Sequence",
            "Flow",
            "Runs `first`, then `second`.",
            vec![exec("in")],
            vec![exec("first"), exec("second")],
        ),
        def(
            "flow.flipflop",
            "Flip Flop",
            "Flow",
            "Alternates between `a` and `b` each time it runs. `is_a` tells which one ran.",
            vec![exec("in")],
            vec![exec("a"), exec("b"), out("is_a", Bool)],
        ),
        def(
            "flow.do_once",
            "Do Once",
            "Flow",
            "Lets execution through the first time only.",
            vec![exec("in")],
            vec![exec("then")],
        ),
        // Actions.
        def("action.print", "Print", "Actions", "Writes a value to the log.", vec![exec("in"), text("value", "Hello")], vec![exec("then")]),
        // Transform.
        def(
            "transform.translate",
            "Translate",
            "Transform",
            "Moves the entity by an offset, in world units.",
            vec![exec("in"), num("x", 0.0), num("y", 0.0), num("z", 0.0)],
            vec![exec("then")],
        ),
        def(
            "transform.rotate",
            "Rotate",
            "Transform",
            "Rotates the entity by Euler angles, in degrees.",
            vec![exec("in"), num("x", 0.0), num("y", 0.0), num("z", 0.0)],
            vec![exec("then")],
        ),
        def(
            "transform.set_position",
            "Set Position",
            "Transform",
            "Places the entity at a position.",
            vec![exec("in"), num("x", 0.0), num("y", 0.0), num("z", 0.0)],
            vec![exec("then")],
        ),
        def(
            "transform.set_scale",
            "Set Scale",
            "Transform",
            "Sets the entity's scale on all three axes.",
            vec![exec("in"), num("x", 1.0), num("y", 1.0), num("z", 1.0)],
            vec![exec("then")],
        ),
        def(
            "transform.position",
            "Get Position",
            "Transform",
            "The entity's position.",
            vec![],
            vec![out("x", Number), out("y", Number), out("z", Number)],
        ),
        // Input.
        def(
            "input.key_down",
            "Is Key Down",
            "Input",
            "True while a key is held.",
            vec![text("key", "Space")],
            vec![out("down", Bool)],
        ),
        def(
            "input.key_pressed",
            "Key Just Pressed",
            "Input",
            "True on the frame a key goes down.",
            vec![text("key", "Space")],
            vec![out("pressed", Bool)],
        ),
        def(
            "input.key_released",
            "Key Just Released",
            "Input",
            "True on the frame a key comes up.",
            vec![text("key", "Space")],
            vec![out("released", Bool)],
        ),
        def(
            "input.mouse_down",
            "Is Mouse Button Down",
            "Input",
            "True while a mouse button is held: Left, Right, Middle, Back or Forward.",
            vec![text("button", "Left")],
            vec![out("down", Bool)],
        ),
        def(
            "input.mouse_pressed",
            "Mouse Button Just Pressed",
            "Input",
            "True on the frame a mouse button goes down: Left, Right, Middle, Back or Forward.",
            vec![text("button", "Left")],
            vec![out("pressed", Bool)],
        ),
        def(
            "input.mouse_released",
            "Mouse Button Just Released",
            "Input",
            "True on the frame a mouse button comes up: Left, Right, Middle, Back or Forward.",
            vec![text("button", "Left")],
            vec![out("released", Bool)],
        ),
        // Entity.
        def(
            "entity.despawn",
            "Despawn",
            "Entity",
            "Removes this entity, and its children, at the end of the frame. The steps after it still run this frame.",
            vec![exec("in")],
            vec![exec("then")],
        ),
        // Time.
        def("time.elapsed", "Elapsed Time", "Time", "Seconds since the app started.", vec![], vec![out("seconds", Number)]),
        // Components. Their `Any` pins take the field's type.
        def(
            "component.get",
            "Get",
            "Components",
            "Reads a field of one of the entity's components. The script's component requires it, so the entity always has it.",
            vec![],
            vec![any("value")],
        ),
        def(
            "component.set",
            "Set",
            "Components",
            "Writes a field of one of the entity's components. The script's component requires it, so the entity always has it.",
            vec![exec("in"), any("value")],
            vec![exec("then"), any("value")],
        ),
        // Variables. Their `Any` pins take the variable's type.
        def("var.get", "Get", "Variables", "Reads a graph variable.", vec![], vec![any("value")]),
        def("var.set", "Set", "Variables", "Writes a graph variable.", vec![exec("in"), any("value")], vec![exec("then"), any("value")]),
    ];
    // Pure nodes go before the components and variables, so the menu keeps its order.
    let at = defs.len() - 4;
    defs.splice(at..at, vs_nodes::catalog().into_iter().map(pure));
    defs
}

pub fn all() -> &'static [NodeDef] {
    static LIBRARY: OnceLock<Vec<NodeDef>> = OnceLock::new();
    LIBRARY.get_or_init(build)
}

pub fn find(kind: &str) -> Option<&'static NodeDef> {
    all().iter().find(|d| d.kind == kind)
}
