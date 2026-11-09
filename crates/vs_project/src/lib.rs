//! Bevy projects made with the editor: where they live, how they're created, and how they're
//! exported as plain Bevy projects.
//!
//! A project is an ordinary Cargo package with a `bevy-editor.json` marker. Its scripts live in
//! `assets/scripts`; its `build.rs` compiles them (see the template). The game's own code only
//! spawns the components generated from the scripts, so [`export`] can swap the build-time
//! generation for plain source files and drop everything else that belongs to the editor.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const MARKER: &str = "bevy-editor.json";

/// The editor's settings, kept in the user's config directory.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Where new projects are created, one folder each.
    pub projects_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let base = dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("."));
        Self { projects_dir: base.join("Bevy Projects") }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("bevy-editor").join("config.json")
    }

    /// The saved settings, or the defaults when there are none yet.
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, serde_json::to_string_pretty(self)?).with_context(|| format!("writing {}", path.display()))
    }

    /// The projects in [`Self::projects_dir`], by name.
    pub fn projects(&self) -> Vec<Project> {
        let mut list: Vec<Project> = std::fs::read_dir(&self.projects_dir)
            .map(|dir| dir.filter_map(|e| Project::open(&e.ok()?.path()).ok()).collect())
            .unwrap_or_default();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }
}

#[derive(Serialize, Deserialize)]
struct Marker {
    name: String,
    version: u32,
}

/// A project made with the editor.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
}

/// The bevy-editor checkout this build came from. Projects depend on its crates by path.
fn editor_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    clean(&std::fs::canonicalize(&root).unwrap_or(root))
}

/// Drops Windows' `\\?\` prefix, which Cargo and TOML paths don't want.
fn clean(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s).to_string())
}

fn toml_path(path: &Path) -> String {
    clean(path).to_string_lossy().replace('\\', "/")
}

/// A Cargo package name for what the user typed: lowercase letters, digits and `_`, starting
/// with a letter. `My Game` → `my_game`.
pub fn package_name(text: &str) -> Option<String> {
    let mut name = String::new();
    for c in text.trim().chars() {
        if c.is_ascii_alphanumeric() {
            name.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '-' || c == '_') && !name.is_empty() && !name.ends_with('_') {
            name.push('_');
        }
    }
    let name = name.trim_end_matches('_').to_string();
    name.starts_with(|c: char| c.is_ascii_alphabetic()).then_some(name)
}

/// Creates a project named `name` in `projects_dir`: the game, a starter script, and the build
/// setup. Fails if the folder already exists.
pub fn create(projects_dir: &Path, name: &str) -> Result<Project> {
    let Some(name) = package_name(name) else {
        bail!("`{name}` can't be a project name: use letters, digits and spaces, starting with a letter");
    };
    let root = projects_dir.join(&name);
    if root.exists() {
        bail!("{} already exists", root.display());
    }
    let editor = editor_root();
    let target = projects_dir.join(".target");
    let fill = |text: &str| {
        text.replace("{{name}}", &name).replace("{{editor}}", &toml_path(&editor)).replace("{{target}}", &toml_path(&target))
    };
    let files: [(&str, String); 8] = [
        ("Cargo.toml", fill(include_str!("../template/Cargo.toml.in"))),
        (".cargo/config.toml", fill(include_str!("../template/cargo_config.toml.in"))),
        (".gitignore", include_str!("../template/gitignore.in").to_string()),
        ("build.rs", include_str!("../template/build.rs").to_string()),
        ("src/main.rs", include_str!("../template/main.rs").to_string()),
        ("src/scripts.rs", include_str!("../template/scripts.rs").to_string()),
        ("assets/scripts/spinner.lua", vs_graph::save_to_string(&vs_graph::starter())),
        (vs_graph::components::CONFIG_FILE, vs_graph::components::DEFAULT_CONFIG.to_string()),
    ];
    for (path, contents) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
    }
    std::fs::write(root.join(MARKER), serde_json::to_string_pretty(&Marker { name: name.clone(), version: 1 })?)?;
    // The editor's lock file pins the Bevy release and its dependencies that the editor's own
    // crates are built and tested with; Cargo drops the entries a project doesn't use.
    let lock = editor.join("Cargo.lock");
    if lock.exists() {
        std::fs::copy(lock, root.join("Cargo.lock"))?;
    }
    Ok(Project { root, name })
}

impl Project {
    pub fn open(root: &Path) -> Result<Self> {
        let marker = std::fs::read_to_string(root.join(MARKER))
            .with_context(|| format!("{} isn't a bevy-editor project", root.display()))?;
        let marker: Marker = serde_json::from_str(&marker)?;
        Ok(Self { root: root.to_path_buf(), name: marker.name })
    }

    /// The project that `file` (a script, say) belongs to, if any.
    pub fn containing(file: &Path) -> Option<Self> {
        file.ancestors().skip(1).find_map(|dir| Self::open(dir).ok())
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.root.join("assets").join("scripts")
    }

    /// The project's scripts, in folders too, sorted by their path in [`Self::scripts_dir`].
    pub fn scripts(&self) -> Vec<PathBuf> {
        vs_graph::script_files(&self.scripts_dir())
    }

    /// A script's path in [`Self::scripts_dir`]: `player/jump.lua`.
    pub fn script_name(&self, file: &Path) -> String {
        vs_graph::script_name(&self.scripts_dir(), file)
    }

    /// Adds an empty script. Its component is named after it (`door` → `Door`).
    pub fn new_script(&self, name: &str) -> Result<PathBuf> {
        let Some(stem) = package_name(name) else {
            bail!("`{name}` can't be a script name: use letters, digits and spaces, starting with a letter");
        };
        let path = self.scripts_dir().join(format!("{stem}.lua"));
        if path.exists() {
            bail!("{} already exists", path.display());
        }
        std::fs::create_dir_all(self.scripts_dir())?;
        let mut graph = vs_graph::Graph::default();
        graph.add_node("event.update", None, 0.0, 0.0);
        std::fs::write(&path, vs_graph::save_to_string(&graph))?;
        Ok(path)
    }

    /// Where the exported source goes.
    /// Which components the game describes for the editor. New projects have one; older ones may
    /// not, and then the game lists only its own.
    pub fn catalog_config(&self) -> PathBuf {
        self.root.join(vs_graph::components::CONFIG_FILE)
    }

    pub fn export_dir(&self) -> PathBuf {
        self.root.join("export")
    }

    /// Where a release build is packaged: the executable and its assets.
    pub fn dist_dir(&self) -> PathBuf {
        self.root.join("dist")
    }

    /// The Cargo target directory, from the project's `.cargo/config.toml`.
    pub fn target_dir(&self) -> PathBuf {
        let config = std::fs::read_to_string(self.root.join(".cargo/config.toml")).unwrap_or_default();
        config
            .parse::<toml_edit::DocumentMut>()
            .ok()
            .and_then(|doc| doc.get("build")?.get("target-dir")?.as_str().map(PathBuf::from))
            .unwrap_or_else(|| self.root.join("target"))
    }

    /// The scripts as IR, or every error in them.
    fn lowered(&self) -> Result<Vec<(String, vs_graph::ir::Module)>> {
        vs_graph::components::load_project(&self.root).map_err(anyhow::Error::msg)?;
        let mut out = Vec::new();
        let mut errors = Vec::new();
        for file in self.scripts() {
            let path = format!("scripts/{}", self.script_name(&file));
            let source = std::fs::read_to_string(&file)?;
            let graph = vs_graph::load_from_str(source.trim_start_matches('\u{feff}'))
                .with_context(|| format!("reading {path}"))?;
            let lowered = vs_graph::lower(&graph);
            for p in lowered.problems.iter().filter(|p| p.severity == vs_graph::Severity::Error) {
                errors.push(format!("{path}: {}", p.message));
            }
            out.push((path, lowered.module));
        }
        if !errors.is_empty() {
            bail!("the scripts have errors:\n{}", errors.join("\n"));
        }
        Ok(out)
    }
}

/// Paths a plain Bevy project doesn't have.
fn excluded(rel: &Path) -> bool {
    let s = rel.to_string_lossy().replace('\\', "/");
    matches!(s.as_str(), "target" | "export" | "dist" | ".cargo" | ".git" | "build.rs" | "src/scripts.rs" | ".bevy-editor" | MARKER)
        || s == "assets/scripts"
}

fn copy_tree(from: &Path, to: &Path, rel: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from.join(rel))? {
        let entry = entry?;
        let rel = rel.join(entry.file_name());
        if excluded(&rel) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(to.join(&rel))?;
            copy_tree(from, to, &rel)?;
        } else {
            std::fs::copy(entry.path(), to.join(&rel))?;
        }
    }
    Ok(())
}

/// The project's `Cargo.toml` without the editor: no build script, no `vs_*` crates, no preview
/// feature, and none of the Bevy features only the editor's setup needs.
fn plain_manifest(manifest: &str) -> Result<String> {
    let mut doc: toml_edit::DocumentMut = manifest.parse().context("parsing Cargo.toml")?;
    if let Some(package) = doc.get_mut("package").and_then(|p| p.as_table_like_mut()) {
        package.remove("build");
    }
    for table in ["dependencies", "build-dependencies", "dev-dependencies"] {
        if let Some(deps) = doc.get_mut(table).and_then(|t| t.as_table_like_mut()) {
            let ours: Vec<String> = deps.iter().map(|(k, _)| k.to_string()).filter(|k| k.starts_with("vs_")).collect();
            for key in ours {
                deps.remove(&key);
            }
            if deps.is_empty() {
                doc.remove(table);
            }
        }
    }
    if let Some(features) = doc.get_mut("features").and_then(|t| t.as_table_like_mut()) {
        features.remove("preview");
        if features.is_empty() {
            doc.remove("features");
        }
    }
    if let Some(bevy) = doc.get_mut("dependencies").and_then(|d| d.get_mut("bevy"))
        && let Some(features) = bevy.get_mut("features").and_then(|f| f.as_array_mut())
    {
        features.retain(|f| !matches!(f.as_str(), Some("file_watcher" | "reflect_functions")));
        if features.is_empty() {
            if let Some(t) = bevy.as_table_like_mut() {
                t.remove("features");
            }
        }
    }
    // `bevy = { version = "0.19.1" }` reads better as `bevy = "0.19.1"`.
    if let Some(bevy) = doc.get_mut("dependencies").and_then(|d| d.get_mut("bevy"))
        && let Some(t) = bevy.as_table_like()
        && t.len() == 1
        && let Some(version) = t.get("version").and_then(|v| v.as_str()).map(str::to_string)
    {
        *bevy = toml_edit::value(version);
    }
    // The template's comment above `bevy` explains the editor's features, which are gone now.
    if let Some(deps) = doc.get_mut("dependencies").and_then(|d| d.as_table_mut())
        && let Some(mut key) = deps.key_mut("bevy")
    {
        key.leaf_decor_mut().clear();
    }
    Ok(doc.to_string())
}

/// Writes `out` as a plain Bevy project: the game's code, its assets without the scripts, and
/// the scripts as Rust source in `src/scripts/`. Nothing in it refers to the editor. `out` is
/// replaced if it exists.
pub fn export(project: &Project, out: &Path) -> Result<()> {
    let lowered = project.lowered()?;
    if out.exists() {
        std::fs::remove_dir_all(out).with_context(|| format!("clearing {}", out.display()))?;
    }
    std::fs::create_dir_all(out)?;
    copy_tree(&project.root, out, Path::new(""))?;

    let scripts: Vec<vs_graph::rust::Script> =
        lowered.iter().map(|(path, ir)| vs_graph::rust::Script { path: path.clone(), ir }).collect();
    for (rel, source) in vs_graph::rust::emit_export(&scripts) {
        let path = out.join("src").join(rel);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, source)?;
    }
    let manifest = std::fs::read_to_string(project.root.join("Cargo.toml"))?;
    std::fs::write(out.join("Cargo.toml"), plain_manifest(&manifest)?)?;

    // Let Cargo drop the editor's crates from the lock file, without upgrading anything.
    if out.join("Cargo.lock").exists() {
        let _ = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--offline", "--manifest-path"])
            .arg(out.join("Cargo.toml"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    Ok(())
}

/// A `cargo` command in `dir` that runs at low priority, without a console window, so a build
/// doesn't take over the machine.
pub fn cargo(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(dir).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW);
    }
    command
}

/// After `cargo build --release` in the export directory: copies the executable and the assets
/// (without scripts) into [`Project::dist_dir`], ready to ship. Returns the executable's path.
pub fn package(project: &Project) -> Result<PathBuf> {
    let exe = format!("{}{}", project.name, std::env::consts::EXE_SUFFIX);
    let built = project.target_dir().join("release").join(&exe);
    if !built.exists() {
        bail!("{} wasn't built", built.display());
    }
    let dist = project.dist_dir();
    if dist.exists() {
        std::fs::remove_dir_all(&dist)?;
    }
    std::fs::create_dir_all(&dist)?;
    std::fs::copy(&built, dist.join(&exe))?;
    let assets = project.export_dir().join("assets");
    if assets.exists() {
        std::fs::create_dir_all(dist.join("assets"))?;
        copy_tree(&assets, &dist.join("assets"), Path::new(""))?;
    }
    Ok(dist.join(exe))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names() {
        assert_eq!(package_name("My Game").as_deref(), Some("my_game"));
        assert_eq!(package_name("  space-shooter 2 ").as_deref(), Some("space_shooter_2"));
        assert_eq!(package_name("3d"), None);
        assert_eq!(package_name("!!"), None);
    }

    #[test]
    fn manifest_loses_the_editor() {
        let manifest = include_str!("../template/Cargo.toml.in").replace("{{name}}", "game").replace("{{editor}}", "/x");
        let plain = plain_manifest(&manifest).unwrap();
        assert!(!plain.contains("vs_"), "{plain}");
        assert!(!plain.contains("preview"), "{plain}");
        assert!(!plain.contains("file_watcher") && !plain.contains("reflect_functions"), "{plain}");
        assert!(!plain.contains("script compiler"), "the comment about the editor's features goes too:
{plain}");
        assert!(plain.contains("bevy = \"0.19.1\""), "{plain}");
    }

    #[test]
    fn create_and_export() {
        let dir = std::env::temp_dir().join(format!("vs_project_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = create(&dir, "Test Game").unwrap();
        assert_eq!(project.name, "test_game");
        assert_eq!(Project::open(&project.root).unwrap(), project);
        assert_eq!(project.scripts().len(), 1);
        project.new_script("Door").unwrap();
        assert!(project.new_script("door").is_err());
        // A script in a folder: the folder is a plugin, in its own module.
        let jump = project.scripts_dir().join("player").join("jump.lua");
        std::fs::create_dir_all(jump.parent().unwrap()).unwrap();
        std::fs::write(&jump, vs_graph::save_to_string(&vs_graph::starter())).unwrap();
        assert_eq!(project.script_name(&jump), "player/jump.lua");
        assert_eq!(project.scripts().len(), 3);
        assert_eq!(dir.join(".target"), project.target_dir());

        let out = project.export_dir();
        export(&project, &out).unwrap();
        let mut files = Vec::new();
        fn walk(dir: &Path, root: &Path, files: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    walk(&p, root, files);
                } else {
                    files.push(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        walk(&out, &out, &mut files);
        files.sort();
        assert_eq!(
            files,
            [
                ".gitignore",
                "Cargo.lock",
                "Cargo.toml",
                "src/main.rs",
                "src/scripts/door.rs",
                "src/scripts/mod.rs",
                "src/scripts/player/jump.rs",
                "src/scripts/player/mod.rs",
                "src/scripts/spinner.rs",
            ]
        );
        let root = std::fs::read_to_string(out.join("src/scripts/mod.rs")).unwrap();
        assert!(root.contains("mod player;") && root.contains("pub use player::jump::Jump;"), "{root}");
        assert!(root.contains("player::PlayerPlugin"), "{root}");
        let folder = std::fs::read_to_string(out.join("src/scripts/player/mod.rs")).unwrap();
        assert!(folder.contains("pub mod jump;") && folder.contains("app.add_plugins(jump::JumpPlugin)"), "{folder}");
        for file in &files {
            if file == "Cargo.lock" {
                continue;
            }
            let text = std::fs::read_to_string(out.join(file)).unwrap();
            for trace in ["vs_", "visual", "lua", "generated", "bevy-editor"] {
                assert!(!text.to_lowercase().contains(trace), "{file} mentions `{trace}`:\n{text}");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
