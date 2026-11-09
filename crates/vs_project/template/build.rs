//! Compiles the scripts in `assets/scripts`, and in its folders, into `$OUT_DIR/scripts.rs`. With the `preview`
//! feature only their components are generated, and the `.lua` files drive them; otherwise the
//! scripts become Rust systems. A script with errors fails the build, with the editor's messages.

use std::path::Path;

use vs_graph::rust::{Mode, Script, emit_dev};

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
    let mode = if std::env::var_os("CARGO_FEATURE_PREVIEW").is_some() { Mode::Preview } else { Mode::Native };
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("scripts.rs");
    std::fs::write(out, emit_dev(&scripts, mode)).unwrap();
}
