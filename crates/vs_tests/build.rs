//! Compiles the test scripts in `assets/scripts` both ways a project's `build.rs` does, into
//! `$OUT_DIR/scripts_native.rs` (the scripts as Rust systems) and `$OUT_DIR/scripts_preview.rs`
//! (only their components; Lua drives them). The differential test runs both side by side. It
//! also writes them as an exported project would have them, in `$OUT_DIR/export/scripts/`. It
//! follows the project template's `build.rs`, so it exercises the same path real projects take.

use std::path::Path;

use vs_graph::rust::{Mode, Script, emit_dev, emit_export};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scripts");
    println!("cargo::rerun-if-changed={}", dir.display());

    // The game's own components, as its last preview run described them.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!("cargo::rerun-if-changed={}", root.join(vs_graph::components::PROJECT_FILE).display());
    vs_graph::components::load_project(root).unwrap_or_else(|e| panic!("{e}"));
    // Run only to write that list (the editor's Refresh catalog): scripts may use components it
    // doesn't have yet, which is why it's being written. Their errors are warnings then.
    let catalog_only = std::env::var_os(vs_graph::components::CATALOG_ENV).is_some();
    println!("cargo::rerun-if-env-changed={}", vs_graph::components::CATALOG_ENV);

    // Subfolders too: each folder is a plugin of the scripts in it.
    let files = vs_graph::script_files(&dir);

    let mut lowered = Vec::new();
    for file in &files {
        println!("cargo::rerun-if-changed={}", file.display());
        let path = format!("scripts/{}", vs_graph::script_name(&dir, file));
        let source = std::fs::read_to_string(file).unwrap();
        let graph = vs_graph::load_from_str(source.trim_start_matches('\u{feff}'))
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        let out = vs_graph::lower(&graph);
        let errors: Vec<_> =
            out.problems.iter().filter(|p| p.severity == vs_graph::Severity::Error).map(|p| &p.message).collect();
        if catalog_only {
            for e in &errors {
                println!("cargo::warning={path}: {e}");
            }
        } else {
            assert!(errors.is_empty(), "{path} has errors: {errors:?}");
        }
        lowered.push((path, out.module));
    }
    let scripts: Vec<Script> = lowered.iter().map(|(path, ir)| Script { path: path.clone(), ir }).collect();

    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    let native = emit_dev(&scripts, Mode::Native);
    let preview = emit_dev(&scripts, Mode::Preview);
    std::fs::write(out.join("scripts_native.rs"), &native).unwrap();
    std::fs::write(out.join("scripts_preview.rs"), &preview).unwrap();
    // The exported project's `src/scripts/`, so a test compiles its modules too. `include!` can't
    // follow `mod x;` to other files, so each is replaced by the file's contents: an inline
    // module resolves paths exactly as a file module does.
    let files = emit_export(&scripts);
    std::fs::write(out.join("scripts_export.rs"), inline("scripts/mod.rs", &files)).unwrap();
}

/// The file at `path` among `files`, with each `mod x;` replaced by `mod x { … }` holding the
/// file of that module.
fn inline(path: &str, files: &[(String, String)]) -> String {
    let dir = path.strip_suffix("mod.rs").unwrap_or(path);
    let source = &files.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("no {path}")).1;
    let mut out = String::new();
    for line in source.lines() {
        let decl = line.trim().strip_suffix(';').and_then(|l| l.strip_prefix("pub mod ").or_else(|| l.strip_prefix("mod ")));
        match decl {
            Some(name) if !name.contains(' ') => {
                let file = format!("{dir}{name}.rs");
                let child = if files.iter().any(|(p, _)| *p == file) { file } else { format!("{dir}{name}/mod.rs") };
                let vis = if line.trim().starts_with("pub ") { "pub " } else { "" };
                out.push_str(&format!("{vis}mod {name} {{\n{}\n}}\n", inline(&child, files)));
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}
