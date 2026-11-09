//! The component cards of the open script: adding one, renaming it, and adding, renaming,
//! retyping and removing its fields.
//!
//! A card is a `define.component` node whose `component` holds the declaration, so moving,
//! deleting, copying and undoing it are the ordinary node operations in `mod.rs`. What's here
//! are the edits inside the card. Renames also update this script's Get / Set nodes that point
//! at the old name, the way renaming a variable updates its nodes; other scripts that use the
//! component still name the old one, and lowering reports them when they're opened.
//!
//! Names are normalized rather than refused where possible (`max hp` becomes `max_hp` for a
//! field, `health bar` becomes `HealthBar` for a component). What's left to refuse is an empty
//! or repeated name; whatever else lowering doesn't accept (a Bevy type's name, say) shows up as
//! a problem on the card.

use vs_graph::{ComponentDef, NodeId, PinType, Value, Variable};

use super::Document;
use crate::canvas::layout::V2;

/// The next type when cycling a variable's or a field's type:
/// Number → Bool → String → Vector → Rotation → Color → Number.
pub fn next_type(ty: PinType) -> PinType {
    match ty {
        PinType::Number => PinType::Bool,
        PinType::Bool => PinType::String,
        PinType::String => PinType::Vec3,
        PinType::Vec3 => PinType::Quat,
        PinType::Quat => PinType::Color,
        _ => PinType::Number,
    }
}

/// `health bar` → `HealthBar`, `health` → `Health`. Already PascalCase names stay as they are.
pub fn pascal_case(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            let first = chars.next().expect("not empty").to_uppercase();
            first.chain(chars).collect::<String>()
        })
        .collect()
}

impl Document {
    fn card(&self, id: NodeId) -> Option<&ComponentDef> {
        self.graph.node(id)?.component.as_ref()
    }

    fn card_mut(&mut self, id: NodeId) -> Option<&mut ComponentDef> {
        self.graph.node_mut(id)?.component.as_mut()
    }

    /// A component name this script doesn't declare and the catalog doesn't know: `Component`,
    /// then `Component2`, `Component3`…
    fn free_component_name(&self) -> String {
        let taken = |name: &str| {
            self.graph.defined_components().any(|d| d.name == name) || vs_graph::components::find(name).is_some()
        };
        let mut name = "Component".to_string();
        let mut n = 1;
        while taken(&name) {
            n += 1;
            name = format!("Component{n}");
        }
        name
    }

    /// Adds a card for a new component, with one number field, at `at` in graph units, and
    /// selects it.
    pub fn add_component(&mut self, at: V2) -> NodeId {
        self.checkpoint();
        let def = ComponentDef {
            name: self.free_component_name(),
            fields: vec![Variable { name: "value".into(), value: Value::Number(0.0) }],
        };
        let id = self.graph.add_component_def(def, at.x, at.y);
        self.select_only(id);
        id
    }

    /// Adds a read-only copy of `name`, a component another script declares, at `at` in graph
    /// units, and selects it.
    pub fn add_component_copy(&mut self, name: &str, at: V2) -> NodeId {
        self.checkpoint();
        let id = self.graph.add_component_copy(name, at.x, at.y);
        self.select_only(id);
        id
    }

    /// Gives each card among `ids` that repeats another card's name a free one, so a copied card
    /// declares a new component instead of the same one twice.
    pub(super) fn rename_copied_cards(&mut self, ids: &[NodeId]) {
        for &id in ids {
            let Some(name) = self.card(id).map(|d| d.name.clone()) else { continue };
            if self.graph.defined_components().filter(|d| d.name == name).count() > 1 {
                let free = self.free_component_name();
                if let Some(def) = self.card_mut(id) {
                    def.name = free;
                }
            }
        }
    }

    /// Points this script's Get / Set nodes at `Component.field` from `old` to `new`.
    fn retarget_field_nodes(&mut self, old: &str, new: &str) {
        for n in &mut self.graph.nodes {
            if n.field.as_deref() == Some(old) {
                n.field = Some(new.to_string());
            }
        }
    }

    /// Renames card `id`'s component to `text` in PascalCase. `Ok(false)` when nothing changed.
    pub fn rename_component(&mut self, id: NodeId, text: &str) -> Result<bool, String> {
        let Some(def) = self.card(id) else { return Ok(false) };
        let name = pascal_case(text);
        let old = def.name.clone();
        if name == old {
            return Ok(false);
        }
        if name.is_empty() {
            return Err("A component needs a name".into());
        }
        if self.graph.defined_components().any(|d| d.name == name) {
            return Err(format!("This script already declares `{name}`"));
        }
        let fields: Vec<String> = def.fields.iter().map(|f| f.name.clone()).collect();
        self.checkpoint();
        self.card_mut(id).expect("checked above").name = name.clone();
        for f in fields {
            self.retarget_field_nodes(&format!("{old}.{f}"), &format!("{name}.{f}"));
        }
        Ok(true)
    }

    /// Adds a number field named `field`, `field2`… to card `id`.
    pub fn add_component_field(&mut self, id: NodeId) {
        let Some(def) = self.card(id) else { return };
        let mut name = "field".to_string();
        let mut n = 1;
        while def.fields.iter().any(|f| f.name == name) {
            n += 1;
            name = format!("field{n}");
        }
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields.push(Variable { name, value: Value::Number(0.0) });
    }

    /// Removes field `i` of card `id`, and this script's Get / Set nodes for it.
    pub fn remove_component_field(&mut self, id: NodeId, i: usize) -> bool {
        let Some(def) = self.card(id) else { return false };
        let Some(field) = def.fields.get(i) else { return false };
        let path = format!("{}.{}", def.name, field.name);
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields.remove(i);
        let ids: Vec<NodeId> =
            self.graph.nodes.iter().filter(|n| n.field.as_deref() == Some(path.as_str())).map(|n| n.id).collect();
        self.graph.remove_nodes(&ids);
        self.selected.retain(|s| !ids.contains(s));
        true
    }

    /// Renames field `i` of card `id` to `text` in snake_case. `Ok(false)` when nothing changed.
    pub fn rename_component_field(&mut self, id: NodeId, i: usize, text: &str) -> Result<bool, String> {
        let Some(def) = self.card(id) else { return Ok(false) };
        let Some(old) = def.fields.get(i).map(|f| f.name.clone()) else { return Ok(false) };
        let name = vs_graph::lower::snake_case(text.trim());
        if name == old {
            return Ok(false);
        }
        if name.is_empty() {
            return Err("A field needs a name".into());
        }
        if def.fields.iter().any(|f| f.name == name) {
            return Err(format!("`{}` already has a field named `{name}`", def.name));
        }
        let component = def.name.clone();
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields[i].name = name.clone();
        self.retarget_field_nodes(&format!("{component}.{old}"), &format!("{component}.{name}"));
        Ok(true)
    }

    /// Sets the starting value of field `i` of card `id`, parsed like its current value.
    /// `Ok(false)` when it's the same value.
    pub fn set_component_value(&mut self, id: NodeId, i: usize, text: &str) -> Result<bool, String> {
        let Some(current) = self.card(id).and_then(|d| d.fields.get(i)).map(|f| f.value.clone()) else {
            return Ok(false);
        };
        let value = current.parse_like(text).ok_or_else(|| format!("`{text}` isn't a valid value"))?;
        if value == current {
            return Ok(false);
        }
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields[i].value = value;
        Ok(true)
    }

    /// Flips a bool field's starting value. Other types are edited as text.
    pub fn toggle_component_value(&mut self, id: NodeId, i: usize) {
        let Some(Value::Bool(b)) = self.card(id).and_then(|d| d.fields.get(i)).map(|f| f.value.clone()) else {
            return;
        };
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields[i].value = Value::Bool(!b);
    }

    /// Gives field `i` of card `id` the next type (see [`next_type`]) with that type's default.
    /// This script's Set nodes for it get a fresh inline value, and links that no longer fit go.
    pub fn cycle_component_field_type(&mut self, id: NodeId, i: usize) {
        let Some(def) = self.card(id) else { return };
        let Some(field) = def.fields.get(i) else { return };
        let ty = next_type(field.value.ty());
        let path = format!("{}.{}", def.name, field.name);
        self.checkpoint();
        self.card_mut(id).expect("checked above").fields[i].value =
            Value::default_for(ty).expect("data types have defaults");
        for n in &mut self.graph.nodes {
            if n.field.as_deref() == Some(path.as_str()) && n.kind == "component.set" {
                n.values.insert("value".into(), Value::default_for(ty).expect("data types have defaults"));
            }
        }
        self.graph.prune_links();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A script with a card, and a Get and a Set node for its field.
    fn doc_with_card() -> (Document, NodeId) {
        let mut doc = Document::default();
        let card = doc.add_component(V2::new(0.0, 0.0));
        doc.graph.add_field_node("component.get", "Component.value", 300.0, 0.0);
        doc.graph.add_field_node("component.set", "Component.value", 300.0, 100.0);
        (doc, card)
    }

    fn fields(doc: &Document) -> Vec<Option<String>> {
        doc.graph.nodes.iter().filter(|n| n.component.is_none()).map(|n| n.field.clone()).collect()
    }

    #[test]
    fn names_are_normalized_and_must_be_unique() {
        assert_eq!(pascal_case("health bar"), "HealthBar");
        assert_eq!(pascal_case("HealthBar"), "HealthBar");
        let (mut doc, card) = doc_with_card();
        let other = doc.add_component(V2::new(0.0, 300.0));
        assert_ne!(doc.card(other).unwrap().name, "Component");
        assert_eq!(doc.rename_component(card, "health bar"), Ok(true));
        assert_eq!(doc.card(card).unwrap().name, "HealthBar");
        assert!(doc.rename_component(other, "HealthBar").is_err());
        assert!(doc.rename_component(other, "  ").is_err());
        assert_eq!(doc.rename_component(card, "HealthBar"), Ok(false));
    }

    #[test]
    fn renaming_retargets_this_scripts_field_nodes() {
        let (mut doc, card) = doc_with_card();
        doc.rename_component(card, "Health").unwrap();
        assert_eq!(fields(&doc), vec![Some("Health.value".into()); 2]);
        doc.rename_component_field(card, 0, "max hp").unwrap();
        assert_eq!(doc.card(card).unwrap().fields[0].name, "max_hp");
        assert_eq!(fields(&doc), vec![Some("Health.max_hp".into()); 2]);
        doc.add_component_field(card);
        assert!(doc.rename_component_field(card, 1, "max_hp").is_err());
        // Each rename is one undo step.
        assert!(doc.undo());
        assert!(doc.undo());
        assert_eq!(fields(&doc), vec![Some("Health.value".into()); 2]);
    }

    #[test]
    fn removing_a_field_removes_its_nodes() {
        let (mut doc, card) = doc_with_card();
        doc.add_component_field(card);
        assert_eq!(doc.card(card).unwrap().fields[1].name, "field");
        assert!(doc.remove_component_field(card, 0));
        assert_eq!(doc.graph.nodes.len(), 1);
        assert_eq!(doc.card(card).unwrap().fields.len(), 1);
        assert!(!doc.remove_component_field(card, 5));
    }

    #[test]
    fn retyping_a_field_resets_values_and_drops_links_that_no_longer_fit() {
        let (mut doc, card) = doc_with_card();
        let get = doc.graph.nodes[1].id;
        let print = doc.graph.add_node("math.add", None, 600.0, 0.0);
        assert!(doc.graph.link(vs_graph::PinRef::new(get, "value"), vs_graph::PinRef::new(print, "a")));
        doc.cycle_component_field_type(card, 0);
        assert_eq!(doc.card(card).unwrap().fields[0].value, Value::Bool(false));
        let set = &doc.graph.nodes[2];
        assert_eq!(set.values.get("value"), Some(&Value::Bool(false)));
        assert!(doc.graph.links.is_empty());
        doc.toggle_component_value(card, 0);
        assert_eq!(doc.card(card).unwrap().fields[0].value, Value::Bool(true));
        assert!(doc.set_component_value(card, 0, "nope").is_err());
    }

    #[test]
    fn a_copied_card_gets_its_own_name() {
        let (mut doc, card) = doc_with_card();
        doc.select_only(card);
        doc.duplicate_selected();
        let names: Vec<String> = doc.graph.defined_components().map(|d| d.name.clone()).collect();
        assert_eq!(names, vec!["Component".to_string(), "Component2".to_string()]);
    }
}
