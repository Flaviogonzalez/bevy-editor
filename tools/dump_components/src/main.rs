//! Writes `crates/vs_graph/bevy_components.json`, the Bevy components the editor offers Get / Set
//! nodes for.
//!
//! Which components is decided here, by hand: the ones a game commonly changes from a script,
//! all in `bevy::prelude`, so exported code names them as a person would. Their fields come from
//! Bevy's type registry, so they always match the Bevy version in use. Run it again after
//! updating Bevy, and review the diff:
//!
//! ```sh
//! cargo run -p dump_components
//! ```

use bevy::prelude::*;

/// Components scripts can use, by the name `bevy::prelude` gives them.
const COMPONENTS: &[&str] = &[
    // The entity itself.
    "Transform",
    // Lights.
    "PointLight",
    "SpotLight",
    "DirectionalLight",
    "AmbientLight",
    "DistanceFog",
    // Cameras.
    "Camera",
    // 2D.
    "Sprite",
    "Text2d",
    // Text and UI.
    "Text",
    "TextColor",
    "TextShadow",
    "BackgroundColor",
    "BorderColor",
    "ImageNode",
    "Outline",
    "Node",
];

fn main() {
    // Bevy registers every reflected type when the app is created; the plugins add the rest.
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().disable::<bevy::winit::WinitPlugin>());
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let described = vs_bevy::components::describe_named(&registry, COMPONENTS);
    for name in COMPONENTS {
        if !described.iter().any(|d| d.name == *name) {
            eprintln!("skipped {name}: not a registered component, or no field a pin can carry");
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/vs_graph/bevy_components.json");
    let json = serde_json::to_string_pretty(&described).expect("components serialize");
    std::fs::write(path, json + "\n").expect("the file is writable");
    let fields: usize = described.iter().map(|d| d.fields.len()).sum();
    println!("wrote {} components, {fields} fields, to {path}", described.len());
}
