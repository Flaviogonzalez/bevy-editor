# bevy-editor

A visual scripting editor for Bevy. You wire events, flow control and engine calls together as a node graph. The same graph runs two ways: as Lua, for a live preview that reloads when you save, and as Rust generated at build time, for shipped builds. A differential test checks, frame by frame and bit for bit, that both move things the same way. The aim is shipped builds that look hand-written, with no trace of visual scripting (see [where this is going](#where-this-is-going-shipped-builds-with-no-trace-of-visual-scripting)).

The file format follows Spartan Engine's visual scripting: the graph is saved *inside* the script file, as JSON in a Lua block comment, followed by the generated Lua. Lua ignores the comment, so the file runs as-is, and the editor reads the graph back from it. The graph is the source of truth, so saving rewrites the code.

```lua
-- A visual script. bevy-editor generated this file from the graph below.
--[==[bevy-visual-script
{ "version": 1, "nodes": [...], "links": [...], "variables": [...] }
]==]

local vars = {
    90, -- speed
}
local state = {}

function on_update()
    -- On Update [1]
    -- Rotate [4]
    do
        local t0 = node.math.mul(vars[1], vs.delta())
        vs.rotate(entity, 0, t0, 0)
    end
end
```

Every generated step carries a `-- Title [node id]` comment that points back to its node.

## What comes preloaded

A node graph only beats typing code when the editor already knows the engine. So the editor ships with Bevy's everyday vocabulary, and none of it is written by hand node by node.

**Values.** Besides numbers, booleans and text, pins carry vectors (`Vec3`), rotations (`Quat`) and colors (`Color`). Inline fields take `1, 2, 3` for a vector, Euler angles in degrees for a rotation, and `#ff8800` or `1, 0.5, 0, 1` (sRGB) for a color. Any value plugs into a text pin.

**Bevy's math.** Every method of `Vec3` and `Quat` whose inputs and result fit pins is a node: 96 of them, with glam's own documentation as their descriptions. [`tools/gen_math`](tools/gen_math/src/main.rs) reads the source of the exact glam version `bevy_math` uses and writes [`glam_nodes.rs`](crates/vs_nodes/src/glam_nodes.rs). The file is checked in, so run it again only after updating Bevy. In exported code they read as method calls (`transform.translation.distance(target)`).

**Components.** Get and Set nodes reach any field of a component that a pin can carry. The menu lists them as `Get PointLight.intensity`, `Set Sprite.color`, and so on. They come from two places:

- Bevy's common components (transform, lights, camera, sprite, text, UI colors), listed in [`bevy_components.json`](crates/vs_graph/bevy_components.json). [`tools/dump_components`](tools/dump_components/src/main.rs) generates it from Bevy's type registry.
- The project's catalog, which the game itself writes to `.bevy-editor/components.json`, since only the running game knows every component (its own and its plugins'). `.bevy-editor/catalog.toml` says which: by default the game's own (`crate::*`), and any others by name or pattern (`"*Light"`, `"RigidBody"`), minus `exclude`. A component needs `#[reflect(Component, Default)]`. In the project panel, **Refresh catalog** builds the game and runs it with no window just to write the file, and **Edit catalog** opens the config. A normal preview writes it too. Commit both files, like a lock file, so a fresh clone builds before anyone runs the game.

Exported code names the game's components `crate::…` and Bevy's prelude ones by their short name. For any other component it guesses (`bevy::light::RectLight`, `avian3d::RigidBody`); when a guess is wrong, `[components.paths]` in the config sets the right path.

A script's component *requires* every component it uses (`#[require(PointLight)]`), so the entity always has them and Get never fails. In exported code a field is a query item and a plain assignment (`point_light.intensity += 10.0`). In the preview, the same access goes through reflection.

**Input and entity.** Key and mouse button state, each as held, just pressed or just released, and Despawn.

## How the two backends stay equal

1. **Each node is written once, as a Rust function** ([`vs_nodes`](crates/vs_nodes/src/lib.rs)). The `nodes!` macro registers it for reflection and generates a typed Lua binding for it. The editor's node list for math, logic and text comes from that reflection, so a new function there is a new node with no other change.
2. **A graph lowers to an IR** ([`ir.rs`](crates/vs_graph/src/ir.rs), [`lower.rs`](crates/vs_graph/src/lower.rs)) with Rust semantics: every node call is its own step in a fixed order, numbers are `f32`, Flip Flop and Do Once state lives in slots. Problems are found here, once.
3. **The Lua backend** ([`lua.rs`](crates/vs_graph/src/lua.rs)) only writes glue. Every computation calls Rust: `node.*` are the `vs_nodes` functions, `vs.*` go through [`vs_nodes::fx`](crates/vs_nodes/src/fx.rs). Lua's own `f64` arithmetic never runs, because it gives different answers (`2^24 + 1 + 1`).
4. **The Rust backend** ([`rust.rs`](crates/vs_graph/src/rust.rs)) builds code with `syn`/`quote` and formats it with `prettyplease`: per script, a component, a plugin and one Bevy system per event. Node bodies are pasted in as code (`nodes!` keeps each body as text), with their arguments substituted, so the output doesn't call `vs_nodes`. Calls whose inputs are all constants are evaluated at generation time by calling the very same function.

[`experiments/reflect_nodes`](experiments/reflect_nodes) has the measurements behind this design.

## Shipped builds with no trace of visual scripting

The goal is that a shipped game looks hand-written. It shouldn't contain a Lua runtime, graph files, `vs_*` crates, or code that reads as generated. Visual scripting is a way to author the game, not something the game depends on. Steps 1 to 5 of the [plan](#next-steps) are done: **Build** in the editor (or `vs build`) writes a plain Bevy project and builds it. A script like the demo exports as roughly this:

```rust
use bevy::prelude::*;

#[derive(Component)]
pub struct Spinner {
    pub speed: f32,
    pub big: bool,
}

impl Default for Spinner {
    fn default() -> Self {
        Self { speed: 90.0, big: false }
    }
}

pub struct SpinnerPlugin;

impl Plugin for SpinnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spin, toggle_size.run_if(input_just_pressed(KeyCode::Space))).chain());
    }
}

fn spin(time: Res<Time>, mut query: Query<(&mut Transform, &Spinner)>) {
    for (mut transform, spinner) in &mut query {
        let step = spinner.speed * time.delta_secs();
        let rotation = Quat::from_euler(EulerRot::XYZ, 0.0, step.to_radians(), 0.0);
        transform.rotation = (transform.rotation * rotation).normalize();

        let bob = (time.elapsed_secs() * 2.0).sin() * 0.25 + 1.0;
        transform.translation = Vec3::new(0.0, bob, 0.0);
    }
}

fn toggle_size(mut query: Query<(&mut Transform, &mut Spinner)>) {
    for (mut transform, mut spinner) in &mut query {
        spinner.big = !spinner.big;
        transform.scale = if spinner.big { Vec3::splat(1.6) } else { Vec3::ONE };
    }
}
```

How it works:

- **The game spawns the generated component, in both modes.** Game code writes `Spinner::default()`, never `VisualScript("scripts/spinner.lua")`. In development, a preview plugin (behind a feature) finds entities with `Spinner` and drives them from the Lua file, with live reload. The shipped build doesn't have that plugin, so `Spinner` is just a component with its systems. The game's own code is the same either way.
- **Node bodies are inlined.** `nodes!` keeps each function's body, and the Rust backend pastes it in with the arguments substituted (`a * b`, `x.sin()`), building the expression as a syntax tree so precedence holds. Each node is still defined once. Lua still calls the compiled function, so the two stay equal. Intrinsics are emitted as the same math `fx` does (`Quat::from_euler(..)`, then `normalize()`).
- **Events become Bevy systems.** Update becomes a system with `Res<Time>` and a `Query` that asks only for what its steps use. A key event becomes a system with `run_if(input_just_pressed(..))`, and Start becomes a system over `Added<T>`. `.chain()` keeps the order the graph defines.
- **Names come from the graph.** Variables have names, and so can nodes: the side panel has a Name field for the selected node. A named node, or a value used more than once, gets a `let` with that name; a value used once is nested in place. Flip Flop and Do Once state takes the node's name too. The output only reads as hand-written if the graph is well named.
- **Shipping is an export, not a `build.rs`.** Export writes a clean copy of the project to `<project>/export`. The generated code goes into `src/scripts/` as ordinary files. `build.rs`, the `vs_*` dependencies, the `preview` feature, the Bevy features only the editor needs, `assets/scripts/` and the editor's marker file are left out, and `Cargo.lock` is pruned. The copy builds with plain `cargo build --release` anywhere. The working project keeps the visual scripting setup.
- **Equality stays tested.** The differential test runs the generated systems and the Lua preview in two minimal `App`s, with manual time and a scripted `ButtonInput`, and compares the transforms bit for bit every frame.

One limit of the preview: the Lua script keeps its own copy of the variables, so changing a component field from game code (`spinner.speed = 10.0`) affects the exported build but not the preview.

**The graph stays the source; exported code is an artifact.** Keeping a graph and hand-edited Rust in sync both ways would need Rust → graph parsing, and the result would be unpredictable, so that isn't planned. What is predictable is a one-way **eject** per script. An ejected script's generated `.rs` moves into the working project as ordinary source, and its graph is archived. From then on that script is hand-written Rust, and the rest stay visual. Because export already produces hand-written-looking code, eject is just export applied to one script, kept in place.

## Layout

| Crate | What it holds |
| ----- | ------------- |
| [`crates/vs_nodes`](crates/vs_nodes) | The node definitions and intrinsics both runtimes call |
| [`crates/vs_graph`](crates/vs_graph) | The graph model, the node library, the IR, both backends and the file format. No UI dependency |
| [`crates/vs_editor`](crates/vs_editor) | The editor app, built on [gpui-kit](https://github.com/longbridge/gpui-kit) (GPUI and gpui-component) |
| [`crates/vs_bevy`](crates/vs_bevy) | The Lua runtime and the preview plugin that attaches it to generated components |
| [`crates/vs_project`](crates/vs_project) | Projects: settings, the project template, export, release builds, and the `vs` command |
| [`crates/vs_tests`](crates/vs_tests) | The differential test, its scripts and the game-side code they expect |
| [`tools/gen_math`](tools/gen_math) | Writes the bevy_math nodes from glam's source |
| [`tools/dump_components`](tools/dump_components) | Writes the bundled component list from Bevy's type registry |

## Running

```sh
cargo run --release                                    # the editor
cargo test -p vs_nodes --features lua -p vs_graph -p vs_bevy -p vs_project
cargo test -p vs_tests                                 # the differential test, without Bevy's renderer
cargo run -p vs_tests --example write_scripts          # after changing nodes: regenerates the test scripts
cargo run -p gen_math                                  # after a Bevy update: the math nodes
cargo run -p dump_components                           # after a Bevy update: the bundled component list
```

The differential test runs seven scripts. One of them, `coverage.lua`, is generated from the node catalog and uses every pure node twice, once wired and once with literals only (so constant folding is checked too). Another test fails when a node is missing from it.

Builds are limited to 6 parallel jobs and skip debug info for dependencies ([`.cargo/config.toml`](.cargo/config.toml), the workspace `Cargo.toml`), because a full Bevy build at full parallelism froze the machine. Projects made by the editor get the same settings, and cargo runs started by the editor run at below-normal priority.

## Projects

The editor keeps the games it makes in one folder, `Documents/Bevy Projects` unless you change it in **Settings** (saved in your config folder, `bevy-editor/config.json`). **New project** asks for a name and creates a Bevy project there:

```
my_game/
  Cargo.toml            bevy, plus vs_bevy behind the `preview` feature and vs_graph for build.rs
  build.rs              turns assets/scripts/*.lua into Rust
  src/main.rs           ordinary Bevy code: spawns a cube with `Spinner::default()`
  src/scripts.rs        includes the generated code
  assets/scripts/spinner.lua
  bevy-editor.json      marks the folder as an editor project
```

Projects share one build folder (`<projects folder>/.target`), so the second project doesn't rebuild Bevy. The side panel lists the project's scripts; **+ New script** adds one, and the status line tells you which component to spawn for it.

**▶ Preview** saves the open script and runs `cargo run --features preview`. The game spawns the generated components and the preview plugin drives them from the `.lua` files, so saving a script while the game runs reloads it. **Build** exports the project, runs `cargo build --release` on the export, and copies the executable and its assets to `<project>/dist`, ready to ship. Both show cargo's output in the side panel, and **■ Stop** ends them. The code panel switches between the Lua the preview runs and the Rust the build gets.

The same commands work from a terminal:

```sh
cargo run -p vs_project --bin vs -- config [<folder>]       # show or set the projects folder
cargo run -p vs_project --bin vs -- new <name> [--in <folder>]
cargo run -p vs_project --bin vs -- export <project>
cargo run -p vs_project --bin vs -- build <project>
```

## Scripts in a Bevy app

Game code only sees the generated components. In a project's `main.rs`:

```rust
mod scripts;
use scripts::Spinner;

app.add_plugins((DefaultPlugins, scripts::ScriptsPlugin));
commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default(), Spinner::default()));
```

Without the `preview` feature, `ScriptsPlugin` adds each script's systems. With it, `ScriptsPlugin` adds `vs_bevy::VisualScriptPlugin` instead and gives every entity with `Spinner` a `VisualScript` pointing at `scripts/spinner.lua`. Each such entity gets its own Lua 5.4 state. `on_script_loaded` runs on the first frame the file is loaded, then `on_update` runs every frame in `Update`. A script that raises an error is logged and stopped until its file changes. Saving a file restarts every script that uses it, from a fresh state.

The plugin embeds Lua through [`mlua`](https://github.com/mlua-rs/mlua) rather than [`bevy_mod_scripting`](https://github.com/makspll/bevy_mod_scripting). The API is small enough to bind by hand, and `bevy_mod_scripting` 0.22 and later require Bevy 0.20, while the games stay on Bevy 0.19 for now.

## Using the editor

Every command is in the menu bar (File, Edit, View, Project), which also shows its shortcut. Hover an icon button to see what it does.

| Action | How |
| ------ | --- |
| Add a node | Right-click empty canvas, or press Tab or Space. Type to filter, use the arrow keys, Enter to place |
| Connect | Drag from a pin to a pin on the other side. Dragging a connected input picks its wire back up |
| Add a connected node | Drop a wire on empty canvas: the menu lists only nodes that fit |
| Edit an inline value | Click the field, type, Enter. Boolean fields toggle on click |
| Move | Drag a node's header or body |
| Select | Click, Shift or Ctrl click to add, drag a box on empty canvas |
| Pan / zoom | Right or middle drag / mouse wheel |
| Frame | F, or View → Frame all |
| Delete | Delete or Backspace |
| Duplicate, select all | Ctrl+D, Ctrl+A |
| Undo, redo | Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z |
| New, open, save, save as | Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S |
| Name a node | Select it, click its Name field in the side panel |
| New / open project, settings | File menu; a dialog takes the name, Enter confirms, Escape cancels |
| Preview, build, stop | F5, Ctrl+B, Shift+F5, or the Project menu and the toolbar's run buttons |

The Explorer on the left shows the open project, its scripts and its catalog of components, and the graph's variables (click a name or value to edit it, the type to change it, and ⋯ to add Get / Set nodes or delete it). The Inspector on the right shows the selected node: what it does, its name and its pins. The panel under the canvas (Ctrl+J) has three tabs: the problems the compiler found (click one to jump to its node), the output of the last preview or build, which it switches to when one starts, and the generated Lua or Rust. Drag the line between two panels to resize them; resizing the window only changes the canvas.

## The Lua runtime contract

The generated Lua expects the host to define these globals. `vs_bevy` provides them ([`runtime.rs`](crates/vs_bevy/src/runtime.rs)). For now a script can only reach its own entity: passing any other entity is an error.

| Global | Meaning |
| ------ | ------- |
| `entity` | The entity the script is attached to |
| `on_script_loaded()` | Called once when the script loads. Generated from On Start nodes |
| `on_update()` | Called every frame. Generated from On Update and On Key Pressed nodes |
| `node.<category>.<name>(...)` | The pure nodes, from `vs_nodes::lua_table` |
| `vs.delta()` | Frame time in seconds |
| `vs.time()` | Seconds since startup |
| `vs.input(device, state, name)` | `device` is `"key"` or `"mouse"`, `state` is `"down"`, `"pressed"` or `"released"`, and `name` a `KeyCode` or `MouseButton` variant name (`Space`, `KeyW`, `Left`) |
| `vs.get(entity, component, path)`, `vs.set(entity, component, path, value)` | A component field by reflection path, such as `"PointLight", "intensity"` |
| `vs.despawn(entity)` | Despawn the entity after this update |
| `vs.vec3(x, y, z)`, `vs.quat_euler(x, y, z)`, `vs.color(r, g, b, a)` | Value literals. Vectors, rotations and colors are userdata wrapping the Rust values |
| `vs.position(entity)` | A table with `x`, `y`, `z` |
| `vs.translate(entity, x, y, z)` | Move by an offset |
| `vs.rotate(entity, x, y, z)` | Rotate by Euler angles in degrees |
| `vs.set_position(entity, x, y, z)`, `vs.set_scale(entity, x, y, z)` | Set the transform |

## Versions and wgpu

The editor is built on [gpui-kit](https://github.com/longbridge/gpui-kit) 0.9, which bundles GPUI (`gpui-pre`, a snapshot of Zed's GPUI, pinned to an exact version), its platform layer, and the [gpui-component](https://github.com/longbridge/gpui-component) library. GPUI is updated by updating gpui-kit, not on its own.

`gpui-pre` 0.3.11 renders with wgpu 30 on Linux and the web; on Windows it uses its own native renderer, with no wgpu. Bevy 0.19, which the games and `vs_bevy` use, is on wgpu 29, and Bevy 0.20 on wgpu 30. So a Bevy viewport inside the editor that shares GPUI's wgpu device would need Bevy 0.20, and would only work where GPUI draws with wgpu. Nothing depends on that today: the editor runs as its own process and talks to the game only through the `.lua` files it writes.

## Next steps

Toward trace-free builds, in this order (each step keeps the differential test green):

1. ~~**Component-first API.**~~ Done: the game spawns `Spinner`, and the `preview` feature attaches Lua to it.
2. ~~**Systems instead of `run(frame)`.**~~ Done: one system per event, chained; the differential test runs two minimal `App`s.
3. ~~**Inline node bodies.**~~ Done: generated code doesn't depend on `vs_nodes`.
4. ~~**Names.**~~ Done: node names in the editor and the file format, readable locals, nested single-use expressions.
5. ~~**Export.**~~ Done: `vs export` / `vs build` and the editor's Preview and Build buttons.
6. **Eject**, per script: move a script's generated `.rs` into the working project's `src/` and archive its graph.

Also pending:

- ECS beyond the script's own entity: other entities as values, spawning, events.
- Export the game's node catalog to the editor, so nodes a game defines with `nodes!` show up without rebuilding the editor (today the editor links `vs_nodes` directly).
- Keep variable values across a hot reload.
- Copy / paste, comments, node search by pin type, a real text field instead of the key-capture inline editor.
