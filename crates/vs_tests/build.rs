//! Compiles the test scripts in `assets/scripts` both ways a project's `build.rs` does, into
//! `$OUT_DIR/scripts_native.rs` (the scripts as Rust systems) and `$OUT_DIR/scripts_preview.rs`
//! (only their components; Lua drives them). The differential test runs both side by side. It
//! follows the project template's `build.rs`, so it exercises the same path real projects take.

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

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("assets/scripts exists")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "lua"))
        .collect();
    files.sort();

    let mut lowered = Vec::new();
    for file in &files {
        println!("cargo::rerun-if-changed={}", file.display());
        let path = format!("scripts/{}", file.file_name().unwrap().to_string_lossy());
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
}
