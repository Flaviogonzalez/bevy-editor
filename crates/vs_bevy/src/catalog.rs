//! The catalog a game describes for the editor: which of its registered components scripts can
//! Get and Set, chosen by the project's `.bevy-editor/catalog.toml`.
//!
//! The game writes it, not the editor, because only the running game has the full type registry:
//! its own components and those of every plugin it uses.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use serde::Deserialize;

use crate::components::{Described, MAX_LEVELS, describe as describe_one};

/// The config, relative to the game's root. Same as `vs_graph::components::CONFIG_FILE`.
pub const CONFIG_FILE: &str = ".bevy-editor/catalog.toml";

/// The catalog the game writes, relative to its root. Same as `vs_graph::components::PROJECT_FILE`.
pub const CATALOG_FILE: &str = ".bevy-editor/components.json";

/// Set when the game runs only to write its catalog. Same as `vs_graph::components::CATALOG_ENV`.
pub const ONLY_ENV: &str = "BEVY_EDITOR_CATALOG_ONLY";

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub components: ComponentsConfig,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ComponentsConfig {
    /// Patterns a component's short name must match one of (see [`matches`]).
    pub include: Vec<String>,
    /// Patterns that leave a component out even if included.
    pub exclude: Vec<String>,
    /// Levels of nested fields to list: 1 is the component's own fields.
    pub depth: usize,
    /// How exported code names a component, by short name, when the guess is wrong.
    pub paths: BTreeMap<String, String>,
}

impl Default for ComponentsConfig {
    /// Without a config, the game lists its own components.
    fn default() -> Self {
        ComponentsConfig { include: vec!["crate::*".into()], exclude: Vec::new(), depth: 3, paths: BTreeMap::new() }
    }
}

impl Config {
    /// The config in `root`, or the default when there's none.
    pub fn load(root: &Path) -> Result<Config, String> {
        let path = root.join(CONFIG_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => Config::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(_) => Ok(Config::default()),
        }
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        if !(1..=MAX_LEVELS).contains(&config.components.depth) {
            return Err(format!("depth must be between 1 and {MAX_LEVELS}"));
        }
        Ok(config)
    }
}

/// `*` matches any run of characters, everything else itself.
fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = text.strip_prefix(first) else { return false };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else { return rest.is_empty() };
    for part in middle {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

/// Whether `pattern` picks a component named `short`. `crate::…` patterns only pick the game's
/// own components (`own`); others pick from any crate.
pub fn matches(pattern: &str, own: bool, short: &str) -> bool {
    match pattern.strip_prefix("crate::") {
        Some(rest) => own && glob(rest, short),
        None => glob(pattern, short),
    }
}

/// How exported code names a component that isn't the game's own: `bevy_light::…::PointLight`
/// becomes `bevy::light::PointLight`, `avian3d::…::RigidBody` becomes `avian3d::RigidBody`. Crates
/// usually export their components from the root; `paths` in the config covers the rest.
fn guess_path(type_path: &str, short: &str) -> String {
    let krate = type_path.split("::").next().unwrap_or(type_path);
    match krate.strip_prefix("bevy_") {
        Some(module) => format!("bevy::{module}::{short}"),
        None => format!("{krate}::{short}"),
    }
}

/// The components `config` picks from the registry, with their fields. A component must be
/// reflected as a component with a `Default` (scripts require the components they use, so Bevy
/// must be able to make one), not generic, and its short name must be unambiguous, since scripts
/// name it by that.
pub fn describe(registry: &TypeRegistry, crate_name: &str, config: &ComponentsConfig) -> Vec<Described> {
    let own_prefix = format!("{crate_name}::");
    let mut out: Vec<Described> = registry
        .iter()
        .filter(|reg| reg.data::<ReflectComponent>().is_some() && reg.data::<ReflectDefault>().is_some())
        .filter_map(|reg| {
            let table = reg.type_info().type_path_table();
            let (path, short) = (table.path(), table.short_path());
            if path.contains('<') || registry.is_ambiguous(short) {
                return None;
            }
            let own = path.starts_with(&own_prefix);
            let picked = config.include.iter().any(|p| matches(p, own, short))
                && !config.exclude.iter().any(|p| matches(p, own, short));
            if !picked {
                return None;
            }
            let rust_path = match config.paths.get(short) {
                Some(p) => p.clone(),
                None if own => format!("crate::{}", &path[own_prefix.len()..]),
                None => guess_path(path, short),
            };
            describe_one(registry, reg, short, rust_path, config.depth.clamp(1, MAX_LEVELS))
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Writes the catalog to `root`, unless it's unchanged (the build script watches the file).
/// Returns how many components it lists.
pub fn write(registry: &TypeRegistry, crate_name: &str, root: &Path) -> Result<usize, String> {
    let config = Config::load(root)?;
    let described = describe(registry, crate_name, &config.components);
    let file = root.join(CATALOG_FILE);
    let new = serde_json::to_value(&described).map_err(|e| e.to_string())?;
    let same = std::fs::read_to_string(&file)
        .ok()
        .and_then(|old| serde_json::from_str::<serde_json::Value>(&old).ok())
        .is_some_and(|old| old == new);
    if !same {
        std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&described).map_err(|e| e.to_string())?;
        std::fs::write(&file, json + "\n").map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(described.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob("*", "Anything"));
        assert!(glob("PointLight", "PointLight"));
        assert!(!glob("PointLight", "PointLights"));
        assert!(glob("*Light", "SpotLight"));
        assert!(!glob("*Light", "LightProbe"));
        assert!(glob("Light*", "LightProbe"));
        assert!(glob("*ext*", "TextColor"));
        assert!(glob("T*t*r", "TextColor"));
        assert!(!glob("T*z*", "TextColor"));
        assert!(!glob("ab*ba", "aba"));
    }

    #[test]
    fn crate_patterns_pick_only_the_games_own() {
        assert!(matches("crate::*", true, "Wobble"));
        assert!(!matches("crate::*", false, "PointLight"));
        assert!(matches("*", false, "PointLight"));
        assert!(matches("crate::Wob*", true, "Wobble"));
    }

    #[test]
    fn paths_are_guessed_from_the_crate() {
        assert_eq!(guess_path("bevy_light::point_light::PointLight", "PointLight"), "bevy::light::PointLight");
        assert_eq!(guess_path("avian3d::dynamics::rigid_body::RigidBody", "RigidBody"), "avian3d::RigidBody");
    }

    #[test]
    fn configs_parse_and_default() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
        let c = Config::parse("[components]\ninclude = [\"*Light\"]\ndepth = 1\n[components.paths]\nX = \"a::X\"\n").unwrap();
        assert_eq!(c.components.include, ["*Light"]);
        assert_eq!(c.components.exclude, Vec::<String>::new());
        assert_eq!(c.components.paths["X"], "a::X");
        assert!(Config::parse("[components]\ndepth = 0\n").is_err());
        assert!(Config::parse("[components]\ninclud = []\n").is_err(), "typos are reported");
    }

    /// The editor writes `vs_graph`'s default config and constants; this reads them.
    #[test]
    fn agrees_with_the_editor() {
        assert_eq!(Config::parse(vs_graph::components::DEFAULT_CONFIG).unwrap(), Config::default());
        assert_eq!(CONFIG_FILE, vs_graph::components::CONFIG_FILE);
        assert_eq!(CATALOG_FILE, vs_graph::components::PROJECT_FILE);
        assert_eq!(ONLY_ENV, vs_graph::components::CATALOG_ENV);
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Mine {
        speed: f32,
        nested: Inner,
    }

    #[derive(Reflect, Default)]
    struct Inner {
        deep: Deeper,
        flag: bool,
    }

    #[derive(Reflect, Default)]
    struct Deeper {
        value: f32,
    }

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct NoDefault(f32);

    #[test]
    fn picks_by_config() {
        let mut registry = TypeRegistry::default();
        registry.register::<Mine>();
        registry.register::<NoDefault>();
        registry.register::<Transform>();
        let crate_name = module_path!().split("::").next().unwrap();
        let names = |c: &ComponentsConfig| describe(&registry, crate_name, c).into_iter().map(|d| d.name).collect::<Vec<_>>();

        let default = ComponentsConfig::default();
        assert_eq!(names(&default), ["Mine"], "the game's own, and only with a Default");
        let all = ComponentsConfig { include: vec!["*".into()], ..default.clone() };
        assert_eq!(names(&all), ["Mine", "Transform"]);
        let some = ComponentsConfig { exclude: vec!["Mine".into()], ..all.clone() };
        assert_eq!(names(&some), ["Transform"]);

        let transform = describe(&registry, crate_name, &all).into_iter().find(|d| d.name == "Transform").unwrap();
        assert_eq!(transform.rust_path, "bevy::transform::Transform");
        let mut renamed = all.clone();
        renamed.paths.insert("Transform".into(), "Transform".into());
        assert_eq!(describe(&registry, crate_name, &renamed).iter().find(|d| d.name == "Transform").unwrap().rust_path, "Transform");

        let fields = |depth| {
            let c = ComponentsConfig { depth, ..default.clone() };
            describe(&registry, crate_name, &c)[0].fields.iter().map(|f| f.path.clone()).collect::<Vec<_>>()
        };
        assert_eq!(fields(1), ["speed"]);
        assert_eq!(fields(2), ["speed", "nested.flag"]);
        assert_eq!(fields(3), ["speed", "nested.deep.value", "nested.flag"]);
        assert_eq!(describe(&registry, crate_name, &default)[0].rust_path, "crate::catalog::tests::Mine");
    }
}
