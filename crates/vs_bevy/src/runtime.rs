//! One running script: a Lua state with the `vs` and `node` APIs installed.
//!
//! This part knows little about Bevy's ECS. The plugin fills a [`Frame`] from the world, calls
//! [`Instance::update_with`] with access to the entity's other components, and copies the
//! transform back. That keeps the API easy to test.
//!
//! Everything a script computes runs as Rust: `node.*` are the `vs_nodes` functions and `vs.*`
//! go through `vs_nodes::fx`, the same code compiled scripts call.

use std::cell::RefCell;
use std::rc::Rc;

use bevy::transform::components::Transform;
use mlua::{Function, Lua, Variadic};
use vs_nodes::{Arg, Frame, PinKind, fx, lua_types};

/// The script entity's components other than its transform, which `vs.get` and `vs.set` reach.
pub trait Components {
    fn get(&self, component: &str, path: &str) -> Result<Arg, String>;
    fn kind(&self, component: &str, path: &str) -> Result<PinKind, String>;
    fn set(&mut self, component: &str, path: &str, value: Arg) -> Result<(), String>;
    /// Whether the entity passes a query filter (`with`, `without`, `added`, `changed`) on
    /// `component`, so a block runs only on the entities its compiled system's query matches.
    fn filter(&self, op: &str, component: &str) -> Result<bool, String>;
}

/// For running a script with no world around it: every component access fails.
pub struct NoComponents;

impl Components for NoComponents {
    fn get(&self, component: &str, _: &str) -> Result<Arg, String> {
        Err(format!("no {component} here"))
    }
    fn kind(&self, component: &str, _: &str) -> Result<PinKind, String> {
        Err(format!("no {component} here"))
    }
    fn set(&mut self, component: &str, _: &str, _: Arg) -> Result<(), String> {
        Err(format!("no {component} here"))
    }
    fn filter(&self, _: &str, component: &str) -> Result<bool, String> {
        Err(format!("no {component} here"))
    }
}

/// `Transform` fields live in the frame, so they interleave correctly with Translate, Rotate and
/// the other transform intrinsics.
fn transform_field(path: &str) -> Option<PinKind> {
    match path {
        "translation" | "scale" => Some(PinKind::Vec3),
        "rotation" => Some(PinKind::Quat),
        _ => None,
    }
}

fn get_transform(t: &Transform, path: &str) -> Option<Arg> {
    match path {
        "translation" => Some(Arg::Vec3(t.translation)),
        "rotation" => Some(Arg::Quat(t.rotation)),
        "scale" => Some(Arg::Vec3(t.scale)),
        _ => None,
    }
}

fn set_transform(t: &mut Transform, path: &str, value: Arg) -> bool {
    match (path, value) {
        ("translation", Arg::Vec3(v)) => t.translation = v,
        ("rotation", Arg::Quat(q)) => t.rotation = q,
        ("scale", Arg::Vec3(v)) => t.scale = v,
        _ => return false,
    }
    true
}

pub struct Instance {
    lua: Lua,
    frame: Rc<RefCell<Frame>>,
    entity: i64,
    started: bool,
}

impl Instance {
    /// Runs the script's top level, which defines its callbacks. `entity` is the value of the
    /// `entity` global; `name` labels errors and log lines.
    pub fn new(source: &str, entity: i64, name: &str) -> mlua::Result<Self> {
        let lua = Lua::new();
        let frame = Rc::new(RefCell::new(Frame::default()));
        install_api(&lua, &frame, entity, name)?;
        lua.load(source).set_name(format!("@{name}")).exec()?;
        Ok(Self { lua, frame, entity, started: false })
    }

    /// [`Self::update_with`] for a script that touches no components but its transform.
    pub fn update(&mut self, frame: Frame) -> mlua::Result<Frame> {
        self.update_with(frame, &mut NoComponents)
    }

    /// Calls `on_script_loaded` the first time, then `on_update`. Returns the frame afterwards,
    /// with the transform as the script left it. `vs.get` and `vs.set` reach `components` only
    /// during this call.
    pub fn update_with(&mut self, frame: Frame, components: &mut dyn Components) -> mlua::Result<Frame> {
        *self.frame.borrow_mut() = frame;
        let first = !self.started;
        self.started = true;
        let components = RefCell::new(components);
        let entity = self.entity;
        let own = |e: i64| -> mlua::Result<()> {
            if e == entity { Ok(()) } else { Err(mlua::Error::runtime("scripts can only reach their own entity for now")) }
        };
        let fail = |e: String| mlua::Error::runtime(e);
        let (components, own, fail) = (&components, &own, &fail);
        self.lua.scope(|scope| {
            let vs: mlua::Table = self.lua.globals().get("vs")?;
            let f = self.frame.clone();
            vs.set(
                "get",
                scope.create_function(move |lua, (e, component, path): (i64, String, String)| {
                    own(e)?;
                    let value = if component == "Transform" {
                        get_transform(&f.borrow().transform, &path).ok_or_else(|| fail(format!("Transform has no field {path}")))?
                    } else {
                        components.borrow().get(&component, &path).map_err(fail)?
                    };
                    lua_types::to_lua(lua, value)
                })?,
            )?;
            let f = self.frame.clone();
            vs.set(
                "set",
                scope.create_function(move |lua, (e, component, path, value): (i64, String, String, mlua::Value)| {
                    own(e)?;
                    if component == "Transform" {
                        let kind = transform_field(&path).ok_or_else(|| fail(format!("Transform has no field {path}")))?;
                        let value = lua_types::from_lua(lua, kind, value)?;
                        set_transform(&mut f.borrow_mut().transform, &path, value);
                        return Ok(());
                    }
                    let kind = components.borrow().kind(&component, &path).map_err(fail)?;
                    let value = lua_types::from_lua(lua, kind, value)?;
                    components.borrow_mut().set(&component, &path, value).map_err(fail)
                })?,
            )?;
            vs.set(
                "filter",
                scope.create_function(move |_, (e, op, component): (i64, String, String)| {
                    own(e)?;
                    components.borrow().filter(&op, &component).map_err(fail)
                })?,
            )?;
            if first {
                self.call("on_script_loaded")?;
            }
            self.call("on_update")
        })?;
        Ok(self.frame.borrow().clone())
    }

    fn call(&self, name: &str) -> mlua::Result<()> {
        match self.lua.globals().get::<Option<Function>>(name)? {
            Some(f) => f.call::<()>(()),
            None => Ok(()),
        }
    }
}

fn install_api(lua: &Lua, frame: &Rc<RefCell<Frame>>, entity: i64, name: &str) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("entity", entity)?;
    globals.set("node", vs_nodes::lua_table(lua)?)?;

    let label = name.to_string();
    globals.set(
        "print",
        lua.create_function(move |_, args: Variadic<mlua::Value>| {
            // Compiled scripts pass one text, already formatted by a `text.*` node. Hand-written
            // Lua may pass anything.
            let parts: Vec<String> =
                args.iter().map(|v| v.to_string().unwrap_or_else(|_| v.type_name().into())).collect();
            fx::print(&label, &parts.join("\t"));
            Ok(())
        })?,
    )?;

    let vs = lua.create_table()?;
    vs_nodes::lua_types::install_constructors(lua, &vs)?;

    // Only the script's own entity is reachable for now. Checking keeps a future multi-entity API
    // from silently doing the wrong thing in old scripts.
    let own = move |e: i64| -> mlua::Result<()> {
        if e == entity {
            Ok(())
        } else {
            Err(mlua::Error::runtime("scripts can only reach their own entity for now"))
        }
    };

    let f = frame.clone();
    vs.set("delta", lua.create_function(move |_, ()| Ok(f.borrow().delta))?)?;
    let f = frame.clone();
    vs.set("time", lua.create_function(move |_, ()| Ok(f.borrow().time))?)?;
    let f = frame.clone();
    vs.set(
        "input",
        lua.create_function(move |_, (device, press, name): (String, String, String)| {
            let f = f.borrow();
            Ok(match device.as_str() {
                "key" => f.keys.query(&press, &name),
                "mouse" => f.mouse.query(&press, &name),
                _ => false,
            })
        })?,
    )?;
    let f = frame.clone();
    vs.set(
        "despawn",
        lua.create_function(move |_, e: i64| {
            own(e)?;
            f.borrow_mut().despawn = true;
            Ok(())
        })?,
    )?;

    let f = frame.clone();
    vs.set(
        "position",
        lua.create_function(move |lua, e: i64| {
            own(e)?;
            let p = f.borrow().transform.translation;
            let t = lua.create_table()?;
            t.set("x", p.x)?;
            t.set("y", p.y)?;
            t.set("z", p.z)?;
            Ok(t)
        })?,
    )?;

    type Op = fn(&mut bevy::transform::components::Transform, f32, f32, f32);
    let ops: [(&str, Op); 4] = [
        ("translate", fx::translate),
        ("rotate", fx::rotate),
        ("set_position", fx::set_position),
        ("set_scale", fx::set_scale),
    ];
    for (op_name, op) in ops {
        let f = frame.clone();
        vs.set(
            op_name,
            lua.create_function(move |_, (e, x, y, z): (i64, f32, f32, f32)| {
                own(e)?;
                op(&mut f.borrow_mut().transform, x, y, z);
                Ok(())
            })?,
        )?;
    }

    globals.set("vs", vs)
}
