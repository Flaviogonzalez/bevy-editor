//! Components through reflection: the preview reads and writes their fields by name, and the
//! editor's catalog of components is described from the type registry.

use std::any::TypeId;

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::{GetPath, PartialReflect, TypeInfo, TypeRegistration, TypeRegistry};
use serde::Serialize;
use vs_nodes::{Arg, PinKind};

/// A field's value as a node value, if its type is one pins carry.
pub fn read(value: &dyn PartialReflect) -> Option<Arg> {
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return Some(Arg::Number(*v));
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return Some(Arg::Bool(*v));
    }
    if let Some(v) = value.try_downcast_ref::<String>() {
        return Some(Arg::Text(v.clone()));
    }
    if let Some(v) = value.try_downcast_ref::<Vec3>() {
        return Some(Arg::Vec3(*v));
    }
    if let Some(v) = value.try_downcast_ref::<Quat>() {
        return Some(Arg::Quat(*v));
    }
    if let Some(v) = value.try_downcast_ref::<Color>() {
        return Some(Arg::Color(*v));
    }
    None
}

/// Stores a node value into a field of the same type. False when the types differ.
pub fn write(target: &mut dyn PartialReflect, value: Arg) -> bool {
    fn put<T: 'static>(target: &mut dyn PartialReflect, v: T) -> bool {
        match target.try_downcast_mut::<T>() {
            Some(slot) => {
                *slot = v;
                true
            }
            None => false,
        }
    }
    match value {
        Arg::Number(v) => put(target, v),
        Arg::Bool(v) => put(target, v),
        Arg::Text(v) => put(target, v),
        Arg::Vec3(v) => put(target, v),
        Arg::Quat(v) => put(target, v),
        Arg::Color(v) => put(target, v),
    }
}

/// The component registered under a short name, such as `PointLight`.
fn registration<'a>(registry: &'a TypeRegistry, name: &str) -> Result<(&'a TypeRegistration, &'a ReflectComponent), String> {
    let reg = registry.get_with_short_type_path(name).ok_or_else(|| format!("no component named {name}"))?;
    let component = reg.data::<ReflectComponent>().ok_or_else(|| format!("{name} isn't a component"))?;
    Ok((reg, component))
}

/// Reads `name.path` on an entity.
pub fn get(world: &World, registry: &TypeRegistry, entity: Entity, name: &str, path: &str) -> Result<Arg, String> {
    let (_, component) = registration(registry, name)?;
    let entity_ref = world.get_entity(entity).map_err(|e| e.to_string())?;
    let value = component.reflect(entity_ref).ok_or_else(|| format!("the entity has no {name}"))?;
    let field = value.reflect_path(path).map_err(|e| format!("{name}.{path}: {e}"))?;
    read(field).ok_or_else(|| format!("{name}.{path} isn't a number, bool, text, vector, rotation or color"))
}

/// The type of `name.path`, so a value coming from Lua can be converted to it.
pub fn kind(registry: &TypeRegistry, name: &str, path: &str) -> Result<PinKind, String> {
    let (reg, _) = registration(registry, name)?;
    let mut fields = Vec::new();
    walk(registry, reg.type_info(), String::new(), 0, MAX_LEVELS, &mut fields);
    fields.into_iter().find(|(p, _)| p == path).map(|(_, k)| k).ok_or_else(|| format!("{name} has no field {path}"))
}

/// Writes `name.path` on an entity.
pub fn set(world: &mut World, registry: &TypeRegistry, entity: Entity, name: &str, path: &str, value: Arg) -> Result<(), String> {
    let (_, component) = registration(registry, name)?;
    let mut entity_mut = world.get_entity_mut(entity).map_err(|e| e.to_string())?;
    let mut target = component.reflect_mut(&mut entity_mut).ok_or_else(|| format!("the entity has no {name}"))?;
    let field = target.reflect_path_mut(path).map_err(|e| format!("{name}.{path}: {e}"))?;
    if write(field, value) { Ok(()) } else { Err(format!("{name}.{path} has another type")) }
}

/// A component as the editor's catalog lists it.
#[derive(Clone, Debug, Serialize)]
pub struct Described {
    pub name: String,
    pub rust_path: String,
    pub fields: Vec<DescribedField>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DescribedField {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
}

/// The name the catalog uses for a pin type, matching `vs_graph::PinType`.
fn type_name(kind: PinKind) -> &'static str {
    match kind {
        PinKind::Number => "Number",
        PinKind::Bool => "Bool",
        PinKind::Text => "String",
        PinKind::Vec3 => "Vec3",
        PinKind::Quat => "Quat",
        PinKind::Color => "Color",
    }
}

/// The deepest nesting a catalog may ask for. Reflection paths are short, so this is generous.
pub const MAX_LEVELS: usize = 6;

/// The levels Bevy's bundled components are described with: `a.b.c` at most.
pub const BUNDLED_LEVELS: usize = 3;

/// The fields of a type that pins can carry, as reflection paths, up to `levels` deep (1 is the
/// type's own fields, 2 reaches `a.b`). Values of pin types are leaves, so `translation` is
/// listed but not `translation.x`.
fn walk(registry: &TypeRegistry, info: &TypeInfo, prefix: String, depth: usize, levels: usize, out: &mut Vec<(String, PinKind)>) {
    let visit = |name: String, id: TypeId, out: &mut Vec<(String, PinKind)>| {
        let path = if prefix.is_empty() { name } else { format!("{prefix}.{name}") };
        match PinKind::of(id) {
            Some(kind) => out.push((path, kind)),
            None if depth + 1 < levels => {
                if let Some(inner) = registry.get_type_info(id) {
                    walk(registry, inner, path, depth + 1, levels, out);
                }
            }
            None => {}
        }
    };
    match info {
        TypeInfo::Struct(s) => {
            for f in s.iter() {
                visit(f.name().to_string(), f.type_id(), out);
            }
        }
        TypeInfo::TupleStruct(s) => {
            for f in s.iter() {
                visit(f.index().to_string(), f.type_id(), out);
            }
        }
        _ => {}
    }
}

/// Describes a registered component, its fields up to `levels` deep, or `None` if none of them
/// fit a pin.
pub fn describe(registry: &TypeRegistry, reg: &TypeRegistration, name: &str, rust_path: String, levels: usize) -> Option<Described> {
    let mut fields = Vec::new();
    walk(registry, reg.type_info(), String::new(), 0, levels, &mut fields);
    if fields.is_empty() {
        return None;
    }
    let fields = fields.into_iter().map(|(path, kind)| DescribedField { path, ty: type_name(kind) }).collect();
    Some(Described { name: name.to_string(), rust_path, fields })
}

/// Bevy components by short name, with the path exported code names them by.
pub fn describe_named(registry: &TypeRegistry, names: &[&str]) -> Vec<Described> {
    names
        .iter()
        .filter_map(|name| {
            let reg = registry.get_with_short_type_path(name)?;
            reg.data::<ReflectComponent>()?;
            describe(registry, reg, name, name.to_string(), BUNDLED_LEVELS)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Label(String);

    #[derive(Reflect, Default, Clone, Copy)]
    struct Offset {
        x: f32,
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Shadow {
        offset: Offset,
        color: Color,
    }

    /// The catalog's paths are the ones reflection reads: `0` for a tuple struct's field, `a.b`
    /// for a nested one.
    #[test]
    fn catalog_paths_read_and_write() {
        let mut world = World::new();
        let registry = AppTypeRegistry::default();
        {
            let mut r = registry.write();
            r.register::<Label>();
            r.register::<Shadow>();
        }
        let e = world.spawn((Label("hi".into()), Shadow::default())).id();
        let r = registry.read();

        let label = describe_named(&r, &["Label"]);
        assert_eq!(label[0].fields[0].path, "0");
        assert_eq!(get(&world, &r, e, "Label", "0").unwrap(), Arg::Text("hi".into()));
        set(&mut world, &r, e, "Label", "0", Arg::Text("bye".into())).unwrap();
        assert_eq!(world.get::<Label>(e).unwrap().0, "bye");

        let shadow = describe_named(&r, &["Shadow"]);
        let paths: Vec<&str> = shadow[0].fields.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["offset.x", "color"]);
        set(&mut world, &r, e, "Shadow", "offset.x", Arg::Number(2.5)).unwrap();
        assert_eq!(get(&world, &r, e, "Shadow", "offset.x").unwrap(), Arg::Number(2.5));
        assert_eq!(kind(&r, "Shadow", "color").unwrap(), PinKind::Color);
        assert!(set(&mut world, &r, e, "Shadow", "color", Arg::Number(1.0)).is_err());
    }
}
