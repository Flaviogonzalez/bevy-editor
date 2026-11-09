//! The graph model: nodes, links and variables. This is what gets saved inside the script file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::library::{self, PinDef, PinType};

pub type NodeId = u32;

/// A literal value: the inline value of an unconnected input, or a variable's initial value.
/// Rotations are kept as the Euler angles the user typed, in degrees, and colors as sRGB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Str(String),
    Vec3([f64; 3]),
    Quat { euler: [f64; 3] },
    Color { rgba: [f64; 4] },
}

impl Value {
    pub fn default_for(ty: PinType) -> Option<Value> {
        match ty {
            PinType::Number => Some(Value::Number(0.0)),
            PinType::Bool => Some(Value::Bool(false)),
            PinType::String => Some(Value::Str(String::new())),
            PinType::Vec3 => Some(Value::Vec3([0.0; 3])),
            PinType::Quat => Some(Value::Quat { euler: [0.0; 3] }),
            PinType::Color => Some(Value::Color { rgba: [1.0; 4] }),
            PinType::Exec | PinType::Any => None,
        }
    }

    pub fn ty(&self) -> PinType {
        match self {
            Value::Bool(_) => PinType::Bool,
            Value::Number(_) => PinType::Number,
            Value::Str(_) => PinType::String,
            Value::Vec3(_) => PinType::Vec3,
            Value::Quat { .. } => PinType::Quat,
            Value::Color { .. } => PinType::Color,
        }
    }

    /// The value as it is shown in an inline field: `1, 2, 3` for vectors and rotations,
    /// `#rrggbb` (or `#rrggbbaa`) for colors that have an exact hex form.
    pub fn display(&self) -> String {
        let list = |v: &[f64]| v.iter().map(|n| format_number(*n)).collect::<Vec<_>>().join(", ");
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => format_number(*n),
            Value::Str(s) => s.clone(),
            Value::Vec3(v) | Value::Quat { euler: v } => list(v),
            Value::Color { rgba } => hex(rgba).unwrap_or_else(|| list(rgba)),
        }
    }

    /// Parses text typed in an inline field into a value of the same kind as `self`. Vectors and
    /// rotations take three numbers; colors take `#rrggbb[aa]` or three or four numbers from 0
    /// to 1.
    pub fn parse_like(&self, text: &str) -> Option<Value> {
        let text = text.trim();
        let numbers = || -> Option<Vec<f64>> {
            text.split(|c: char| c == ',' || c.is_whitespace())
                .filter(|p| !p.is_empty())
                .map(|p| p.parse().ok())
                .collect()
        };
        match self {
            Value::Bool(_) => text.parse().ok().map(Value::Bool),
            Value::Number(_) => text.parse().ok().map(Value::Number),
            Value::Str(_) => Some(Value::Str(text.to_string())),
            Value::Vec3(_) => numbers()?.try_into().ok().map(Value::Vec3),
            Value::Quat { .. } => numbers()?.try_into().ok().map(|euler| Value::Quat { euler }),
            Value::Color { .. } => {
                if let Some(hex) = text.strip_prefix('#') {
                    return parse_hex(hex).map(|rgba| Value::Color { rgba });
                }
                match numbers()?.as_slice() {
                    [r, g, b] => Some(Value::Color { rgba: [*r, *g, *b, 1.0] }),
                    [r, g, b, a] => Some(Value::Color { rgba: [*r, *g, *b, *a] }),
                    _ => None,
                }
            }
        }
    }
}

/// `#rrggbb` or `#rrggbbaa`, when every component is a whole number of 255ths.
fn hex(rgba: &[f64; 4]) -> Option<String> {
    let bytes: Option<Vec<u8>> = rgba
        .iter()
        .map(|c| {
            let b = (c * 255.0).round();
            ((0.0..=255.0).contains(&b) && b / 255.0 == *c).then_some(b as u8)
        })
        .collect();
    let bytes = bytes?;
    let mut s = format!("#{:02x}{:02x}{:02x}", bytes[0], bytes[1], bytes[2]);
    if bytes[3] != 255 {
        s.push_str(&format!("{:02x}", bytes[3]));
    }
    Some(s)
}

fn parse_hex(hex: &str) -> Option<[f64; 4]> {
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok().map(|b| b as f64 / 255.0);
    Some([byte(0)?, byte(2)?, byte(4)?, if hex.len() == 8 { byte(6)? } else { 1.0 }])
}

pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    /// Inline values for inputs, keyed by pin name. Used only while the input is unconnected.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
    /// The variable a Get / Set node refers to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub var: Option<String>,
    /// The component field a Get / Set Component node refers to, such as `PointLight.intensity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// An optional name. Exported Rust uses it for the value or state the node produces (a
    /// Flip Flop named `big` becomes a `big` field), and keeps a named value in its own `let`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// One end of a link.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PinRef {
    pub node: NodeId,
    pub pin: String,
}

impl PinRef {
    pub fn new(node: NodeId, pin: impl Into<String>) -> Self {
        Self { node, pin: pin.into() }
    }
}

/// A link always goes from an output pin to an input pin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub from: PinRef,
    pub to: PinRef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub version: u32,
    pub next_id: NodeId,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Default for Graph {
    fn default() -> Self {
        Self { version: 1, next_id: 1, variables: Vec::new(), nodes: Vec::new(), links: Vec::new() }
    }
}

/// The resolved pins of a node. Most come straight from the library; variable nodes get the
/// variable's type.
#[derive(Clone, Debug)]
pub struct Pins {
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Input,
    Output,
}

impl Graph {
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn variable(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    pub fn var_type(&self, name: &str) -> PinType {
        self.variable(name).map_or(PinType::Any, |v| v.value.ty())
    }

    /// Adds a node of `kind` at a graph position and returns its id. Inline values start at the
    /// library defaults.
    pub fn add_node(&mut self, kind: &str, var: Option<String>, x: f32, y: f32) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let mut node =
            Node { id, kind: kind.to_string(), x, y, values: BTreeMap::new(), var, field: None, name: None };
        let pins = self.pins_of(&node);
        for pin in pins.inputs {
            if let Some(value) = pin.default.clone().or_else(|| Value::default_for(pin.ty)) {
                node.values.insert(pin.name.clone(), value);
            }
        }
        self.nodes.push(node);
        id
    }

    /// Adds a Get / Set Component node for `field` (`PointLight.intensity`).
    pub fn add_field_node(&mut self, kind: &str, field: &str, x: f32, y: f32) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let mut node = Node {
            id,
            kind: kind.to_string(),
            x,
            y,
            values: BTreeMap::new(),
            var: None,
            field: Some(field.to_string()),
            name: None,
        };
        for pin in self.pins_of(&node).inputs {
            if let Some(value) = pin.default.clone().or_else(|| Value::default_for(pin.ty)) {
                node.values.insert(pin.name.clone(), value);
            }
        }
        self.nodes.push(node);
        id
    }

    pub fn pins_of(&self, node: &Node) -> Pins {
        let Some(def) = library::find(&node.kind) else {
            return Pins { inputs: Vec::new(), outputs: Vec::new() };
        };
        let mut pins = Pins { inputs: def.inputs.clone(), outputs: def.outputs.clone() };
        let resolved = match (&node.var, &node.field) {
            (Some(var), _) => Some(self.var_type(var)),
            (None, Some(field)) => Some(crate::components::field(field).map_or(PinType::Any, |(_, f)| f.ty)),
            (None, None) => None,
        };
        if let Some(ty) = resolved {
            for pin in pins.inputs.iter_mut().chain(pins.outputs.iter_mut()) {
                if pin.ty == PinType::Any {
                    pin.ty = ty;
                }
            }
        }
        pins
    }

    pub fn pin(&self, pin: &PinRef, side: Side) -> Option<PinDef> {
        let node = self.node(pin.node)?;
        let pins = self.pins_of(node);
        let list = match side {
            Side::Input => pins.inputs,
            Side::Output => pins.outputs,
        };
        list.into_iter().find(|p| p.name == pin.pin)
    }

    pub fn links_to(&self, pin: &PinRef) -> impl Iterator<Item = &Link> {
        self.links.iter().filter(move |l| &l.to == pin)
    }

    pub fn links_from(&self, pin: &PinRef) -> impl Iterator<Item = &Link> {
        self.links.iter().filter(move |l| &l.from == pin)
    }

    pub fn is_connected(&self, pin: &PinRef, side: Side) -> bool {
        self.links.iter().any(|l| match side {
            Side::Input => &l.to == pin,
            Side::Output => &l.from == pin,
        })
    }

    /// Whether an output can be wired into an input.
    pub fn can_link(&self, from: &PinRef, to: &PinRef) -> bool {
        if from.node == to.node {
            return false;
        }
        let (Some(out), Some(inp)) = (self.pin(from, Side::Output), self.pin(to, Side::Input))
        else {
            return false;
        };
        out.ty.connects_to(inp.ty)
    }

    /// Wires `from` into `to`. A data input and an exec output hold one link each, so an older
    /// link on them is replaced. Returns false when the pins don't fit.
    pub fn link(&mut self, from: PinRef, to: PinRef) -> bool {
        if !self.can_link(&from, &to) {
            return false;
        }
        let exec = self.pin(&from, Side::Output).is_some_and(|p| p.ty == PinType::Exec);
        if exec {
            self.links.retain(|l| l.from != from);
        } else {
            self.links.retain(|l| l.to != to);
        }
        self.links.push(Link { from, to });
        true
    }

    pub fn remove_nodes(&mut self, ids: &[NodeId]) {
        self.nodes.retain(|n| !ids.contains(&n.id));
        self.links.retain(|l| !ids.contains(&l.from.node) && !ids.contains(&l.to.node));
    }

    /// Copies `ids` with an offset, keeping the links between them. Returns the new ids.
    pub fn duplicate(&mut self, ids: &[NodeId], dx: f32, dy: f32) -> Vec<NodeId> {
        let mut map = BTreeMap::new();
        let copies: Vec<Node> = self.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect();
        let mut new_ids = Vec::new();
        for mut node in copies {
            let id = self.next_id;
            self.next_id += 1;
            map.insert(node.id, id);
            node.id = id;
            node.x += dx;
            node.y += dy;
            new_ids.push(id);
            self.nodes.push(node);
        }
        let links: Vec<Link> = self
            .links
            .iter()
            .filter_map(|l| {
                Some(Link {
                    from: PinRef::new(*map.get(&l.from.node)?, l.from.pin.clone()),
                    to: PinRef::new(*map.get(&l.to.node)?, l.to.pin.clone()),
                })
            })
            .collect();
        self.links.extend(links);
        new_ids
    }

    /// Drops links whose pins no longer exist, such as after a variable changed type.
    pub fn prune_links(&mut self) {
        let valid: Vec<bool> = self.links.iter().map(|l| self.can_link(&l.from, &l.to)).collect();
        let mut i = 0;
        self.links.retain(|_| {
            i += 1;
            valid[i - 1]
        });
    }
}
