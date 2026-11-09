//! Prints the catalog read from reflection, then writes `src/generated.rs` (Rust) and shows the
//! Lua for the same programs.
//!
//! Run it with `cargo run --example codegen`.

use reflect_nodes::{catalog, demo, lua_backend, nodes, rust_backend};

fn main() {
    let registry = nodes::registry();
    let (specs, skipped) = catalog::build(&registry);

    println!("== Catalog, from reflection ==");
    for spec in &specs {
        let inputs: Vec<String> = spec.inputs.iter().map(|(n, t)| format!("{n}: {t:?}")).collect();
        println!("{:<15} ({}) -> {:?}   // {}", spec.name, inputs.join(", "), spec.output, spec.description);
    }
    for s in &skipped {
        println!("skipped: {s}");
    }

    let programs = demo::programs(&specs);
    let rust = rust_backend::emit_module(&programs);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/generated.rs");
    std::fs::write(path, &rust).unwrap();

    println!("\n== Rust (written to src/generated.rs) ==\n{rust}");
    println!("== Lua ==");
    for p in &programs {
        println!("-- {}\n{}", p.name, lua_backend::emit(p));
    }
}
