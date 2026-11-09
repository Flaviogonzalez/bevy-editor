//! How node values cross into Lua. Numbers, booleans and text are plain Lua values. Vectors,
//! rotations and colors are userdata holding the Rust value itself, so a value Lua passes back
//! into Rust is bit for bit the one Rust handed out.

use bevy::color::Color;
use bevy::math::{Quat, Vec3};
use mlua::{FromLua, IntoLua, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value};

use crate::fx;

/// A Rust type nodes can take or return, and the Lua-facing type it travels as.
pub trait LuaType: Sized {
    type Wrapper: FromLua + IntoLua;
    fn wrap(self) -> Self::Wrapper;
    fn unwrap(wrapper: Self::Wrapper) -> Self;
}

macro_rules! plain {
    ($($ty:ty),*) => {$(
        impl LuaType for $ty {
            type Wrapper = $ty;
            fn wrap(self) -> $ty { self }
            fn unwrap(wrapper: $ty) -> $ty { wrapper }
        }
    )*};
}

plain!(f32, bool, String);

macro_rules! boxed {
    ($wrapper:ident, $ty:ty, $name:literal, [$($field:ident),*]) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $wrapper(pub $ty);

        impl LuaType for $ty {
            type Wrapper = $wrapper;
            fn wrap(self) -> $wrapper { $wrapper(self) }
            fn unwrap(wrapper: $wrapper) -> $ty { wrapper.0 }
        }

        impl FromLua for $wrapper {
            fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
                match &value {
                    Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
                    _ => Err(mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: $name.into(),
                        message: None,
                    }),
                }
            }
        }

        impl UserData for $wrapper {
            #[allow(unused_variables)]
            fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
                $(fields.add_field_method_get(stringify!($field), |_, v| Ok(boxed!(@get v.0, $field)));)*
            }

            fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
                methods.add_meta_method(MetaMethod::ToString, |_, v, ()| Ok(format!("{:?}", v.0)));
                methods.add_meta_method(MetaMethod::Eq, |_, a, b: $wrapper| Ok(*a == b));
            }
        }
    };
    (@get $v:expr, $field:ident) => { $v.$field };
}

boxed!(LuaVec3, Vec3, "Vec3", [x, y, z]);
boxed!(LuaQuat, Quat, "Quat", [x, y, z, w]);
boxed!(LuaColor, Color, "Color", []);

/// `vs.vec3`, `vs.quat_euler` and `vs.color`: the constructors generated Lua uses for literals.
/// They call the same functions generated Rust does.
pub fn install_constructors(lua: &Lua, vs: &mlua::Table) -> mlua::Result<()> {
    vs.set("vec3", lua.create_function(|_, (x, y, z): (f32, f32, f32)| Ok(LuaVec3(Vec3::new(x, y, z))))?)?;
    vs.set("quat_euler", lua.create_function(|_, (x, y, z): (f32, f32, f32)| Ok(LuaQuat(fx::euler(x, y, z))))?)?;
    vs.set("quat", lua.create_function(|_, (x, y, z, w): (f32, f32, f32, f32)| Ok(LuaQuat(Quat::from_xyzw(x, y, z, w))))?)?;
    vs.set("color", lua.create_function(|_, (r, g, b, a): (f32, f32, f32, f32)| Ok(LuaColor(fx::srgba(r, g, b, a))))?)?;
    Ok(())
}

/// A node value as a Lua value.
pub fn to_lua(lua: &Lua, arg: crate::Arg) -> mlua::Result<Value> {
    use crate::Arg;
    match arg {
        Arg::Number(n) => n.into_lua(lua),
        Arg::Bool(b) => b.into_lua(lua),
        Arg::Text(s) => s.into_lua(lua),
        Arg::Vec3(v) => LuaVec3(v).into_lua(lua),
        Arg::Quat(q) => LuaQuat(q).into_lua(lua),
        Arg::Color(c) => LuaColor(c).into_lua(lua),
    }
}

/// A Lua value as a node value of `kind`, converted the way typed bindings convert arguments.
pub fn from_lua(lua: &Lua, kind: crate::PinKind, value: Value) -> mlua::Result<crate::Arg> {
    use crate::{Arg, PinKind};
    Ok(match kind {
        PinKind::Number => Arg::Number(f32::from_lua(value, lua)?),
        PinKind::Bool => Arg::Bool(bool::from_lua(value, lua)?),
        PinKind::Text => Arg::Text(String::from_lua(value, lua)?),
        PinKind::Vec3 => Arg::Vec3(LuaVec3::from_lua(value, lua)?.0),
        PinKind::Quat => Arg::Quat(LuaQuat::from_lua(value, lua)?.0),
        PinKind::Color => Arg::Color(LuaColor::from_lua(value, lua)?.0),
    })
}
