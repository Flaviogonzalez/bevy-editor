//! Node geometry in graph units, shared by drawing and hit testing so both always agree.

use vs_graph::{ComponentDef, Graph, Node, NodeId, PinDef, PinRef, PinType, Side, Value, Variable, library};

pub const NODE_W: f32 = 210.0;
pub const HEADER_H: f32 = 30.0;
/// Space between the header and the first pin row.
pub const PAD_TOP: f32 = 4.0;
pub const ROW_H: f32 = 24.0;
pub const PAD_BOTTOM: f32 = 8.0;
pub const PIN_R: f32 = 5.0;
/// How close, in graph units, the pointer has to be to grab a pin.
pub const PIN_GRAB: f32 = 10.0;
pub const FIELD_X: f32 = 72.0;
pub const FIELD_W: f32 = 64.0;
pub const FIELD_H: f32 = 18.0;
/// A component card is wider than a node: each row holds a name, a value and a type.
pub const CARD_W: f32 = 270.0;
/// The footer of an event node, where the query of the system it compiles to is shown.
pub const SYSTEM_H: f32 = 36.0;

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
    /// The parts of a component card, or of a read-only copy; empty for other nodes.
    pub parts: Vec<(CardPart, Rect)>,
    /// A read-only copy: of its parts, only [`CardPart::Source`] takes clicks, so pressing
    /// anywhere else on it moves it like any node.
    pub readonly: bool,
    /// An event node's footer, which shows its system's query. It takes clicks like the body.
    pub system: Option<Rect>,
}

/// A clickable part of a component card. Field indices are positions in its `fields`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardPart {
    Name,
    FieldName(usize),
    Value(usize),
    /// The type label: clicking it switches the field to the next type.
    Type(usize),
    Remove(usize),
    AddField,
    /// On a read-only copy: where the component is declared. Clicking it opens that script.
    Source,
}

/// What a read-only copy of `name` shows: the component as this script sees it (its own card,
/// then the catalog), with each field's starting value, and the file that declares it. A
/// component nothing declares shows no fields.
pub fn copy_view(graph: &Graph, name: &str) -> (ComponentDef, Option<String>) {
    if let Some(def) = graph.defined_components().find(|d| d.name == name) {
        return (def.clone(), None);
    }
    let Some(info) = vs_graph::components::find(name) else {
        return (ComponentDef { name: name.to_string(), fields: Vec::new() }, None);
    };
    let fields = info
        .fields
        .iter()
        .filter_map(|f| {
            let value = f.default.clone().or_else(|| Value::default_for(f.ty))?;
            Some(Variable { name: f.path.clone(), value })
        })
        .collect();
    (ComponentDef { name: info.name, fields }, info.source)
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
    if let Some(def) = &node.component {
        return card_layout(node, def, false);
    }
    if let Some(name) = &node.copy_of {
        return card_layout(node, &copy_view(graph, name).0, true);
    }
    let pins = graph.pins_of(node);
    let rows = pins.inputs.len().max(pins.outputs.len()).max(1);
    let body = HEADER_H + PAD_TOP + rows as f32 * ROW_H + PAD_BOTTOM;
    let event = library::find(&node.kind).is_some_and(|d| d.is_event());
    let system = event.then(|| Rect::new(node.x, node.y + body, NODE_W, SYSTEM_H));
    let h = body + if event { SYSTEM_H } else { 0.0 };
    let rect = Rect::new(node.x, node.y, NODE_W, h);
    let row_y = |i: usize| node.y + HEADER_H + PAD_TOP + i as f32 * ROW_H + ROW_H * 0.5;

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

    NodeLayout { id: node.id, rect, inputs, outputs, fields, parts: Vec::new(), readonly: false, system }
}

/// A component card: the name, a row per field (name, value, type, remove) and a row to add a
/// field. It has no pins, since it's data rather than logic. A read-only copy has the same rows
/// without the buttons, and the link to its script beside the name.
fn card_layout(node: &Node, def: &ComponentDef, readonly: bool) -> NodeLayout {
    let rows = def.fields.len() + if readonly { 1 } else { 2 };
    let h = HEADER_H + PAD_TOP + rows as f32 * ROW_H + PAD_BOTTOM;
    let rect = Rect::new(node.x, node.y, CARD_W, h);
    let top = |row: usize| node.y + HEADER_H + PAD_TOP + row as f32 * ROW_H + (ROW_H - FIELD_H) * 0.5;
    let x = |dx: f32| node.x + dx;
    let mut parts = if readonly {
        vec![
            (CardPart::Name, Rect::new(x(10.0), top(0), 110.0, FIELD_H)),
            (CardPart::Source, Rect::new(x(124.0), top(0), CARD_W - 134.0, FIELD_H)),
        ]
    } else {
        vec![(CardPart::Name, Rect::new(x(10.0), top(0), CARD_W - 20.0, FIELD_H))]
    };
    for i in 0..def.fields.len() {
        let y = top(i + 1);
        parts.push((CardPart::FieldName(i), Rect::new(x(10.0), y, 82.0, FIELD_H)));
        parts.push((CardPart::Value(i), Rect::new(x(96.0), y, 108.0, FIELD_H)));
        parts.push((CardPart::Type(i), Rect::new(x(208.0), y, 36.0, FIELD_H)));
        if !readonly {
            parts.push((CardPart::Remove(i), Rect::new(x(248.0), y, 14.0, FIELD_H)));
        }
    }
    if !readonly {
        parts.push((CardPart::AddField, Rect::new(x(10.0), top(def.fields.len() + 1), 90.0, FIELD_H)));
    }
    NodeLayout { id: node.id, rect, inputs: Vec::new(), outputs: Vec::new(), fields: Vec::new(), parts, readonly, system: None }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Pin(PinRef, Side),
    Field(NodeId, String),
    Card(NodeId, CardPart),
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
        for (part, rect) in &l.parts {
            if rect.contains(p) && (!l.readonly || *part == CardPart::Source) {
                return Hit::Card(l.id, *part);
            }
        }
        if l.header().contains(p) {
            return Hit::Header(l.id);
        }
        return Hit::Body(l.id);
    }
    Hit::Empty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_shows_the_card_it_copies_and_only_its_link_takes_clicks() {
        let mut g = Graph::default();
        let fields = vec![Variable { name: "max".into(), value: Value::Number(5.0) }];
        g.add_component_def(ComponentDef { name: "Health".into(), fields }, 0.0, 0.0);
        let copy = g.add_component_copy("Health", 0.0, 300.0);
        let (view, source) = copy_view(&g, "Health");
        assert_eq!(view.fields[0].value, Value::Number(5.0));
        assert_eq!(source, None);
        let l = layout(&g, g.node(copy).unwrap());
        assert!(l.readonly);
        assert!(!l.parts.iter().any(|(p, _)| matches!(p, CardPart::Remove(_) | CardPart::AddField)));
        let rect = |part: CardPart| l.parts.iter().find(|(p, _)| *p == part).unwrap().1;
        let center = |r: Rect| V2::new(r.min.x + r.w() * 0.5, r.min.y + r.h() * 0.5);
        let layouts = [l.clone()];
        assert_eq!(hit_test(&layouts, center(rect(CardPart::Value(0)))), Hit::Body(copy));
        assert_eq!(hit_test(&layouts, center(rect(CardPart::Source))), Hit::Card(copy, CardPart::Source));
        // Nothing declares it: no fields, and no file to go to.
        let (empty, source) = copy_view(&g, "Nothing");
        assert!(empty.fields.is_empty() && source.is_none());
    }
}
