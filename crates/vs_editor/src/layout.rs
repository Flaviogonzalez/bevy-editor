//! Node geometry in graph units, shared by drawing and hit testing so both always agree.

use vs_graph::{Graph, Node, NodeId, PinDef, PinRef, PinType, Side};

pub const NODE_W: f32 = 210.0;
pub const HEADER_H: f32 = 26.0;
pub const ROW_H: f32 = 22.0;
pub const PAD_BOTTOM: f32 = 8.0;
pub const PIN_R: f32 = 5.0;
/// How close, in graph units, the pointer has to be to grab a pin.
pub const PIN_GRAB: f32 = 10.0;
pub const FIELD_X: f32 = 72.0;
pub const FIELD_W: f32 = 64.0;
pub const FIELD_H: f32 = 16.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dist(self, other: V2) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}

impl std::ops::Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        V2::new(self.x * s, self.y * s)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub min: V2,
    pub max: V2,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { min: V2::new(x, y), max: V2::new(x + w, y + h) }
    }

    pub fn from_points(a: V2, b: V2) -> Self {
        Self {
            min: V2::new(a.x.min(b.x), a.y.min(b.y)),
            max: V2::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.min.x <= o.max.x && self.max.x >= o.min.x && self.min.y <= o.max.y && self.max.y >= o.min.y
    }

    pub fn union(&self, o: &Rect) -> Rect {
        Rect {
            min: V2::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            max: V2::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        }
    }

    pub fn w(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn h(&self) -> f32 {
        self.max.y - self.min.y
    }
}

#[derive(Clone, Debug)]
pub struct PinLayout {
    pub def: PinDef,
    pub center: V2,
    pub connected: bool,
}

#[derive(Clone, Debug)]
pub struct NodeLayout {
    pub id: NodeId,
    pub rect: Rect,
    pub inputs: Vec<PinLayout>,
    pub outputs: Vec<PinLayout>,
    /// Inline value fields of unconnected data inputs: (pin name, rect).
    pub fields: Vec<(String, Rect)>,
}

impl NodeLayout {
    pub fn header(&self) -> Rect {
        Rect::new(self.rect.min.x, self.rect.min.y, self.rect.w(), HEADER_H)
    }

    pub fn pin_center(&self, pin: &str, side: Side) -> Option<V2> {
        let list = match side {
            Side::Input => &self.inputs,
            Side::Output => &self.outputs,
        };
        list.iter().find(|p| p.def.name == pin).map(|p| p.center)
    }
}

pub fn layout(graph: &Graph, node: &Node) -> NodeLayout {
    let pins = graph.pins_of(node);
    let rows = pins.inputs.len().max(pins.outputs.len()).max(1);
    let h = HEADER_H + rows as f32 * ROW_H + PAD_BOTTOM;
    let rect = Rect::new(node.x, node.y, NODE_W, h);
    let row_y = |i: usize| node.y + HEADER_H + i as f32 * ROW_H + ROW_H * 0.5;

    let mut fields = Vec::new();
    let inputs = pins
        .inputs
        .into_iter()
        .enumerate()
        .map(|(i, def)| {
            let connected = graph.is_connected(&PinRef::new(node.id, def.name.clone()), Side::Input);
            if !connected && def.ty != PinType::Exec && node.values.contains_key(&def.name) {
                // Vectors, rotations and colors hold three or four numbers: give them the room up
                // to the output pins.
                let w = match def.ty {
                    PinType::Vec3 | PinType::Quat | PinType::Color => NODE_W - FIELD_X - 14.0,
                    _ => FIELD_W,
                };
                fields.push((def.name.clone(), Rect::new(node.x + FIELD_X, row_y(i) - FIELD_H * 0.5, w, FIELD_H)));
            }
            PinLayout { def, center: V2::new(node.x, row_y(i)), connected }
        })
        .collect();
    let outputs = pins
        .outputs
        .into_iter()
        .enumerate()
        .map(|(i, def)| {
            let connected =
                graph.is_connected(&PinRef::new(node.id, def.name.clone()), Side::Output);
            PinLayout { def, center: V2::new(node.x + NODE_W, row_y(i)), connected }
        })
        .collect();

    NodeLayout { id: node.id, rect, inputs, outputs, fields }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Pin(PinRef, Side),
    Field(NodeId, String),
    Header(NodeId),
    Body(NodeId),
    Empty,
}

/// What lies under a graph-space point. Later nodes draw on top, so they're tested first.
pub fn hit_test(layouts: &[NodeLayout], p: V2) -> Hit {
    for l in layouts.iter().rev() {
        for (side, pins) in [(Side::Input, &l.inputs), (Side::Output, &l.outputs)] {
            for pin in pins {
                if pin.center.dist(p) <= PIN_GRAB {
                    return Hit::Pin(PinRef::new(l.id, pin.def.name.clone()), side);
                }
            }
        }
        if !l.rect.contains(p) {
            continue;
        }
        for (pin, rect) in &l.fields {
            if rect.contains(p) {
                return Hit::Field(l.id, pin.clone());
            }
        }
        if l.header().contains(p) {
            return Hit::Header(l.id);
        }
        return Hit::Body(l.id);
    }
    Hit::Empty
}
