//! Does the Lua preview move things exactly like the generated systems?
//!
//! Two Bevy apps run the same scripts: one with the preview components driven by Lua, one with
//! the scripts compiled to systems. Both get the same frames: irregular frame times, keys pressed
//! and held at random, random starting transforms. After every frame the transforms must match
//! bit for bit.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use vs_tests::Wobble;
use vs_bevy::{LuaScript, VisualScript};

mod native {
    include!(concat!(env!("OUT_DIR"), "/scripts_native.rs"));
}

mod preview {
    include!(concat!(env!("OUT_DIR"), "/scripts_preview.rs"));
}

const SEEDS: u64 = 25;
const FRAMES: usize = 300;
const KEYS: [KeyCode; 3] = [KeyCode::Space, KeyCode::KeyW, KeyCode::KeyR];
const BUTTONS: [MouseButton; 2] = [MouseButton::Left, MouseButton::Right];

fn bits(t: &Transform) -> [u32; 10] {
    let [a, b, c] = t.translation.to_array().map(f32::to_bits);
    let [d, e, f, g] = t.rotation.to_array().map(f32::to_bits);
    let [h, i, j] = t.scale.to_array().map(f32::to_bits);
    [a, b, c, d, e, f, g, h, i, j]
}

fn base_app() -> App {
    let mut app = App::new();
    app.add_plugins((TaskPoolPlugin::default(), bevy::time::TimePlugin));
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.register_type::<Wobble>();
    app
}

/// Sets the time, keys and mouse buttons for the next update, as Bevy's input and time plugins
/// would.
fn feed(app: &mut App, delta: Duration, held: &[KeyCode], buttons: &[MouseButton]) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(delta));
    fn set<T: Copy + Eq + std::hash::Hash + Send + Sync + 'static>(app: &mut App, all: &[T], held: &[T]) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<T>>();
        input.clear();
        for b in all {
            if held.contains(b) {
                input.press(*b);
            } else {
                input.release(*b);
            }
        }
    }
    set(app, &KEYS, held);
    set(app, &BUTTONS, buttons);
}

/// Each entity's transform, or `None` once it despawned.
fn transforms(app: &App, entities: &[Entity]) -> Vec<Option<Transform>> {
    entities.iter().map(|e| app.world().get::<Transform>(*e).copied()).collect()
}

/// Presses and releases at random: a held button lets go one frame in ten, a free one goes down
/// one frame in twenty.
fn toggle<T: Copy + PartialEq>(rng: &mut Rng, all: &[T], held: &mut Vec<T>) {
    for b in all {
        if held.contains(b) {
            if rng.below(10) == 0 {
                held.retain(|k| k != b);
            }
        } else if rng.below(20) == 0 {
            held.push(*b);
        }
    }
}

#[test]
fn preview_matches_compiled_systems() {
    let demo_source = include_str!("../assets/scripts/demo.lua");
    let logic_source = include_str!("../assets/scripts/logic.lua");
    let math_source = include_str!("../assets/scripts/math.lua");
    let coverage_source = include_str!("../assets/scripts/coverage.lua");
    let components_source = include_str!("../assets/scripts/components.lua");
    let input_source = include_str!("../assets/scripts/input.lua");
    let despawn_source = include_str!("../assets/scripts/despawn.lua");
    let charged_source = include_str!("../assets/scripts/charged.lua");

    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let starts: Vec<Transform> = (0..8)
            .map(|_| Transform {
                translation: Vec3::new(rng.float(-5.0, 5.0), rng.float(-5.0, 5.0), rng.float(-5.0, 5.0)),
                rotation: Quat::from_euler(EulerRot::XYZ, rng.float(-3.0, 3.0), rng.float(-3.0, 3.0), 0.0),
                scale: Vec3::splat(rng.float(0.5, 2.0)),
            })
            .collect();

        // Compiled: the generated plugin and components, nothing else.
        let mut rust = base_app();
        rust.add_plugins(native::ScriptsPlugin);
        let rust_entities = vec![
            rust.world_mut().spawn((starts[0], native::Demo::default())).id(),
            rust.world_mut().spawn((starts[1], native::Logic::default())).id(),
            rust.world_mut().spawn((starts[2], native::Math::default())).id(),
            rust.world_mut().spawn((starts[3], native::Coverage::default())).id(),
            rust.world_mut().spawn((starts[4], native::Components::default())).id(),
            rust.world_mut().spawn((starts[5], native::Input::default())).id(),
            rust.world_mut().spawn((starts[6], native::Despawn::default())).id(),
            // Requires `Charge`, which it declares, and `Stats`, which `stats.lua` declares.
            rust.world_mut().spawn((starts[7], native::Charged::default())).id(),
        ];

        // Preview: the generated components with Lua attached. The scripts are added as assets
        // directly, so they're ready on the first frame instead of loading from disk.
        let mut lua = base_app();
        lua.add_plugins(bevy::asset::AssetPlugin::default());
        lua.add_plugins(preview::ScriptsPlugin);
        let mut scripts = lua.world_mut().resource_mut::<Assets<LuaScript>>();
        let demo = scripts.add(LuaScript { source: demo_source.to_string() });
        let logic = scripts.add(LuaScript { source: logic_source.to_string() });
        let math = scripts.add(LuaScript { source: math_source.to_string() });
        let coverage = scripts.add(LuaScript { source: coverage_source.to_string() });
        let components = scripts.add(LuaScript { source: components_source.to_string() });
        let input = scripts.add(LuaScript { source: input_source.to_string() });
        let despawn = scripts.add(LuaScript { source: despawn_source.to_string() });
        let charged = scripts.add(LuaScript { source: charged_source.to_string() });
        let lua_entities = vec![
            lua.world_mut().spawn((starts[0], preview::Demo::default(), VisualScript(demo))).id(),
            lua.world_mut().spawn((starts[1], preview::Logic::default(), VisualScript(logic))).id(),
            lua.world_mut().spawn((starts[2], preview::Math::default(), VisualScript(math))).id(),
            lua.world_mut().spawn((starts[3], preview::Coverage::default(), VisualScript(coverage))).id(),
            lua.world_mut().spawn((starts[4], preview::Components::default(), VisualScript(components))).id(),
            lua.world_mut().spawn((starts[5], preview::Input::default(), VisualScript(input))).id(),
            lua.world_mut().spawn((starts[6], preview::Despawn::default(), VisualScript(despawn))).id(),
            lua.world_mut().spawn((starts[7], preview::Charged::default(), VisualScript(charged))).id(),
        ];

        let mut held: Vec<KeyCode> = Vec::new();
        let mut buttons: Vec<MouseButton> = Vec::new();
        let mut despawned_at = None;
        for frame in 0..FRAMES {
            toggle(&mut rng, &KEYS, &mut held);
            toggle(&mut rng, &BUTTONS, &mut buttons);
            // Frame times between 1 and 50 ms, with no pattern.
            let delta = Duration::from_micros(1_000 + rng.below(49_000));
            feed(&mut rust, delta, &held, &buttons);
            feed(&mut lua, delta, &held, &buttons);
            rust.update();
            lua.update();

            let (r, l) = (transforms(&rust, &rust_entities), transforms(&lua, &lua_entities));
            for (i, (r, l)) in r.iter().zip(&l).enumerate() {
                assert_eq!(l.map(|t| bits(&t)), r.map(|t| bits(&t)), "seed {seed}, frame {frame}, entity {i}: Lua {l:?} vs Rust {r:?}");
            }
            if despawned_at.is_none() && r[6].is_none() {
                despawned_at = Some(frame);
            }
            // The game's component, which the script both requires and changes.
            let (r, l) = (rust.world().get::<Wobble>(rust_entities[4]), lua.world().get::<Wobble>(lua_entities[4]));
            assert_eq!(format!("{l:?}"), format!("{r:?}"), "seed {seed}, frame {frame}: Wobble differs");
            // The components scripts declare, changed through reflection in the preview.
            let (r, l) = (rust.world().get::<native::Stats>(rust_entities[7]), lua.world().get::<preview::Stats>(lua_entities[7]));
            assert_eq!(format!("{l:?}"), format!("{r:?}"), "seed {seed}, frame {frame}: Stats differs");
            let (r, l) = (rust.world().get::<native::Charge>(rust_entities[7]), lua.world().get::<preview::Charge>(lua_entities[7]));
            assert_eq!(format!("{l:?}"), format!("{r:?}"), "seed {seed}, frame {frame}: Charge differs");
        }
        // The scripts really did something: otherwise matching would prove nothing.
        assert!(despawned_at.is_some(), "seed {seed}: the despawn script never despawned");
        let moved: Vec<Transform> = transforms(&rust, &rust_entities).into_iter().map(|t| t.unwrap_or_default()).collect();
        assert_ne!(bits(&moved[0]), bits(&starts[0]), "seed {seed}: the demo cube never moved");
        assert_ne!(bits(&moved[1]), bits(&starts[1]), "seed {seed}: the logic cube never moved");
        assert_ne!(bits(&moved[2]), bits(&starts[2]), "seed {seed}: the math cube never moved");
        assert_ne!(bits(&moved[3]), bits(&starts[3]), "seed {seed}: the coverage cube never moved");
        assert_ne!(bits(&moved[4]), bits(&starts[4]), "seed {seed}: the components cube never moved");
        assert_ne!(bits(&moved[5]), bits(&starts[5]), "seed {seed}: the input cube never moved");
        assert_ne!(bits(&moved[7]), bits(&starts[7]), "seed {seed}: the charged cube never moved");
        // Over 300 frames of 1-50 ms the charge passes 3 at least once, so `Stats` changes too.
        assert!(rust.world().get::<native::Stats>(rust_entities[7]).is_some_and(|s| s.power > 1.5), "seed {seed}: Stats unchanged");
        assert!(rust.world().get::<Wobble>(rust_entities[4]).is_some_and(|w| w.amount != 0.0), "seed {seed}: Wobble unchanged");
    }
}

/// The coverage script uses every pure node, so the test above checks all of them. It's written
/// by `write_scripts` from the catalog; this fails when a node was added since it last ran.
#[test]
fn coverage_script_uses_every_node() {
    let graph = vs_graph::load_from_str(include_str!("../assets/scripts/coverage.lua")).unwrap();
    let missing: Vec<&str> = vs_graph::vs_nodes::catalog()
        .iter()
        .map(|n| n.kind)
        .filter(|kind| !graph.nodes.iter().any(|n| n.kind == *kind))
        .collect();
    assert!(
        missing.is_empty(),
        "coverage.lua doesn't use {missing:?}: run `cargo run -p vs_tests --example write_scripts`"
    );
}

/// `.bevy-editor/components.json` is what the preview writes for the game's components, so the
/// build and the editor know them before any preview run.
#[test]
fn project_component_list_is_current() {
    let mut app = App::new();
    app.register_type::<Wobble>();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let config = vs_bevy::catalog::Config::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    let described = vs_bevy::catalog::describe(&registry, "vs_tests", &config.components);
    let written: serde_json::Value =
        serde_json::from_str(include_str!("../.bevy-editor/components.json")).unwrap();
    assert_eq!(serde_json::to_value(&described).unwrap(), written);
}

/// The pairs of systems Bevy's ambiguity detection reports for the compiled scripts are the
/// pairs `vs_graph::access` predicts from the IR, conflicting on the same components. Each pair is
/// written `script::system` both ways, with its components by short name.
#[test]
fn predicted_ambiguities_match_bevy() {
    type Pair = (String, String, Vec<String>);
    fn pair(a: String, b: String, mut components: Vec<String>) -> Pair {
        components.sort();
        if a <= b { (a, b, components) } else { (b, a, components) }
    }
    // The last two segments of a path: `differential::native::demo::update_demo` → `demo::update_demo`.
    fn tail(path: &str, n: usize) -> String {
        let parts: Vec<&str> = path.split("::").collect();
        parts[parts.len().saturating_sub(n)..].join("::")
    }

    let mut app = base_app();
    app.add_plugins(native::ScriptsPlugin);
    let world = app.world_mut();
    let mut reported: Vec<Pair> = {
        let mut schedule = world.resource_mut::<Schedules>().remove(Update).expect("the scripts add systems to Update");
        schedule.initialize(world).unwrap();
        // Initializing moves the systems out of the graph, so they're named from the schedule.
        let system_names: std::collections::HashMap<_, _> =
            schedule.systems().unwrap().map(|(key, system)| (key, system.name().to_string())).collect();
        schedule
            .graph()
            .conflicting_systems()
            .0
            .iter()
            .map(|(a, b, ids)| {
                let components =
                    ids.iter().map(|id| tail(&world.components().get_name(*id).unwrap().to_string(), 1)).collect();
                pair(tail(&system_names[a], 2), tail(&system_names[b], 2), components)
            })
            .collect()
    };

    // The same scripts, lowered as `build.rs` does.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    vs_graph::components::load_project(root).unwrap();
    let mut files: Vec<_> = std::fs::read_dir(root.join("assets/scripts"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "lua"))
        .collect();
    files.sort();
    let names: Vec<_> = files.iter().map(|f| vs_graph::rust::names(&f.to_string_lossy())).collect();
    let scripts: Vec<Vec<vs_graph::access::System>> = files
        .iter()
        .zip(&names)
        .map(|(file, names)| {
            let graph = vs_graph::load_from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
            vs_graph::access::systems(&vs_graph::lower(&graph).module, names)
        })
        .collect();
    let at = |at: vs_graph::access::At| format!("{}::{}", names[at.script].module, scripts[at.script][at.system].name);
    let mut predicted: Vec<Pair> = vs_graph::access::ambiguities(&scripts)
        .into_iter()
        .map(|a| pair(at(a.a), at(a.b), a.components))
        .collect();

    reported.sort();
    predicted.sort();
    assert_eq!(predicted, reported);
    // Several scripts move their entity, and `watcher.lua` reads a declared component and a game
    // component that others write, so the rule is checked beyond `Transform`.
    let on = |c: &str| reported.iter().filter(|(_, _, cs)| cs.iter().any(|x| x == c)).count();
    assert!(on("Transform") > 10, "{reported:?}");
    assert!(reported.contains(&pair("charged::update_charged".into(), "watcher::setup_watcher".into(), vec!["Stats".into()])));
    assert!(reported.contains(&pair("components::update_components".into(), "watcher::update_watcher".into(), vec!["Wobble".into()])));
}

/// Small deterministic generator, so a failing seed reproduces.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn float(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() >> 40) as f32 / (1u64 << 24) as f32 * (hi - lo)
    }
}
