//! How much slower is the preview? Times the `bob` program three ways.
//!
//! Run it with `cargo run --release --example bench`.

use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

use reflect_nodes::ir::Value;
use reflect_nodes::{catalog, demo, generated, lua_backend, nodes};

const N: usize = 200_000;

fn main() {
    let registry = Rc::new(nodes::registry());
    let (specs, _) = catalog::build(&registry);
    let lua = lua_backend::runtime(registry.clone(), &specs).unwrap();
    let bob = demo::programs(&specs).into_iter().find(|p| p.name == "bob").unwrap();

    // 1. Generated Rust.
    let start = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..N {
        acc += generated::bob(black_box(i as f32 * 0.01));
    }
    black_box(acc);
    let rust = start.elapsed();

    // 2. Lua preview: glue in Lua, every node a reflected Rust call. The loop runs inside Lua so
    //    only the per-node cost is measured, not the Rust -> Lua call per iteration.
    let function = lua_backend::load(&lua, &bob).unwrap();
    lua.globals().set("bob", function).unwrap();
    let start = Instant::now();
    let _: f64 = lua
        .load(format!("local acc = 0 for i = 0, {} do acc = acc + bob(i * 0.01) end return acc", N - 1))
        .eval()
        .unwrap();
    let preview = start.elapsed();

    // 2b. Lua preview with the typed bindings `nodes!` generates: same Rust functions, no reflection.
    let typed = lua_backend::runtime_typed().unwrap();
    let function = lua_backend::load(&typed, &bob).unwrap();
    typed.globals().set("bob", function).unwrap();
    let start = Instant::now();
    let _: f64 = typed
        .load(format!("local acc = 0 for i = 0, {} do acc = acc + bob(i * 0.01) end return acc", N - 1))
        .eval()
        .unwrap();
    let preview_typed = start.elapsed();

    // 3. What vs_bevy does today: Lua's own math, no reflection.
    let start = Instant::now();
    let _: f64 = lua
        .load(format!(
            "local acc = 0 for i = 0, {} do acc = acc + (math.sin(i * 0.01 * 2) * 0.25 + 1) end return acc",
            N - 1
        ))
        .eval()
        .unwrap();
    let native_lua = start.elapsed();

    let per = |d: std::time::Duration| d.as_nanos() as f64 / N as f64;
    println!("bob ({} node calls) x {N}", bob.calls.len());
    println!("generated Rust        {:>9.1} ns/run", per(rust));
    println!("Lua + reflected nodes {:>9.1} ns/run  ({:.0}x Rust)", per(preview), per(preview) / per(rust));
    println!("Lua + typed bindings  {:>9.1} ns/run  ({:.0}x Rust)", per(preview_typed), per(preview_typed) / per(rust));
    println!("Lua native math      {:>9.1} ns/run  ({:.0}x Rust)", per(native_lua), per(native_lua) / per(rust));
    println!(
        "frame budget check: at 60 fps, the preview fits {:.0} runs of `bob` per millisecond",
        1_000_000.0 / per(preview)
    );
    let _ = Value::Number(0.0);
}
