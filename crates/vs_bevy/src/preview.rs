//! Preview mode: a game's generated components, driven by their `.lua` files.
//!
//! In a game's development build, `vs_graph::rust::emit_dev` (preview mode) generates only the
//! components, plus a plugin that calls [`register`] for each. The game spawns those components
//! as it would in a shipped build; this attaches the matching script, so the Lua runs and reloads
//! when the editor saves. A shipped build compiles the same components with their systems
//! instead, and doesn't depend on this crate.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::catalog;

use crate::VisualScript;

/// Runs the script at `path` (such as `scripts/spinner.lua`) on every entity with a `C`.
pub fn register<C: Component>(app: &mut App, path: &'static str) {
    app.add_systems(PreUpdate, move |mut commands: Commands, assets: Res<AssetServer>, query: Query<Entity, (With<C>, Without<VisualScript>)>| {
        for entity in &query {
            commands.entity(entity).insert(VisualScript(assets.load(path)));
        }
    });
}

/// Writes the game's catalog of components (see [`crate::catalog`]) into
/// `<root>/.bevy-editor/components.json` once the app has started, so the editor can offer Get /
/// Set nodes for them. Generated preview code calls this.
///
/// With [`catalog::ONLY_ENV`] set, the game runs just for this: its window stays hidden and it
/// exits once the catalog is written, failing if it couldn't be.
pub fn dump_components(app: &mut App, package: &'static str, root: &'static str) {
    // The package name, not the crate's: a test or example of the game includes this too.
    let crate_name = package.replace('-', "_");
    let only = std::env::var_os(catalog::ONLY_ENV).is_some();
    if only {
        // The primary window exists once `WindowPlugin` is built; winit opens it later.
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        for mut window in windows.iter_mut(world) {
            window.visible = false;
        }
    }
    app.add_systems(Startup, move |registry: Res<AppTypeRegistry>, mut exit: MessageWriter<AppExit>| {
        let result = catalog::write(&registry.read(), &crate_name, std::path::Path::new(root));
        match &result {
            Ok(n) => info!(target: "visual_script", "catalog: {n} component(s) available to scripts"),
            Err(e) => error!(target: "visual_script", "couldn't write the catalog for the editor: {e}"),
        }
        if only {
            exit.write(if result.is_ok() { AppExit::Success } else { AppExit::error() });
        }
    });
}
