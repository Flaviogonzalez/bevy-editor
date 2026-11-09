//! Runs bevy-editor visual scripts in Bevy.
//!
//! Add [`VisualScriptPlugin`], then give an entity a [`VisualScript`] pointing at a `.lua` file the
//! editor saved. Each script gets its own Lua state with the `vs` API, its `on_script_loaded` runs
//! once and its `on_update` every frame. When the file changes on disk (with Bevy's
//! `file_watcher` feature), the script restarts with the new code.
//!
//! Games made with the editor don't use `VisualScript` directly: they spawn the components
//! generated from their scripts, and [`preview::register`] attaches the scripts to them. Shipped
//! builds compile those components to plain Bevy systems and don't depend on this crate.

pub mod catalog;
pub mod components;
pub mod preview;
pub mod runtime;

use std::collections::HashMap;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::input::ButtonInput;
use bevy::ecs::system::SystemState;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

pub use runtime::{Components, Instance};
pub use vs_nodes::{Buttons, Frame};

pub struct VisualScriptPlugin;

impl Plugin for VisualScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<LuaScript>()
            .init_asset_loader::<LuaScriptLoader>()
            .insert_non_send(Runtime::default())
            .add_systems(Update, run_scripts);
    }
}

/// The source of a `.lua` script file.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct LuaScript {
    pub source: String,
}

/// Runs a script on this entity.
#[derive(Component, Clone, Debug)]
#[require(Transform)]
pub struct VisualScript(pub Handle<LuaScript>);

#[derive(Default, TypePath)]
struct LuaScriptLoader;

#[derive(Debug, thiserror::Error)]
enum LuaScriptLoaderError {
    #[error("couldn't read the script: {0}")]
    Io(#[from] std::io::Error),
    #[error("the script isn't valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl AssetLoader for LuaScriptLoader {
    type Asset = LuaScript;
    type Settings = ();
    type Error = LuaScriptLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<LuaScript, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let source = String::from_utf8(bytes)?;
        // Some Windows editors save UTF-8 with a byte order mark, which Lua rejects.
        let source = source.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(source);
        Ok(LuaScript { source })
    }

    fn extensions(&self) -> &[&str] {
        &["lua"]
    }
}

struct Running {
    script: AssetId<LuaScript>,
    /// `None` after an error: the script stays stopped until its file changes.
    instance: Option<Instance>,
}

/// The Lua states, keyed by entity. Lua isn't `Send`, so this lives on the main thread.
#[derive(Default)]
struct Runtime {
    running: HashMap<Entity, Running>,
}

/// One entity's components, reached by reflection for `vs.get` and `vs.set`.
struct WorldComponents<'w> {
    world: &'w mut World,
    registry: &'w AppTypeRegistry,
    entity: Entity,
}

impl Components for WorldComponents<'_> {
    fn get(&self, component: &str, path: &str) -> Result<vs_nodes::Arg, String> {
        components::get(self.world, &self.registry.read(), self.entity, component, path)
    }

    fn kind(&self, component: &str, path: &str) -> Result<vs_nodes::PinKind, String> {
        components::kind(&self.registry.read(), component, path)
    }

    fn set(&mut self, component: &str, path: &str, value: vs_nodes::Arg) -> Result<(), String> {
        components::set(self.world, &self.registry.read(), self.entity, component, path, value)
    }
}

/// Runs every script for one frame. Exclusive, because scripts may read and write any component
/// of their entity through reflection.
fn run_scripts(world: &mut World, events: &mut SystemState<MessageReader<AssetEvent<LuaScript>>>) {
    let mut runtime = world.remove_non_send::<Runtime>().unwrap_or_default();

    // A changed file restarts every script that uses it.
    let changed: Vec<AssetId<LuaScript>> = match events.get_mut(world) {
        Ok(mut reader) => reader
            .read()
            .filter_map(|event| match event {
                AssetEvent::Modified { id } | AssetEvent::Removed { id } => Some(id.clone()),
                _ => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    runtime.running.retain(|_, r| !changed.contains(&r.script));

    let mut query = world.query::<(Entity, &VisualScript, &Transform)>();
    let targets: Vec<(Entity, Handle<LuaScript>, Transform)> =
        query.iter(world).map(|(e, s, t)| (e, s.0.clone(), *t)).collect();
    runtime.running.retain(|entity, _| targets.iter().any(|(e, ..)| e == entity));
    let registry = world.resource::<AppTypeRegistry>().clone();

    for (entity, handle, transform) in targets {
        let id = handle.id();
        let name = handle.path().map_or_else(|| format!("{id}"), |p| p.to_string());
        if runtime.running.get(&entity).is_none_or(|r| r.script != id) {
            // Not loaded yet: try again next frame.
            let Some(asset) = world.resource::<Assets<LuaScript>>().get(id) else { continue };
            let instance = match Instance::new(&asset.source, entity.to_bits() as i64, &name) {
                Ok(instance) => {
                    info!(target: "visual_script", "started {name} on {entity}");
                    Some(instance)
                }
                Err(e) => {
                    error!(target: "visual_script", "{name} failed to load: {e}");
                    None
                }
            };
            runtime.running.insert(entity, Running { script: id, instance });
        }

        let running = runtime.running.get_mut(&entity).expect("inserted above");
        let Some(instance) = &mut running.instance else { continue };
        let frame = frame_for(
            world.resource::<Time>(),
            world.get_resource::<ButtonInput<KeyCode>>(),
            world.get_resource::<ButtonInput<MouseButton>>(),
            transform,
        );
        let mut access = WorldComponents { world, registry: &registry, entity };
        match instance.update_with(frame, &mut access) {
            Ok(after) => {
                // Only write when the script moved something, so change detection stays useful.
                if after.transform != transform
                    && let Some(mut t) = world.get_mut::<Transform>(entity)
                {
                    *t = after.transform;
                }
                // Like `commands.entity(e).despawn()` in the compiled systems: after the update.
                if after.despawn {
                    world.despawn(entity);
                }
            }
            Err(e) => {
                error!(target: "visual_script", "{name} stopped: {e}");
                running.instance = None;
            }
        }
    }
    world.insert_non_send(runtime);
}

/// What a script sees this frame.
pub fn frame_for(
    time: &Time,
    keys: Option<&ButtonInput<KeyCode>>,
    mouse: Option<&ButtonInput<MouseButton>>,
    transform: Transform,
) -> Frame {
    Frame {
        delta: time.delta_secs(),
        time: time.elapsed_secs(),
        keys: keys.map(Buttons::of).unwrap_or_default(),
        mouse: mouse.map(Buttons::of).unwrap_or_default(),
        transform,
        despawn: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vs_graph::{Graph, PinRef, Value, Variable};

    /// The editor's spinner graph, compiled and run against the real API.
    #[test]
    fn runs_a_compiled_graph() {
        let mut g = Graph::default();
        g.variables.push(Variable { name: "speed".into(), value: Value::Number(90.0) });
        let start = g.add_node("event.start", None, 0.0, 0.0);
        let print = g.add_node("action.print", None, 0.0, 0.0);
        let update = g.add_node("event.update", None, 0.0, 0.0);
        let speed = g.add_node("var.get", Some("speed".into()), 0.0, 0.0);
        let mul = g.add_node("math.mul", None, 0.0, 0.0);
        let rotate = g.add_node("transform.rotate", None, 0.0, 0.0);
        let translate = g.add_node("transform.translate", None, 0.0, 0.0);
        g.link(PinRef::new(start, "then"), PinRef::new(print, "in"));
        g.link(PinRef::new(update, "then"), PinRef::new(rotate, "in"));
        g.link(PinRef::new(rotate, "then"), PinRef::new(translate, "in"));
        g.link(PinRef::new(speed, "value"), PinRef::new(mul, "a"));
        g.link(PinRef::new(update, "delta"), PinRef::new(mul, "b"));
        g.link(PinRef::new(mul, "result"), PinRef::new(rotate, "y"));
        g.node_mut(translate).unwrap().values.insert("x".into(), Value::Number(2.0));

        let compiled = vs_graph::compile(&g);
        assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
        let file = vs_graph::save_to_string(&g);

        let mut instance = Instance::new(&file, 7, "test").expect("script loads");
        let after = instance.update(Frame { delta: 1.0, ..Default::default() }).expect("runs");
        let (yaw, _, _) = after.transform.rotation.to_euler(EulerRot::YXZ);
        assert!((yaw.to_degrees() - 90.0).abs() < 0.01, "yaw = {}", yaw.to_degrees());
        assert_eq!(after.transform.translation, Vec3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn key_events_fire_once() {
        let mut g = Graph::default();
        let key = g.add_node("event.key", None, 0.0, 0.0);
        let translate = g.add_node("transform.translate", None, 0.0, 0.0);
        g.link(PinRef::new(key, "then"), PinRef::new(translate, "in"));
        g.node_mut(translate).unwrap().values.insert("y".into(), Value::Number(1.0));
        let mut instance = Instance::new(&vs_graph::save_to_string(&g), 1, "test").unwrap();

        let space = || vec!["Space".to_string()];
        let pressed = Frame {
            keys: Buttons { down: space(), just_pressed: space(), ..Default::default() },
            ..Default::default()
        };
        let after = instance.update(pressed).unwrap();
        assert_eq!(after.transform.translation.y, 1.0);
        let held = Frame {
            keys: Buttons { down: space(), ..Default::default() },
            transform: after.transform,
            ..Default::default()
        };
        assert_eq!(instance.update(held).unwrap().transform.translation.y, 1.0);
    }

    #[test]
    fn rejects_other_entities() {
        let source = "function on_update() vs.translate(entity + 1, 1, 0, 0) end";
        let mut instance = Instance::new(source, 1, "test").unwrap();
        assert!(instance.update(Frame::default()).is_err());
    }
}
