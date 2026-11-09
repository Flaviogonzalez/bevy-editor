# One definition per node, two backends

Can each node be written once, as a Rust function, and drive both the Lua preview and the generated Rust without the two drifting apart?

```sh
cargo run --example codegen            # catalog from reflection, writes src/generated.rs, prints the Lua
cargo test                             # differential tests
cargo run --release --example bench    # preview overhead
```

## How it fits together

1. **Nodes are Rust functions** ([`src/nodes.rs`](src/nodes.rs)). The `nodes!` macro keeps each function as-is, registers it in a `bevy_reflect` `FunctionRegistry` with its parameter names, and generates typed `mlua` bindings for it.
2. **The catalog comes from reflection** ([`src/catalog.rs`](src/catalog.rs)): node names, pin names and types, and descriptions from the doc comments. Nothing is listed by hand.
3. **A graph lowers to an IR** ([`src/ir.rs`](src/ir.rs)): a flat list of calls, each with its own variable, in a fixed order. Neither backend decides evaluation order.
4. **Lua backend** ([`src/lua_backend.rs`](src/lua_backend.rs)): glue that only calls `node.*`. It never does arithmetic itself, so every operation runs the same compiled Rust as the Rust backend.
5. **Rust backend** ([`src/rust_backend.rs`](src/rust_backend.rs)): `quote` builds the code as tokens and `prettyplease` formats it. See [`src/generated.rs`](src/generated.rs).

## Results (2026-11-08)

**Equivalence holds.** The differential tests compare bit for bit, with NaN, infinities, both zeros and subnormals in the inputs:

- the five demo programs, Lua preview against the compiled generated Rust, 2000 random inputs each;
- 2000 random programs (up to depth 5, numbers, booleans and text), Lua preview against a Rust reference interpreter;
- reflected bindings against typed bindings on every case;
- a freshness check that `src/generated.rs` matches what the generator produces now.

As a sanity check, breaking the Lua number literals (three decimals instead of exact) makes the random-program test fail, and proptest shrinks the input to a minimal case.

**Why the nodes have to be Rust even in the preview:** `native_lua_arithmetic_would_diverge` shows Lua's own f64 math giving `2^24 + 1 + 1 = 16777218`, while Rust's f32 gives `16777216`.

**Reflection gives types, not parameter names.** `into_function()` records each argument's type but leaves its name empty, so the pins would be unnamed. The `nodes!` macro fills in the names, which is why the definitions go through it.

**Calling through reflection is too slow for the hot path.** Timing `bob` (4 node calls), release build, median of three runs:

| Path | Time per run | Compared to Rust |
| ---- | ------------ | ---------------- |
| Generated Rust | 6 ns | 1× |
| Lua, typed bindings | 230 ns | ~37× |
| Lua, nodes through reflection | 1,630 ns | ~260× |
| Lua, native math (what `vs_bevy` does today) | 43 ns | ~7× |

Typed bindings are about 7 times faster than reflection and run exactly the same function. Reflection is still useful for the catalog. A 4-node script at 230 ns leaves room for hundreds of scripted entities per frame in the preview, which is enough for editing. Shipped builds use the Rust backend.

## What this leaves for the real implementation

- Statements and state (events, Branch, Flip Flop, Set) in the IR. This experiment only covers data expressions.
- ECS access (transforms, queries) as nodes: same pattern, but the functions take references to components, which needs the glue to pass entity handles.
- Wiring it into the editor and `vs_bevy`, replacing the hand-written node library.
