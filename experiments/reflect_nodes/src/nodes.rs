//! The node library, written once as plain Rust functions.
//!
//! `nodes!` keeps each function as-is (the Rust backend calls it directly) and also registers it
//! in a `FunctionRegistry` with its parameter names, so the editor and the Lua preview can find
//! and call it by reflection. Reflection alone knows the parameter *types* of a function but not
//! their names, which is why the macro wraps the definitions.

use bevy_reflect::func::args::ArgInfo;
use bevy_reflect::func::{DynamicFunction, FunctionRegistry, IntoFunction, SignatureInfo};

macro_rules! nodes {
    ($(
        $(#[doc = $doc:literal])*
        fn $name:ident($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty $body:block
    )*) => {
        $(
            $(#[doc = $doc])*
            pub fn $name($($arg: $ty),*) -> $ret $body
        )*

        /// Every node, registered by name with its parameter names.
        pub fn registry() -> FunctionRegistry {
            let mut registry = FunctionRegistry::default();
            $(
                let inner = $name.into_function();
                let mut index = 0usize;
                let args: Vec<ArgInfo> = vec![$({
                    let info = ArgInfo::new::<$ty>(index).with_name(stringify!($arg));
                    index += 1;
                    info
                }),*];
                let _ = index;
                let signature = SignatureInfo::named(stringify!($name)).with_args(args).with_return::<$ret>();
                registry
                    .register(DynamicFunction::new(move |a| inner.call(a), signature))
                    .expect("node names are unique");
            )*
            registry
        }

        /// Typed Lua bindings: the same functions as the registry, called without reflection.
        pub fn lua_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
            let table = lua.create_table()?;
            $(
                table.set(
                    stringify!($name),
                    lua.create_function(|_, ($($arg,)*): ($($ty,)*)| Ok($name($($arg),*)))?,
                )?;
            )*
            Ok(table)
        }

        /// Node descriptions, from the doc comments.
        pub fn description(name: &str) -> &'static str {
            match name {
                $(stringify!($name) => concat!($($doc),*).trim_ascii(),)*
                _ => "",
            }
        }
    };
}

nodes! {
    /// a + b
    fn add(a: f32, b: f32) -> f32 { a + b }
    /// a × b
    fn mul(a: f32, b: f32) -> f32 { a * b }
    /// a ÷ b. Division by zero gives infinity or NaN, as in Rust.
    fn div(a: f32, b: f32) -> f32 { a / b }
    /// Sine of an angle in radians.
    fn sin(x: f32) -> f32 { x.sin() }
    /// Keeps x between lo and hi.
    fn clamp(x: f32, lo: f32, hi: f32) -> f32 { x.max(lo).min(hi) }
    /// a > b
    fn greater(a: f32, b: f32) -> bool { a > b }
    /// Picks a when the condition holds, b otherwise.
    fn select(condition: bool, a: f32, b: f32) -> f32 { if condition { a } else { b } }
    /// Formats a number the way Rust does.
    fn number_to_text(value: f32) -> String { value.to_string() }
    /// Concatenates two texts.
    fn join(a: String, b: String) -> String { a + &b }
}
