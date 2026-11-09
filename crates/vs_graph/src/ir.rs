//! The intermediate representation both backends read.
//!
//! The IR has Rust semantics and fixes everything a backend could otherwise decide differently:
//! evaluation order (every node call is its own `Let`, in order), types (numbers are `f32`), and
//! where state lives. Backends only spell it out, so the Lua preview and the generated Rust do the
//! same work in the same order.
//!
//! Arithmetic never appears here: a `Let` calls a pure node by kind, and both backends call the
//! same compiled function for it (`vs_nodes::math::add`, or its Lua binding `node.math.add`).

use crate::library::PinType;

/// A literal. Numbers compare by bits, so `NaN == NaN` and `0.0 != -0.0`.
#[derive(Clone, Debug)]
pub enum Lit {
    Number(f32),
    Bool(bool),
    Text(String),
    Vec3([f32; 3]),
    /// A rotation from Euler angles in degrees (`vs_nodes::fx::euler`). Both backends build it
    /// with the same function, so it's kept as angles and reads that way in generated code.
    Quat([f32; 3]),
    /// An sRGB color with alpha.
    Color([f32; 4]),
}

fn same_bits(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

impl PartialEq for Lit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Lit::Number(a), Lit::Number(b)) => a.to_bits() == b.to_bits(),
            (Lit::Bool(a), Lit::Bool(b)) => a == b,
            (Lit::Text(a), Lit::Text(b)) => a == b,
            (Lit::Vec3(a), Lit::Vec3(b)) | (Lit::Quat(a), Lit::Quat(b)) => same_bits(a, b),
            (Lit::Color(a), Lit::Color(b)) => same_bits(a, b),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Key,
    Mouse,
}

impl Device {
    /// The name the Lua runtime uses.
    pub fn name(self) -> &'static str {
        match self {
            Device::Key => "key",
            Device::Mouse => "mouse",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// Held this frame.
    Down,
    /// Went down this frame.
    Pressed,
    /// Came up this frame.
    Released,
}

impl Press {
    /// The name the Lua runtime uses.
    pub fn name(self) -> &'static str {
        match self {
            Press::Down => "down",
            Press::Pressed => "pressed",
            Press::Released => "released",
        }
    }

    /// The `ButtonInput` method that asks for it.
    pub fn method(self) -> &'static str {
        match self {
            Press::Down => "pressed",
            Press::Pressed => "just_pressed",
            Press::Released => "just_released",
        }
    }
}

/// What a script reads from its frame or its entity.
#[derive(Clone, Debug, PartialEq)]
pub enum Intrinsic {
    /// Seconds since the last frame.
    Delta,
    /// Seconds since the app started.
    Time,
    /// A key or mouse button's state, by `KeyCode` / `MouseButton` variant name.
    Button { device: Device, press: Press, name: String },
    /// The entity's position on one axis.
    Position(Axis),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Lit(Lit),
    /// The result of an earlier `Let` in the same function.
    Local(u32),
    /// A script variable, by index into [`Module::variables`].
    Var(usize),
    /// A state slot (a Flip Flop or a Do Once), by index.
    State(usize),
    Intrinsic(Intrinsic),
    /// A field of one of the entity's components: by index into [`Module::components`], and
    /// the reflection path inside it (`intensity`).
    Field { component: usize, path: String, ty: PinType },
}

/// `let local = node(args)`: a call to a pure node, by kind (such as `math.add`).
#[derive(Clone, Debug, PartialEq)]
pub struct Let {
    pub local: u32,
    pub node: &'static str,
    pub args: Vec<Operand>,
    /// A readable name for the result: the node's name if it has one, else its title in
    /// snake case (`multiply`).
    pub name: String,
    /// Whether the user named the node, so exported Rust keeps the value in its own `let`.
    pub named: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOp {
    Translate,
    Rotate,
    SetPosition,
    SetScale,
    Print,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    If { cond: Operand, then: Vec<Step>, otherwise: Vec<Step> },
    SetVar { var: usize, value: Operand },
    SetState { slot: usize, value: Operand },
    /// Removes the entity (and its children) at the end of the frame.
    Despawn,
    SetField { component: usize, path: String, value: Operand },
    /// Translate/Rotate/SetPosition/SetScale take x, y, z; Print takes one text.
    Effect { op: EffectOp, args: Vec<Operand> },
}

/// One action and the node calls that feed it. Data nodes are evaluated fresh for each step, so
/// a step sees the effects of the steps before it.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// Which graph node this comes from, such as `Rotate [4]`. Empty when it continues the step
    /// before it.
    pub comment: String,
    pub lets: Vec<Let>,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Once, on the script's first frame, before the update events.
    Start,
    /// Every frame.
    Update,
    /// On the frame a key goes down, by `KeyCode` variant name.
    Key(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub comment: String,
    /// The event node's name, if it has one. Exported Rust names the event's system after it.
    pub name: Option<String>,
    pub event: Event,
    pub body: Vec<Step>,
}

/// A component the script reads or writes. Its own component requires it.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentUse {
    /// The short name, such as `PointLight`.
    pub name: String,
    /// How Rust names the type.
    pub rust_path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    pub name: String,
    pub initial: Lit,
}

/// A whole script. Each Start block runs once on the first frame; then, every frame, the Update
/// and Key blocks run in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    pub variables: Vec<Variable>,
    /// The script's boolean state slots, by name hint (`flip_flop`, or the node's name). They
    /// start `false`.
    pub states: Vec<String>,
    /// The components the script reads or writes, `Transform` included.
    pub components: Vec<ComponentUse>,
    pub blocks: Vec<Block>,
}

impl Module {
    pub fn has_start(&self) -> bool {
        self.blocks.iter().any(|b| b.event == Event::Start)
    }

    pub fn has_update(&self) -> bool {
        self.blocks.iter().any(|b| b.event != Event::Start)
    }
}
