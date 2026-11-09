//! The single definition of every visual scripting node.
//!
//! Pure nodes are plain Rust functions declared with [`nodes!`]. From each one we get:
//!
//! - the function itself, which the Rust backend inlines from its source and calls to fold
//!   constants;
//! - a reflected entry in a `FunctionRegistry`, from which [`catalog`] builds the editor's node list;
//! - with the `lua` feature, a typed Lua binding that calls the same function, so the preview
//!   computes exactly what the shipped game computes.
//!
//! Nodes come in sets, each a `nodes!` invocation: [`core`] is written by hand, [`glam`] is
//! generated from Bevy's math types by `tools/gen_math` and checked in.
//!
//! Nodes that touch the frame or the entity (time, input, the transform) are the fixed set of
//! intrinsics in [`fx`] and [`Frame`], shared by both runtimes for the same reason.

use std::any::TypeId;

use bevy::color::Color;
use bevy::math::{Quat, Vec3};
use bevy::reflect::func::FunctionRegistry;

pub mod fx;
#[cfg(feature = "lua")]
pub mod lua_types;
pub use fx::{Buttons, Frame};

/// The data types a pin can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PinKind {
    Number,
    Bool,
    Text,
    Vec3,
    Quat,
    Color,
}

impl PinKind {
    pub fn of(id: TypeId) -> Option<PinKind> {
        [
            (TypeId::of::<f32>(), PinKind::Number),
            (TypeId::of::<bool>(), PinKind::Bool),
            (TypeId::of::<String>(), PinKind::Text),
            (TypeId::of::<Vec3>(), PinKind::Vec3),
            (TypeId::of::<Quat>(), PinKind::Quat),
            (TypeId::of::<Color>(), PinKind::Color),
        ]
        .into_iter()
        .find(|(t, _)| *t == id)
        .map(|(_, k)| k)
    }
}

/// A pure node as the editor and the compiler see it.
#[derive(Clone, Debug)]
pub struct PureNode {
    /// `category_module.function`, such as `math.add`. Also the registry name.
    pub kind: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    /// The Lua expression the preview calls, such as `node.math.add`.
    pub lua_path: String,
    pub inputs: Vec<(String, PinKind)>,
    pub output: PinKind,
    /// The function body as source, such as `{ a + b }`. The Rust backend inlines it with the
    /// arguments substituted, so exported code doesn't depend on this crate.
    pub body: &'static str,
}

/// A value passed to [`call`].
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Number(f32),
    Bool(bool),
    Text(String),
    Vec3(Vec3),
    Quat(Quat),
    Color(Color),
}

trait ArgType: Sized {
    fn from_arg(arg: &Arg) -> Option<Self>;
    fn into_arg(self) -> Arg;
}

macro_rules! arg_type {
    ($($ty:ty => $variant:ident),*) => {$(
        impl ArgType for $ty {
            fn from_arg(arg: &Arg) -> Option<Self> {
                if let Arg::$variant(v) = arg { Some(v.clone()) } else { None }
            }
            fn into_arg(self) -> Arg {
                Arg::$variant(self)
            }
        }
    )*};
}

arg_type!(f32 => Number, bool => Bool, String => Text, Vec3 => Vec3, Quat => Quat, Color => Color);

struct Meta {
    kind: &'static str,
    title: &'static str,
    category: &'static str,
    description: &'static str,
    module: &'static str,
    function: &'static str,
    body: &'static str,
}

/// Declares a set of pure nodes, grouped by category, in a module named `$set`. See the crate
/// docs for what each declaration produces. Reflection records parameter types but not their
/// names, so the macro adds them.
///
/// A node is either a function with its body:
///
/// ```ignore
/// "Add" fn add(a: f32, b: f32) -> f32 { a + b }
/// ```
///
/// or a wrapper around an existing function, by path. The compiler checks that the path has
/// exactly the declared signature, so a dependency update that changes it fails the build instead
/// of silently changing the node:
///
/// ```ignore
/// "Atan2" use atan2(y: f32, x: f32) -> f32 = ops::atan2;
/// "Lerp" use lerp(a: f32, b: f32, t: f32) -> f32 = <f32 as FloatExt>::lerp;
/// ```
///
/// Bodies and paths are written with `bevy::prelude::*` in scope, as exported code has it.
macro_rules! nodes {
    ($set:ident; $(
        $category:literal $module:ident {
            $( $(#[doc = $doc:literal])* $title:literal $form:ident $name:ident ($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty $( = $path:path ; )? $($body:block)? )*
        }
    )*) => {
        pub mod $set {
            #[allow(unused_imports)]
            use bevy::prelude::*;
            #[allow(unused_imports)]
            use $crate::{Arg, ArgType, Meta};
            #[allow(unused_imports)]
            use bevy::reflect::func::args::ArgInfo;
            #[allow(unused_imports)]
            use bevy::reflect::func::{DynamicFunction, FunctionRegistry, IntoFunction, SignatureInfo};

            $(
                #[doc = concat!("Nodes in the ", $category, " category.")]
                pub mod $module {
                    #[allow(unused_imports)]
                    use bevy::prelude::*;
                    $(
                        nodes!(@fn [$(#[doc = $doc])*] $name ($($arg : $ty),*) -> $ret $( = $path )? $($body)?);
                    )*
                }
            )*

            #[allow(unused_assignments)]
            pub(crate) fn register(registry: &mut FunctionRegistry) {
                $($(
                    let inner = $module::$name.into_function();
                    #[allow(unused_mut, unused_variables, unused_assignments)]
                    let mut index = 0usize;
                    let args: Vec<ArgInfo> = vec![$({
                        let info = ArgInfo::new::<$ty>(index).with_name(stringify!($arg));
                        index += 1;
                        info
                    }),*];
                    let kind = concat!(stringify!($module), ".", stringify!($name));
                    let signature = SignatureInfo::named(kind).with_args(args).with_return::<$ret>();
                    registry
                        .register(DynamicFunction::new(move |a| inner.call(a), signature))
                        .expect("node kinds are unique");
                )*)*
            }

            pub(crate) fn metas() -> Vec<Meta> {
                vec![$($(Meta {
                    kind: concat!(stringify!($module), ".", stringify!($name)),
                    title: $title,
                    category: $category,
                    description: concat!($($doc),*),
                    module: stringify!($module),
                    function: stringify!($name),
                    body: nodes!(@body ($($arg),*) $( = $path )? $($body)?),
                },)*)*]
            }

            /// `None` when no node here has this kind; `Some(None)` when the args don't fit.
            pub(crate) fn call(kind: &str, args: &[Arg]) -> Option<Option<Arg>> {
                $($(
                    if kind == concat!(stringify!($module), ".", stringify!($name)) {
                        #[allow(unused_mut, unused_variables, unused_assignments)]
                        let mut args = args.iter();
                        $(let Some($arg) = args.next().and_then(<$ty as ArgType>::from_arg) else { return Some(None) };)*
                        return Some(Some(ArgType::into_arg($module::$name($($arg),*))));
                    }
                )*)*
                None
            }

            #[cfg(feature = "lua")]
            pub(crate) fn lua_install(lua: &mlua::Lua, node: &mlua::Table) -> mlua::Result<()> {
                #[allow(unused_imports)]
                use $crate::lua_types::LuaType;
                $(
                    let category: mlua::Table = match node.get::<Option<mlua::Table>>(stringify!($module))? {
                        Some(t) => t,
                        None => lua.create_table()?,
                    };
                    $(
                        category.set(
                            stringify!($name),
                            lua.create_function(|_, ($($arg,)*): ($(<$ty as LuaType>::Wrapper,)*)| {
                                Ok(LuaType::wrap($module::$name($(<$ty as LuaType>::unwrap($arg)),*)))
                            })?,
                        )?;
                    )*
                    node.set(stringify!($module), category)?;
                )*
                Ok(())
            }
        }
    };
    (@fn [$($doc:tt)*] $name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty = $path:path) => {
        $($doc)*
        pub fn $name($($arg: $ty),*) -> $ret {
            const _: fn($($ty),*) -> $ret = $path;
            $path($($arg),*)
        }
    };
    (@fn [$($doc:tt)*] $name:ident ($($arg:ident : $ty:ty),*) -> $ret:ty $body:block) => {
        $($doc)*
        pub fn $name($($arg: $ty),*) -> $ret $body
    };
    (@body ($($arg:ident),*) = $path:path) => {
        concat!("{ ", stringify!($path), "(", $(stringify!($arg), ", ",)* ") }")
    };
    (@body ($($arg:ident),*) $body:block) => {
        stringify!($body)
    };
}

mod core_nodes;
mod glam_nodes;
pub use core_nodes::core;
pub use glam_nodes::glam;

/// Every pure node, registered by kind with its parameter names.
pub fn registry() -> FunctionRegistry {
    let mut registry = FunctionRegistry::default();
    core::register(&mut registry);
    glam::register(&mut registry);
    registry
}

fn metas() -> Vec<Meta> {
    let mut metas = core::metas();
    metas.extend(glam::metas());
    metas
}

/// Calls a node by kind. The Rust backend uses it to fold nodes whose inputs are all literals
/// into one literal, computed by the same function. `None` for an unknown kind or wrong args.
pub fn call(kind: &str, args: &[Arg]) -> Option<Arg> {
    core::call(kind, args).or_else(|| glam::call(kind, args)).flatten()
}

/// Typed Lua bindings: a `node` table with one subtable per category, calling the same functions
/// as the registry without reflection on each call.
#[cfg(feature = "lua")]
pub fn lua_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let node = lua.create_table()?;
    core::lua_install(lua, &node)?;
    glam::lua_install(lua, &node)?;
    Ok(node)
}

/// The `KeyCode` variants scripts can name, read from `KeyCode`'s reflected type info. Variants
/// with data (unidentified keys) are left out, because a script can't spell them.
pub fn key_names() -> Vec<&'static str> {
    use bevy::input::keyboard::KeyCode;
    use bevy::reflect::{Typed, enums::VariantInfo};
    let info = KeyCode::type_info().as_enum().expect("KeyCode is an enum");
    info.iter().filter(|v| matches!(v, VariantInfo::Unit(_))).map(|v| v.name()).collect()
}

/// The pure nodes, built from the registry by reflection. Panics if a node uses a type pins
/// can't carry, so such a node can't ship by accident.
pub fn catalog() -> Vec<PureNode> {
    let registry = registry();
    metas()
        .into_iter()
        .map(|meta| {
            let function = registry.get(meta.kind).expect("every meta is registered");
            let signature = function.info().base();
            let inputs = signature
                .args()
                .iter()
                .map(|a| {
                    let kind = PinKind::of(a.ty().id())
                        .unwrap_or_else(|| panic!("{}: unsupported parameter type", meta.kind));
                    (a.name().expect("nodes! names every parameter").to_string(), kind)
                })
                .collect();
            let output = PinKind::of(signature.return_info().ty().id())
                .unwrap_or_else(|| panic!("{}: unsupported return type", meta.kind));
            PureNode {
                kind: meta.kind,
                title: meta.title,
                category: meta.category,
                description: meta.description.trim_ascii(),
                lua_path: format!("node.{}.{}", meta.module, meta.function),
                inputs,
                output,
                body: meta.body,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_names_and_types_from_reflection() {
        let catalog = catalog();
        let add = catalog.iter().find(|n| n.kind == "math.add").unwrap();
        assert_eq!(add.inputs, vec![("a".into(), PinKind::Number), ("b".into(), PinKind::Number)]);
        assert_eq!(add.output, PinKind::Number);
        assert_eq!(add.title, "Add");
        let gt = catalog.iter().find(|n| n.kind == "logic.gt").unwrap();
        assert_eq!(gt.output, PinKind::Bool);
        let cross = catalog.iter().find(|n| n.kind == "vec3.cross").unwrap();
        assert_eq!(cross.output, PinKind::Vec3);
    }

    #[test]
    fn call_runs_the_node() {
        assert_eq!(call("math.mul", &[Arg::Number(2.0), Arg::Number(3.0)]), Some(Arg::Number(6.0)));
        assert_eq!(call("logic.not", &[Arg::Bool(true)]), Some(Arg::Bool(false)));
        assert_eq!(call("math.mul", &[Arg::Bool(true)]), None);
        let add = catalog().into_iter().find(|n| n.kind == "math.add").unwrap();
        assert_eq!(add.body.replace(' ', ""), "{a+b}");
    }

    #[test]
    fn wrappers_record_a_call_as_their_body() {
        let atan2 = catalog().into_iter().find(|n| n.kind == "math.atan2").unwrap();
        assert_eq!(atan2.body.replace(' ', ""), "{f32::atan2(y,x,)}");
        assert_eq!(call("math.atan2", &[Arg::Number(1.0), Arg::Number(2.0)]), Some(Arg::Number(1.0f32.atan2(2.0))));
    }

    #[test]
    fn key_names_come_from_keycode() {
        let keys = key_names();
        assert!(keys.contains(&"Space") && keys.contains(&"KeyW") && keys.contains(&"ArrowUp"));
        assert!(!keys.contains(&"Unidentified"));
    }

    #[cfg(feature = "lua")]
    #[test]
    fn lua_bindings_call_the_same_functions() {
        let lua = mlua::Lua::new();
        lua.globals().set("node", lua_table(&lua).unwrap()).unwrap();
        let vs = lua.create_table().unwrap();
        lua_types::install_constructors(&lua, &vs).unwrap();
        lua.globals().set("vs", vs).unwrap();
        let sum: f32 = lua.load("return node.math.add(16777216, 1)").eval().unwrap();
        assert_eq!(sum, core::math::add(16_777_216.0, 1.0));
        let text: String = lua.load("return node.text.from_number(0.1)").eval().unwrap();
        assert_eq!(text, "0.1");
        let len: f32 = lua.load("return node.vec3.length(vs.vec3(3, 4, 0))").eval().unwrap();
        assert_eq!(len, 5.0);
        let v: lua_types::LuaVec3 = lua.load("return node.vec3.cross(vs.vec3(1, 0, 0), vs.vec3(0, 1, 0))").eval().unwrap();
        assert_eq!(v.0, Vec3::Z);
    }
}
