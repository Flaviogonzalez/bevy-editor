# Hot patching probe

Can Bevy's hot patching (subsecond, through `dx serve --hot-patch`) make a Rust backend iterate as fast as the Lua runtime? `src/main.rs` holds a system written the way a Rust backend could generate it from `demo.lua`.

```sh
cargo install dioxus-cli@0.7.10 --locked   # must match the subsecond version in Cargo.lock
BEVY_ASSET_ROOT="." dx serve --hot-patch --platform windows
```

## Results (Bevy 0.19.1, dx 0.7.10, Windows 11, 2026-11-08)

| Change | Result |
| ------ | ------ |
| Edit a system's body (a constant, the math, a new log line) | Applied live in 2.5 to 3.5 s. Same process, state kept |
| Register a new system in `main` | The patch applies, but the system never runs: `main` already ran. Needs a restart |
| Add a field to a component | The patch applies **silently**, and the existing entity reads the new field as garbage (`bob = 0` instead of `0.5`). This is undefined behavior, with no error and no restart |

Other observations:

- The first `dx` build took about 7.5 minutes, since it uses its own profile, separate from `cargo build`.
- `dx` builds on every file save. When several saves land close together, it can build a half-edited file, fail, and not retry until the next save.
- `dx` labels everything the app writes to stderr as `ERROR`, including Bevy's `INFO` logs.

## What it means for a Rust backend

Changes to a graph's logic (rewiring, constants, math) map to system bodies, so they can be patched. Structural changes (a new variable, a new event, a new script) change component layouts or the schedule, so they need a restart. The editor would have to tell the two apart and restart when needed, because a missed restart corrupts memory without any warning.

One way to make variables safe to patch is to keep them out of the component's layout: store them in fixed-layout storage, such as a `Vec<f64>` indexed by slot, instead of as struct fields.
