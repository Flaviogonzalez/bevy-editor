//! The editor's node catalog, read from the registry instead of written by hand.

use std::any::TypeId;

use bevy_reflect::func::FunctionRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinType {
    Number,
    Bool,
    Text,
}

impl PinType {
    pub fn of(id: TypeId) -> Option<PinType> {
        if id == TypeId::of::<f32>() {
            Some(PinType::Number)
        } else if id == TypeId::of::<bool>() {
            Some(PinType::Bool)
        } else if id == TypeId::of::<String>() {
            Some(PinType::Text)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeSpec {
    pub name: String,
    pub description: &'static str,
    pub inputs: Vec<(String, PinType)>,
    pub output: PinType,
}

/// One spec per registered function. Functions with types the editor can't show are skipped
/// with a reason, so gaps are visible instead of silent.
pub fn build(registry: &FunctionRegistry) -> (Vec<NodeSpec>, Vec<String>) {
    let mut specs = Vec::new();
    let mut skipped = Vec::new();
    for function in registry.iter() {
        let info = function.info();
        let name = info.name().map(|n| n.to_string()).unwrap_or_default();
        let signature = info.base();
        let inputs: Option<Vec<(String, PinType)>> = signature
            .args()
            .iter()
            .map(|a| {
                let ty = PinType::of(a.ty().id())?;
                Some((a.name().unwrap_or("?").to_string(), ty))
            })
            .collect();
        match (inputs, PinType::of(signature.return_info().ty().id())) {
            (Some(inputs), Some(output)) => specs.push(NodeSpec {
                description: crate::nodes::description(&name),
                name,
                inputs,
                output,
            }),
            _ => skipped.push(format!("{name}: unsupported parameter or return type")),
        }
    }
    specs.sort_by(|a, b| a.name.cmp(&b.name));
    (specs, skipped)
}
