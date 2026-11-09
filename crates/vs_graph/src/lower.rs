//! Graph → IR.
//!
//! Events become blocks, exec wires become steps in order, and data wires become `Let`s placed
//! right before the step that needs them. Problems (unknown nodes, loops, nodes that never run)
//! are reported here once, for both backends.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::components;
use crate::graph::{Graph, Node, NodeId, PinRef, Side, Value};
use crate::ir::{Action, Axis, Block, ComponentUse, Declared, Device, EffectOp, Event, Intrinsic, Let, Lit, Module, Operand, Press, Step, Variable};
use crate::library::{self, PinType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub severity: Severity,
    pub node: Option<NodeId>,
    pub message: String,
}

pub struct Lowered {
    pub module: Module,
    pub problems: Vec<Problem>,
}

impl Lowered {
    pub fn errors(&self) -> usize {
        self.problems.iter().filter(|p| p.severity == Severity::Error).count()
    }
}

pub fn lower(graph: &Graph) -> Lowered {
    let mut l = Lowerer {
        graph,
        problems: Vec::new(),
        reached: BTreeSet::new(),
        states: BTreeMap::new(),
        state_names: Vec::new(),
        next_local: 0,
        keys: vs_nodes::key_names(),
        pure: vs_nodes::catalog(),
        components: Vec::new(),
    };

    let mut declares = Vec::new();
    for node in &graph.nodes {
        match library::find(&node.kind) {
            None => l.error(node.id, format!("Unknown node type `{}`", node.kind)),
            Some(def) if def.is_definition() => {
                if let Some(declared) = l.declaration(node) {
                    declares.push(declared);
                }
            }
            Some(def) if def.is_copy() => match &node.copy_of {
                Some(name) if components::find(name).is_some_and(|c| c.source.is_some()) => {}
                Some(name) if graph.defined_components().any(|d| d.name == *name) => {}
                Some(name) => l.warning(
                    node.id,
                    format!("No script in the project declares `{name}`: it was renamed or removed"),
                ),
                None => l.error(node.id, "Component copy without a component".into()),
            },
            Some(def) if def.is_variable() => match &node.var {
                Some(name) if graph.variable(name).is_some() => {}
                Some(name) => l.error(node.id, format!("Variable `{name}` doesn't exist")),
                None => l.error(node.id, "Variable node without a variable".into()),
            },
            Some(def) if def.is_field() => match &node.field {
                Some(field) if components::field_in(graph, field).is_some() => {}
                Some(field) => l.error(
                    node.id,
                    format!("Unknown component field `{field}`. If it's the game's own, run the preview once so the editor learns it"),
                ),
                None => l.error(node.id, "Component node without a field".into()),
            },
            _ => {}
        }
    }

    let variables = graph
        .variables
        .iter()
        .map(|v| Variable { name: v.name.clone(), initial: lit_of(&v.value) })
        .collect();

    let mut blocks = Vec::new();
    for node in &graph.nodes {
        let event = match node.kind.as_str() {
            "event.start" => Event::Start,
            "event.update" => Event::Update,
            "event.key" => match l.key(node, "key") {
                Some(key) => Event::Key(key),
                None => continue,
            },
            _ => continue,
        };
        l.reached.insert(node.id);
        let mut body = Vec::new();
        l.chain(&PinRef::new(node.id, "then"), &mut body, &mut Vec::new());
        blocks.push(Block { comment: label(node), node: node.id, name: node.name.clone(), event, body });
    }

    for node in &graph.nodes {
        let Some(def) = library::find(&node.kind) else { continue };
        let has_exec_in = def.inputs.iter().any(|p| p.ty == PinType::Exec);
        if has_exec_in && !l.reached.contains(&node.id) {
            l.warning(node.id, format!("{} never runs: connect it to an event", title_of(node)));
        }
    }

    let module = Module { variables, states: l.state_names, components: l.components, blocks, declares };
    Lowered { module, problems: l.problems }
}

/// A literal as the IR stores it: numbers become `f32` here, once.
pub fn lit_of(value: &Value) -> Lit {
    let f = |v: &[f64]| -> Vec<f32> { v.iter().map(|n| *n as f32).collect() };
    match value {
        Value::Bool(b) => Lit::Bool(*b),
        Value::Number(n) => Lit::Number(*n as f32),
        Value::Str(s) => Lit::Text(s.clone()),
        Value::Vec3(v) => Lit::Vec3(f(v).try_into().expect("three components")),
        Value::Quat { euler } => Lit::Quat(f(euler).try_into().expect("three angles")),
        Value::Color { rgba } => Lit::Color(f(rgba).try_into().expect("four components")),
    }
}

fn default_lit(ty: PinType) -> Lit {
    Value::default_for(ty).as_ref().map_or(Lit::Number(0.0), lit_of)
}

/// Whether a literal fits a pin of type `ty`.
fn fits(lit: &Lit, ty: PinType) -> bool {
    matches!(
        (lit, ty),
        (Lit::Number(_), PinType::Number)
            | (Lit::Bool(_), PinType::Bool)
            | (Lit::Text(_), PinType::String)
            | (Lit::Vec3(_), PinType::Vec3)
            | (Lit::Quat(_), PinType::Quat)
            | (Lit::Color(_), PinType::Color)
    )
}

fn title_of(node: &Node) -> String {
    let base = library::find(&node.kind).map_or(node.kind.as_str(), |d| d.title);
    match (&node.var, &node.field) {
        (Some(var), _) => format!("{base} {var}"),
        (None, Some(field)) => format!("{base} {field}"),
        (None, None) => base.to_string(),
    }
}

/// `Join Text` → `join_text`.
pub fn snake_case(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && !out.is_empty() && !out.ends_with('_') && out.chars().last().is_some_and(|p| p.is_lowercase()) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

fn label(node: &Node) -> String {
    format!("{} [{}]", title_of(node), node.id)
}

/// Data outputs already computed in the current step, so a node feeding two inputs runs once.
type Cache = HashMap<PinRef, Operand>;

struct Lowerer<'a> {
    graph: &'a Graph,
    problems: Vec<Problem>,
    /// Nodes reached from an event, to warn about the ones that never run.
    reached: BTreeSet<NodeId>,
    /// State slot of each Flip Flop / Do Once node.
    states: BTreeMap<NodeId, usize>,
    state_names: Vec<String>,
    next_local: u32,
    keys: Vec<&'static str>,
    pure: Vec<vs_nodes::PureNode>,
    components: Vec<ComponentUse>,
}

impl Lowerer<'_> {
    fn error(&mut self, node: NodeId, message: String) {
        self.problems.push(Problem { severity: Severity::Error, node: Some(node), message });
    }

    fn warning(&mut self, node: NodeId, message: String) {
        self.problems.push(Problem { severity: Severity::Warning, node: Some(node), message });
    }

    fn state(&mut self, node: &Node) -> usize {
        if let Some(slot) = self.states.get(&node.id) {
            return *slot;
        }
        let slot = self.state_names.len();
        let default = if node.kind == "flow.do_once" { "done" } else { "flip_flop" };
        self.state_names.push(node.name.clone().unwrap_or_else(|| default.to_string()));
        self.states.insert(node.id, slot);
        slot
    }

    /// The component and path a Get / Set Component node refers to, adding the component to the
    /// ones the script uses.
    fn field(&mut self, node: &Node) -> Option<(usize, String, PinType)> {
        let (component, field) = components::field_in(self.graph, node.field.as_deref()?)?;
        let index = match self.components.iter().position(|c| c.name == component.name) {
            Some(i) => i,
            None => {
                self.components.push(ComponentUse { name: component.name, rust_path: component.rust_path });
                self.components.len() - 1
            }
        };
        Some((index, field.path, field.ty))
    }

    /// Checks a Define Component node: the component needs a type name that nothing else uses,
    /// and its fields need distinct snake_case names, since the preview reaches them by those
    /// names through reflection. `None` when it has errors.
    fn declaration(&mut self, node: &Node) -> Option<Declared> {
        let Some(def) = &node.component else {
            self.error(node.id, "Component card without a declaration".into());
            return None;
        };
        let name = &def.name;
        let mut ok = true;
        let type_name = name.starts_with(|c: char| c.is_ascii_uppercase()) && name.chars().all(|c| c.is_ascii_alphanumeric());
        if !type_name {
            self.error(
                node.id,
                format!("`{name}` isn't a component name: start with a capital letter and use only letters and digits, such as `Health`"),
            );
            ok = false;
        } else if crate::rust::is_reserved_type(name) || components::is_bundled(name) {
            self.error(node.id, format!("`{name}` is already a Bevy type: pick another name"));
            ok = false;
        }
        let first = self.graph.defined_components().position(|d| d.name == *name);
        let this = self.graph.nodes.iter().filter(|n| n.component.is_some()).position(|n| n.id == node.id);
        if first != this {
            self.error(node.id, format!("`{name}` is declared twice in this script"));
            ok = false;
        }
        for (i, field) in def.fields.iter().enumerate() {
            let f = &field.name;
            if !crate::rust::is_field_name(f) {
                self.error(node.id, format!("Field `{f}` of `{name}` needs a snake_case name, such as `max_value`"));
                ok = false;
            } else if def.fields[..i].iter().any(|other| other.name == *f) {
                self.error(node.id, format!("`{name}` has two fields named `{f}`"));
                ok = false;
            }
        }
        let fields = def.fields.iter().map(|f| Variable { name: f.name.clone(), initial: lit_of(&f.value) }).collect();
        ok.then(|| Declared { name: name.clone(), fields })
    }

    fn var(&self, node: &Node) -> Option<usize> {
        let name = node.var.as_deref()?;
        self.graph.variables.iter().position(|v| v.name == name)
    }

    /// A key name typed into a pin. Keys must be literal so the Rust backend can write
    /// `KeyCode::Space`, and must name a real `KeyCode`.
    /// The mouse button typed into `pin`, checked against Bevy's `MouseButton` names.
    fn mouse_button(&mut self, node: &Node, pin: &str) -> Option<String> {
        if self.graph.is_connected(&PinRef::new(node.id, pin), Side::Input) {
            self.error(node.id, "The button must be typed in, not wired".into());
            return None;
        }
        let name = match node.values.get(pin) {
            Some(Value::Str(s)) => s.trim().to_string(),
            _ => String::new(),
        };
        if MOUSE_BUTTONS.contains(&name.as_str()) {
            Some(name)
        } else {
            self.error(node.id, format!("`{name}` isn't a mouse button. Use Left, Right, Middle, Back or Forward"));
            None
        }
    }

    fn key(&mut self, node: &Node, pin: &str) -> Option<String> {
        if self.graph.is_connected(&PinRef::new(node.id, pin), Side::Input) {
            self.error(node.id, "The key must be typed in, not wired".into());
            return None;
        }
        let key = match node.values.get(pin) {
            Some(Value::Str(s)) => s.trim().to_string(),
            _ => String::new(),
        };
        if self.keys.contains(&key.as_str()) {
            Some(key)
        } else {
            self.error(node.id, format!("`{key}` isn't a key. Use Bevy's KeyCode names, such as Space, KeyW or ArrowUp"));
            None
        }
    }

    /// Lowers the steps reached through the exec output `from`. `stack` holds the nodes on the
    /// current path, so a wire loop is reported instead of recursing forever. A straight run of
    /// statements is a loop, not recursion: only branches nest, so long scripts don't need a
    /// deep stack.
    fn chain(&mut self, from: &PinRef, out: &mut Vec<Step>, stack: &mut Vec<NodeId>) {
        let base = stack.len();
        let mut from = from.clone();
        loop {
            let Some(link) = self.graph.links_from(&from).next() else { break };
            let Some(node) = self.graph.node(link.to.node) else { break };
            if stack.contains(&node.id) {
                self.error(node.id, "Execution wires form a loop".into());
                break;
            }
            stack.push(node.id);
            self.reached.insert(node.id);
            match self.statement(node, out, stack) {
                Some(next) => from = next,
                None => break,
            }
        }
        stack.truncate(base);
    }

    /// Lowers one statement into `out`. Returns the exec output the run continues from, if any.
    fn statement(&mut self, node: &Node, out: &mut Vec<Step>, stack: &mut Vec<NodeId>) -> Option<PinRef> {
        let id = node.id;
        let comment = label(node);
        let mut lets = Vec::new();
        let mut cache = Cache::new();
        let action = match node.kind.as_str() {
            "flow.branch" => {
                let cond = self.input(node, "condition", &mut lets, &mut cache);
                let mut then = Vec::new();
                let mut otherwise = Vec::new();
                self.chain(&PinRef::new(id, "true"), &mut then, stack);
                self.chain(&PinRef::new(id, "false"), &mut otherwise, stack);
                out.push(Step { comment, lets, action: Action::If { cond, then, otherwise } });
                return None;
            }
            "flow.sequence" => {
                self.chain(&PinRef::new(id, "first"), out, stack);
                return Some(PinRef::new(id, "second"));
            }
            "flow.flipflop" => {
                // state = not state; if state then a else b
                let slot = self.state(node);
                let flipped = self.call("logic.not", vec![Operand::State(slot)], &mut lets);
                out.push(Step { comment, lets, action: Action::SetState { slot, value: flipped } });
                let mut then = Vec::new();
                let mut otherwise = Vec::new();
                self.chain(&PinRef::new(id, "a"), &mut then, stack);
                self.chain(&PinRef::new(id, "b"), &mut otherwise, stack);
                let cond = Operand::State(slot);
                out.push(Step { comment: String::new(), lets: Vec::new(), action: Action::If { cond, then, otherwise } });
                return None;
            }
            "flow.do_once" => {
                // if not state then state = true; ... end
                let slot = self.state(node);
                let cond = self.call("logic.not", vec![Operand::State(slot)], &mut lets);
                let set = Action::SetState { slot, value: Operand::Lit(Lit::Bool(true)) };
                let mut then = vec![Step { comment: String::new(), lets: Vec::new(), action: set }];
                self.chain(&PinRef::new(id, "then"), &mut then, stack);
                out.push(Step { comment, lets, action: Action::If { cond, then, otherwise: Vec::new() } });
                return None;
            }
            "entity.despawn" => Action::Despawn,
            "action.print" => {
                let text = self.input(node, "value", &mut lets, &mut cache);
                Action::Effect { op: EffectOp::Print, args: vec![text] }
            }
            "transform.translate" | "transform.rotate" | "transform.set_position" | "transform.set_scale" => {
                let op = match node.kind.as_str() {
                    "transform.translate" => EffectOp::Translate,
                    "transform.rotate" => EffectOp::Rotate,
                    "transform.set_position" => EffectOp::SetPosition,
                    _ => EffectOp::SetScale,
                };
                let args = ["x", "y", "z"].map(|p| self.input(node, p, &mut lets, &mut cache)).to_vec();
                Action::Effect { op, args }
            }
            "component.set" => {
                let value = self.input(node, "value", &mut lets, &mut cache);
                match self.field(node) {
                    Some((component, path, _)) => Action::SetField { component, path, value },
                    // Already reported as an unknown field.
                    None => return Some(PinRef::new(id, "then")),
                }
            }
            "var.set" => {
                let value = self.input(node, "value", &mut lets, &mut cache);
                match self.var(node) {
                    Some(var) => Action::SetVar { var, value },
                    // Already reported as a missing variable.
                    None => return Some(PinRef::new(id, "then")),
                }
            }
            _ => {
                self.error(id, format!("{} can't be used as a statement", title_of(node)));
                return None;
            }
        };
        out.push(Step { comment, lets, action });
        Some(PinRef::new(id, "then"))
    }

    /// A call with a generated name, for helper calls (conversions, state flips).
    fn call(&mut self, node: &'static str, args: Vec<Operand>, lets: &mut Vec<Let>) -> Operand {
        let name = node.rsplit('.').next().unwrap_or(node).to_string();
        self.call_named(node, args, lets, name, false)
    }

    fn call_named(&mut self, node: &'static str, args: Vec<Operand>, lets: &mut Vec<Let>, name: String, named: bool) -> Operand {
        let local = self.next_local;
        self.next_local += 1;
        lets.push(Let { local, node, args, name, named });
        Operand::Local(local)
    }

    /// The operand feeding an input: the linked output, or the inline value. `stack` holds the
    /// data nodes being built, to report loops.
    fn input(&mut self, node: &Node, pin: &str, lets: &mut Vec<Let>, cache: &mut Cache) -> Operand {
        self.input_in(node, pin, lets, cache, &mut Vec::new())
    }

    fn input_in(&mut self, node: &Node, pin: &str, lets: &mut Vec<Let>, cache: &mut Cache, stack: &mut Vec<NodeId>) -> Operand {
        let target = PinRef::new(node.id, pin);
        let to_ty = self.graph.pin(&target, Side::Input).map_or(PinType::Any, |p| p.ty);
        if let Some(link) = self.graph.links_to(&target).next().cloned() {
            let from_ty = self.graph.pin(&link.from, Side::Output).map(|p| p.ty);
            let value = self.output(&link.from, lets, cache, stack);
            // Numbers and booleans feeding a text input go through the same conversion nodes in
            // both backends, so `0.1` prints the same everywhere.
            let conversion = match from_ty {
                Some(PinType::Number) => Some("text.from_number"),
                Some(PinType::Bool) => Some("text.from_bool"),
                Some(PinType::Vec3) => Some("text.from_vec3"),
                Some(PinType::Quat) => Some("text.from_quat"),
                Some(PinType::Color) => Some("text.from_color"),
                _ => None,
            };
            return match (to_ty, conversion) {
                (PinType::String, Some(kind)) => self.call(kind, vec![value], lets),
                _ => value,
            };
        }
        let lit = match node.values.get(pin).map(lit_of) {
            Some(lit) if fits(&lit, to_ty) => lit,
            _ => default_lit(to_ty),
        };
        Operand::Lit(lit)
    }

    /// The operand for a data output.
    fn output(&mut self, pin: &PinRef, lets: &mut Vec<Let>, cache: &mut Cache, stack: &mut Vec<NodeId>) -> Operand {
        if let Some(op) = cache.get(pin) {
            return op.clone();
        }
        let Some(node) = self.graph.node(pin.node) else { return Operand::Lit(Lit::Number(0.0)) };
        if stack.contains(&node.id) {
            self.error(node.id, "Data wires form a loop".into());
            return Operand::Lit(Lit::Number(0.0));
        }
        stack.push(node.id);
        let op = match (node.kind.as_str(), pin.pin.as_str()) {
            ("event.update", "delta") => Operand::Intrinsic(Intrinsic::Delta),
            ("flow.flipflop", "is_a") => Operand::State(self.state(node)),
            ("transform.position", axis) => {
                let axis = match axis {
                    "x" => Axis::X,
                    "y" => Axis::Y,
                    _ => Axis::Z,
                };
                Operand::Intrinsic(Intrinsic::Position(axis))
            }
            (kind @ ("input.key_down" | "input.key_pressed" | "input.key_released"), _) => match self.key(node, "key") {
                Some(name) => Operand::Intrinsic(Intrinsic::Button { device: Device::Key, press: press_of(kind), name }),
                None => Operand::Lit(Lit::Bool(false)),
            },
            (kind @ ("input.mouse_down" | "input.mouse_pressed" | "input.mouse_released"), _) => {
                match self.mouse_button(node, "button") {
                    Some(name) => Operand::Intrinsic(Intrinsic::Button { device: Device::Mouse, press: press_of(kind), name }),
                    None => Operand::Lit(Lit::Bool(false)),
                }
            }
            ("time.elapsed", _) => Operand::Intrinsic(Intrinsic::Time),
            ("component.get" | "component.set", "value") => match self.field(node) {
                Some((component, path, ty)) => Operand::Field { component, path, ty },
                None => Operand::Lit(Lit::Number(0.0)),
            },
            ("var.get" | "var.set", "value") => match self.var(node) {
                Some(var) => Operand::Var(var),
                None => Operand::Lit(Lit::Number(0.0)),
            },
            (kind, _) => match self.pure.iter().find(|n| n.kind == kind).cloned() {
                Some(pure) => {
                    let mut args = Vec::new();
                    for (name, _) in &pure.inputs {
                        args.push(self.input_in(node, name, lets, cache, stack));
                    }
                    let (name, named) = match &node.name {
                        Some(name) => (name.clone(), true),
                        None => (snake_case(&title_of(node)), false),
                    };
                    self.call_named(pure.kind, args, lets, name, named)
                }
                None => {
                    self.error(node.id, format!("{} has no output `{}`", title_of(node), pin.pin));
                    Operand::Lit(Lit::Number(0.0))
                }
            },
        };
        stack.pop();
        cache.insert(pin.clone(), op.clone());
        op
    }
}

/// `MouseButton` variants a node can name (`Other(n)` has no use in a script yet).
const MOUSE_BUTTONS: [&str; 5] = ["Left", "Right", "Middle", "Back", "Forward"];

/// Down, just pressed or just released, from an input node's kind.
fn press_of(kind: &str) -> Press {
    if kind.ends_with("_pressed") {
        Press::Pressed
    } else if kind.ends_with("_released") {
        Press::Released
    } else {
        Press::Down
    }
}
