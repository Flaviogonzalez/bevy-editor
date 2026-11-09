//! The components scripts can read and write, and their fields.
//!
//! Two sources. Bevy's common components come bundled ([`BUNDLED`]): which ones is a curated
//! list, and their fields are read from Bevy's type registry by
//! `tools/dump_components`. The project's catalog is written by the game itself
//! when it runs in preview, into [`PROJECT_FILE`]: its own components, plus any others its
//! [`CONFIG_FILE`] asks for. [`load_project`] reads it.
//!
//! A third source needs no preview run: the components the project's scripts declare on their
//! Define Component cards. [`load_project`] reads those too, from the script files, and a script
//! always knows the ones it declares itself ([`field_in`]).
//!
//! A field is named `Component.path`, such as `PointLight.intensity` or `Sprite.color`.

use std::path::Path;
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::graph::{ComponentDef, Graph, Value};
use crate::library::PinType;

/// Where a game's scripts live, relative to its root.
pub const SCRIPTS_DIR: &str = "assets/scripts";

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
    /// The script that declares it, by its path in [`SCRIPTS_DIR`] (`stats.lua`,
    /// `player/stats.lua`); `None` for the bundled and the game's own Rust components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldInfo {
    /// The reflection path inside the component, such as `intensity` or `0`.
    pub path: String,
    #[serde(rename = "type")]
    pub ty: PinType,
    /// The starting value, known for the fields of declared components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

struct Store {
    bundled: Vec<ComponentInfo>,
    project: Vec<ComponentInfo>,
    declared: Vec<ComponentInfo>,
}

fn store() -> &'static RwLock<Store> {
    static STORE: OnceLock<RwLock<Store>> = OnceLock::new();
    STORE.get_or_init(|| {
        let bundled = serde_json::from_str(BUNDLED).expect("the bundled catalog parses");
        RwLock::new(Store { bundled, project: Vec::new(), declared: Vec::new() })
    })
}

/// Every known component: the bundled ones, the project's, then the ones its scripts declare.
/// A project entry for a bundled component (asked for to get deeper fields, say) takes its fields
/// but keeps the bundled name for code, which is the prelude's (`PointLight`, not a guessed
/// `bevy::light::PointLight`). A declared component replaces a project entry of the same name:
/// the preview lists it too, once it's reflected, but with a guessed path.
pub fn all() -> Vec<ComponentInfo> {
    let store = store().read().unwrap();
    let mut out = store.bundled.clone();
    for c in &store.project {
        match out.iter_mut().find(|known| known.name == c.name) {
            Some(known) => known.fields = c.fields.clone(),
            None => out.push(c.clone()),
        }
    }
    for c in &store.declared {
        match out.iter_mut().find(|known| known.name == c.name) {
            Some(known) => *known = c.clone(),
            None => out.push(c.clone()),
        }
    }
    out
}

/// Whether `name` is one of Bevy's bundled components.
pub fn is_bundled(name: &str) -> bool {
    store().read().unwrap().bundled.iter().any(|c| c.name == name)
}

/// A declared component as the catalog lists it. Its Rust path is its bare name: the generated
/// code re-exports it at the root of the scripts, and a script of another module imports it from
/// there (see [`crate::rust`]).
pub fn declared_info(def: &ComponentDef) -> ComponentInfo {
    ComponentInfo {
        name: def.name.clone(),
        rust_path: def.name.clone(),
        fields: def
            .fields
            .iter()
            .map(|f| FieldInfo { path: f.name.clone(), ty: f.value.ty(), default: Some(f.value.clone()) })
            .collect(),
        source: None,
    }
}

/// Replaces the components the project's scripts declare.
pub fn set_declared(components: Vec<ComponentInfo>) {
    store().write().unwrap().declared = components;
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

/// Reads the catalog the game wrote into `root/.bevy-editor/components.json`, and the components
/// the scripts in `root/assets/scripts` declare, replacing the previous project's. None written
/// yet means none. Returns how many components the two list.
///
/// A game's `build.rs` calls this before compiling any script, so a script can use a component
/// another one declares. Two components that would get the same type name are an error.
pub fn load_project(root: &Path) -> Result<usize, String> {
    let path = root.join(PROJECT_FILE);
    let list: Vec<ComponentInfo> = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?,
        Err(_) => Vec::new(),
    };
    let declared = scan_declared(&root.join(SCRIPTS_DIR))?;
    let n = list.len() + declared.len();
    set_project(list);
    set_declared(declared);
    Ok(n)
}

/// The components the scripts in `dir` declare. Files that don't parse are skipped: compiling
/// them reports why.
///
/// Every generated component is re-exported beside the script modules, so each type name may
/// appear once: a script's own component (`health.lua` → `Health`, unless it only declares) and
/// each declared one.
fn scan_declared(dir: &Path) -> Result<Vec<ComponentInfo>, String> {
    let files = crate::script_files(dir);
    let names: Vec<String> = files.iter().map(|f| crate::script_name(dir, f)).collect();
    if let Some(clash) = crate::rust::module_clash(&names) {
        return Err(clash);
    }
    let mut out = Vec::new();
    // Each type name and what generates it, for the message on a clash.
    let mut owners: Vec<(String, String)> = Vec::new();
    let mut claim = |name: String, what: String| match owners.iter().find(|(n, _)| *n == name) {
        Some((_, first)) => Err(format!("Two components are named `{name}`: {first}, and {what}. Rename one")),
        None => {
            owners.push((name, what));
            Ok(())
        }
    };
    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else { continue };
        let Ok(graph) = crate::load_from_str(source.trim_start_matches('\u{feff}')) else { continue };
        let name = crate::script_name(dir, file);
        let path = format!("scripts/{name}");
        if !graph.declares_only() {
            claim(crate::rust::names(&path).ty, format!("the component of {path}"))?;
        }
        for def in graph.defined_components() {
            claim(def.name.clone(), format!("the one declared in {path}"))?;
            out.push(ComponentInfo { source: Some(name.clone()), ..declared_info(def) });
        }
    }
    Ok(out)
}

/// The component and field info for `Component.path` as `graph` sees it: the components it
/// declares itself first, since those may not be saved yet, then the catalog.
pub fn field_in(graph: &Graph, field: &str) -> Option<(ComponentInfo, FieldInfo)> {
    let (name, path) = split(field)?;
    let component = match graph.defined_components().find(|d| d.name == name) {
        Some(def) => declared_info(def),
        None => find(name)?,
    };
    let info = component.fields.iter().find(|f| f.path == path)?.clone();
    Some((component, info))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::{Value, Variable};

    /// A fresh `assets/scripts`-like folder with the given scripts.
    fn scripts(name: &str, files: &[(&str, Graph)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vs_graph_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (file, graph) in files {
            std::fs::write(dir.join(file), crate::save_to_string(graph)).unwrap();
        }
        dir
    }

    fn declaring(name: &str) -> Graph {
        let mut g = Graph::default();
        let fields = vec![Variable { name: "value".into(), value: Value::Number(1.0) }];
        g.add_component_def(ComponentDef { name: name.into(), fields }, 0.0, 0.0);
        g
    }

    #[test]
    fn scripts_declare_components_for_the_whole_project() {
        // `health.lua` only declares, so declaring `Health` doesn't clash with its own name.
        let dir = scripts("declare", &[("health.lua", declaring("Health")), ("spinner.lua", crate::starter())]);
        let found = scan_declared(&dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rust_path, "Health");
        assert_eq!(found[0].source.as_deref(), Some("health.lua"));
        let value = FieldInfo { path: "value".into(), ty: PinType::Number, default: Some(Value::Number(1.0)) };
        assert_eq!(found[0].fields, vec![value]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn two_components_with_one_name_are_an_error() {
        // `spinner.lua` has logic, so it has its own `Spinner` component.
        let dir = scripts("clash", &[("a.lua", declaring("Spinner")), ("spinner.lua", crate::starter())]);
        let err = scan_declared(&dir).unwrap_err();
        assert!(err.contains("Two components are named `Spinner`"), "{err}");
        assert!(err.contains("scripts/a.lua") && err.contains("scripts/spinner.lua"), "{err}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_script_sees_the_components_it_declares() {
        let g = declaring("Unsaved");
        let (component, field) = field_in(&g, "Unsaved.value").unwrap();
        assert_eq!((component.name.as_str(), field.ty), ("Unsaved", PinType::Number));
        assert!(field_in(&Graph::default(), "Unsaved.value").is_none());
        assert!(field_in(&Graph::default(), "Transform.translation").is_some());
    }
}

/// [`load_project`] by its old name: projects created before it was renamed call this from their
/// `build.rs`, which is theirs and isn't regenerated.
pub fn add_from_project(root: &std::path::Path) -> Result<usize, String> {
    load_project(root)
}
