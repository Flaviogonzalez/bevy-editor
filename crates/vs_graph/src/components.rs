//! The components scripts can read and write, and their fields.
//!
//! Two sources. Bevy's common components come bundled ([`BUNDLED`]): which ones is a curated
//! list, and their fields are read from Bevy's type registry by
//! `tools/dump_components`. The project's catalog is written by the game itself
//! when it runs in preview, into [`PROJECT_FILE`]: its own components, plus any others its
//! [`CONFIG_FILE`] asks for. [`load_project`] reads it.
//!
//! A field is named `Component.path`, such as `PointLight.intensity` or `Sprite.color`.

use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::library::PinType;

/// The bundled catalog of Bevy components.
pub const BUNDLED: &str = include_str!("../bevy_components.json");

/// Where a game writes its catalog of components, relative to its root.
pub const PROJECT_FILE: &str = ".bevy-editor/components.json";

/// Which components the game describes, relative to its root. `vs_bevy::catalog` reads it.
pub const CONFIG_FILE: &str = ".bevy-editor/catalog.toml";

/// Set (to anything) when running the game only to write its catalog: it starts with its window
/// hidden, writes the catalog and exits. `vs_bevy::catalog::ONLY_ENV` is the same name.
pub const CATALOG_ENV: &str = "BEVY_EDITOR_CATALOG_ONLY";

/// The catalog config new projects start with, and that the editor writes when asked to edit a
/// missing one.
pub const DEFAULT_CONFIG: &str = r#"# Which components the editor offers Get / Set nodes for, besides Bevy's common ones.
# The game reads this when it starts in preview and writes .bevy-editor/components.json.
# In the editor, "Refresh catalog" runs it without opening a window.
#
# A component is listed when it's reflected with `#[reflect(Component, Default)]` and matches an
# `include` pattern but no `exclude` one. Patterns are short type names, `*` matches anything:
#   "crate::*"     the game's own components
#   "PointLight"   one component, from any crate
#   "*Light"       every component whose name ends in Light
[components]
include = ["crate::*"]
exclude = []

# How many levels of nested fields to list: 3 reaches `a.b.c`, 1 only the component's own fields.
depth = 3

# How exported code names a component that isn't the game's own, when the guess
# (`bevy::<crate>::<Name>`, `<crate>::<Name>`) is wrong:
[components.paths]
# NotShadowCaster = "bevy::light::NotShadowCaster"
"#;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// The short name scripts use, such as `PointLight`.
    pub name: String,
    /// How exported code names the type: `PointLight` for Bevy's prelude types, `crate::…` for
    /// the game's own.
    pub rust_path: String,
    pub fields: Vec<FieldInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldInfo {
    /// The reflection path inside the component, such as `intensity` or `0`.
    pub path: String,
    #[serde(rename = "type")]
    pub ty: PinType,
}

struct Store {
    bundled: Vec<ComponentInfo>,
    project: Vec<ComponentInfo>,
}

fn store() -> &'static RwLock<Store> {
    static STORE: OnceLock<RwLock<Store>> = OnceLock::new();
    STORE.get_or_init(|| {
        let bundled = serde_json::from_str(BUNDLED).expect("the bundled catalog parses");
        RwLock::new(Store { bundled, project: Vec::new() })
    })
}

/// Every known component: the bundled ones, then the project's. A project entry for a bundled
/// component (asked for to get deeper fields, say) takes its fields but keeps the bundled name for
/// code, which is the prelude's (`PointLight`, not a guessed `bevy::light::PointLight`).
pub fn all() -> Vec<ComponentInfo> {
    let store = store().read().unwrap();
    let mut out = store.bundled.clone();
    for c in &store.project {
        match out.iter_mut().find(|known| known.name == c.name) {
            Some(known) => known.fields = c.fields.clone(),
            None => out.push(c.clone()),
        }
    }
    out
}

pub fn find(name: &str) -> Option<ComponentInfo> {
    all().into_iter().find(|c| c.name == name)
}

/// The project's components, as its catalog lists them.
pub fn project() -> Vec<ComponentInfo> {
    store().read().unwrap().project.clone()
}

/// Replaces the project's components.
pub fn set_project(components: Vec<ComponentInfo>) {
    store().write().unwrap().project = components;
}

/// Reads the catalog the game wrote into `root/.bevy-editor/components.json`, replacing the
/// previous project's. None written yet means none. Returns how many components it lists.
pub fn load_project(root: &std::path::Path) -> Result<usize, String> {
    let path = root.join(PROJECT_FILE);
    let list: Vec<ComponentInfo> = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?,
        Err(_) => Vec::new(),
    };
    let n = list.len();
    set_project(list);
    Ok(n)
}

/// `PointLight.intensity` → (`PointLight`, `intensity`).
pub fn split(field: &str) -> Option<(&str, &str)> {
    field.split_once('.')
}

/// The component and field info for `Component.path`.
pub fn field(field: &str) -> Option<(ComponentInfo, FieldInfo)> {
    let (name, path) = split(field)?;
    let component = find(name)?;
    let info = component.fields.iter().find(|f| f.path == path)?.clone();
    Some((component, info))
}

/// [`load_project`] by its old name: projects created before it was renamed call this from their
/// `build.rs`, which is theirs and isn't regenerated.
pub fn add_from_project(root: &std::path::Path) -> Result<usize, String> {
    load_project(root)
}
