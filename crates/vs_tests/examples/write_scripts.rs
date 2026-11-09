//! Writes the differential test's scripts into `assets/scripts`: hand-built ones (`demo.lua`,
//! `logic.lua`, `math.lua`, `components.lua`, `input.lua`, `despawn.lua`) and `coverage.lua`,
//! built from the node catalog so it uses every pure node.
//!
//! They're normal visual scripts: open them in the editor to change them.
//!
//! Run it with `cargo run -p vs_tests --example write_scripts` after adding or changing nodes.

use vs_graph::{Graph, PinRef, Value, Variable};

fn link(g: &mut Graph, a: u32, pa: &str, b: u32, pb: &str) {
    assert!(g.link(PinRef::new(a, pa), PinRef::new(b, pb)), "{pa} -> {pb}");
}

fn set(g: &mut Graph, node: u32, pin: &str, value: Value) {
    g.node_mut(node).unwrap().values.insert(pin.into(), value);
}

fn main() {
    // The game's own components, as the preview described them.
    vs_graph::components::load_project(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    write("demo.lua", &demo());
    write("logic.lua", &logic());
    write("math.lua", &math());
    write("coverage.lua", &coverage());
    write("components.lua", &components());
    write("input.lua", &input());
    write("despawn.lua", &despawn());
}

/// Every key and mouse button state, each moving the entity its own way.
fn input() -> Graph {
    let mut g = Graph::default();
    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);
    let update = node(&mut g, "event.update");
    let cases: [(&str, &str, &str, &str, [f64; 3]); 6] = [
        ("input.mouse_down", "button", "Left", "transform.translate", [0.5, 0.0, 0.0]),
        ("input.mouse_pressed", "button", "Right", "transform.translate", [0.0, 1.0, 0.0]),
        ("input.mouse_released", "button", "Left", "transform.translate", [0.0, 0.0, 1.0]),
        ("input.key_pressed", "key", "KeyW", "transform.set_scale", [2.0, 1.0, 0.5]),
        ("input.key_released", "key", "Space", "transform.rotate", [0.0, 10.0, 0.0]),
        ("input.key_down", "key", "KeyR", "transform.translate", [-0.25, 0.0, 0.0]),
    ];
    let mut last = (update, "then");
    for (kind, pin, name, effect, xyz) in cases {
        let ask = node(&mut g, kind);
        set(&mut g, ask, pin, Value::Str(name.into()));
        let seq = node(&mut g, "flow.sequence");
        link(&mut g, last.0, last.1, seq, "in");
        let branch = node(&mut g, "flow.branch");
        link(&mut g, seq, "first", branch, "in");
        // The output is named after the state: down, pressed or released.
        link(&mut g, ask, kind.rsplit('_').next().unwrap(), branch, "condition");
        let act = node(&mut g, effect);
        for (axis, v) in ["x", "y", "z"].into_iter().zip(xyz) {
            set(&mut g, act, axis, Value::Number(v));
        }
        link(&mut g, branch, "true", act, "in");
        last = (seq, "second");
    }
    g
}

/// Drifts, and despawns itself when R is released. The drift after Despawn still runs that frame.
fn despawn() -> Graph {
    let mut g = Graph::default();
    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);
    let update = node(&mut g, "event.update");
    let released = node(&mut g, "input.key_released");
    set(&mut g, released, "key", Value::Str("KeyR".into()));
    let branch = node(&mut g, "flow.branch");
    link(&mut g, update, "then", branch, "in");
    link(&mut g, released, "released", branch, "condition");
    let bye = node(&mut g, "entity.despawn");
    link(&mut g, branch, "true", bye, "in");
    let drift = node(&mut g, "transform.translate");
    set(&mut g, drift, "x", Value::Number(0.125));
    link(&mut g, bye, "then", drift, "in");
    let drift_on = node(&mut g, "transform.translate");
    set(&mut g, drift_on, "y", Value::Number(0.0625));
    link(&mut g, branch, "false", drift_on, "in");
    g
}

fn write(name: &str, g: &Graph) {
    let compiled = vs_graph::compile(g);
    assert!(compiled.problems.is_empty(), "{name}: {:?}", compiled.problems);
    let path = format!("{}/assets/scripts/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&path, vs_graph::save_to_string(g)).unwrap();
    println!("wrote {path}");
}

/// Spins at `speed`, bobs, and toggles its size with Space.
fn demo() -> Graph {
    let mut g = Graph::default();
    g.variables.push(Variable { name: "speed".into(), value: Value::Number(90.0) });

    // On Start: say hello.
    let start = g.add_node("event.start", None, 0.0, -160.0);
    let print = g.add_node("action.print", None, 280.0, -160.0);
    set(&mut g, print, "value", Value::Str("demo script loaded: Space toggles the size".into()));
    link(&mut g, start, "then", print, "in");

    // On Update: spin at `speed` degrees per second, and bob up and down.
    let update = g.add_node("event.update", None, 0.0, 0.0);
    let speed = g.add_node("var.get", Some("speed".into()), 0.0, 120.0);
    let mul = g.add_node("math.mul", None, 260.0, 100.0);
    let rotate = g.add_node("transform.rotate", None, 540.0, 0.0);
    link(&mut g, update, "then", rotate, "in");
    link(&mut g, speed, "value", mul, "a");
    link(&mut g, update, "delta", mul, "b");
    link(&mut g, mul, "result", rotate, "y");

    let time = g.add_node("time.elapsed", None, 0.0, 260.0);
    let twice = g.add_node("math.mul", None, 260.0, 240.0);
    let sin = g.add_node("math.sin", None, 520.0, 240.0);
    let amp = g.add_node("math.mul", None, 780.0, 240.0);
    let lift = g.add_node("math.add", None, 1040.0, 240.0);
    let place = g.add_node("transform.set_position", None, 1300.0, 0.0);
    set(&mut g, twice, "b", Value::Number(2.0));
    set(&mut g, amp, "b", Value::Number(0.25));
    set(&mut g, lift, "b", Value::Number(1.0));
    link(&mut g, time, "seconds", twice, "a");
    link(&mut g, twice, "result", sin, "x");
    link(&mut g, sin, "result", amp, "a");
    link(&mut g, amp, "result", lift, "a");
    link(&mut g, lift, "result", place, "y");
    link(&mut g, rotate, "then", place, "in");

    // On Key Pressed Space: alternate between a big and a normal size.
    let key = g.add_node("event.key", None, 0.0, 420.0);
    let flip = g.add_node("flow.flipflop", None, 280.0, 420.0);
    let big = g.add_node("transform.set_scale", None, 560.0, 380.0);
    let normal = g.add_node("transform.set_scale", None, 560.0, 520.0);
    for axis in ["x", "y", "z"] {
        set(&mut g, big, axis, Value::Number(1.6));
    }
    link(&mut g, key, "then", flip, "in");
    link(&mut g, flip, "a", big, "in");
    link(&mut g, flip, "b", normal, "in");

    g
}

/// Exercises the rest of the language: Do Once, Sequence, Branch, variables of each type, Is Key
/// Down, Get Position, and numbers and booleans turned into text.
fn logic() -> Graph {
    let mut g = Graph::default();
    g.variables.push(Variable { name: "frames".into(), value: Value::Number(0.0) });
    g.variables.push(Variable { name: "label".into(), value: Value::Str("frames: ".into()) });
    g.variables.push(Variable { name: "moved".into(), value: Value::Bool(false) });

    // On Start, once: announce.
    let start = g.add_node("event.start", None, 0.0, -200.0);
    let once = g.add_node("flow.do_once", None, 260.0, -200.0);
    let hello = g.add_node("action.print", None, 520.0, -200.0);
    set(&mut g, hello, "value", Value::Str("logic script loaded: W moves, it resets every 100 frames".into()));
    link(&mut g, start, "then", once, "in");
    link(&mut g, once, "then", hello, "in");

    // On Update: count frames, then move.
    let update = g.add_node("event.update", None, 0.0, 0.0);
    let seq = g.add_node("flow.sequence", None, 260.0, 0.0);
    link(&mut g, update, "then", seq, "in");

    let frames = g.add_node("var.get", Some("frames".into()), 260.0, 140.0);
    let inc = g.add_node("math.add", None, 520.0, 140.0);
    let count = g.add_node("var.set", Some("frames".into()), 780.0, 0.0);
    set(&mut g, inc, "b", Value::Number(1.0));
    link(&mut g, frames, "value", inc, "a");
    link(&mut g, inc, "result", count, "value");
    link(&mut g, seq, "first", count, "in");

    // Every 100 frames: report, go back to the start on x and z, and reset the counter.
    let limit = g.add_node("logic.gt", None, 1040.0, 140.0);
    let every = g.add_node("flow.branch", None, 1040.0, 0.0);
    set(&mut g, limit, "b", Value::Number(99.0));
    link(&mut g, count, "value", limit, "a");
    link(&mut g, limit, "result", every, "condition");
    link(&mut g, count, "then", every, "in");

    let label = g.add_node("var.get", Some("label".into()), 1300.0, 260.0);
    let join = g.add_node("text.join", None, 1560.0, 260.0);
    let report = g.add_node("action.print", None, 1300.0, -60.0);
    link(&mut g, label, "value", join, "a");
    link(&mut g, count, "value", join, "b"); // a number into text
    link(&mut g, join, "result", report, "value");
    link(&mut g, every, "true", report, "in");

    let pos = g.add_node("transform.position", None, 1560.0, 120.0);
    let back = g.add_node("transform.set_position", None, 1560.0, -60.0);
    link(&mut g, pos, "y", back, "y");
    link(&mut g, report, "then", back, "in");
    let reset = g.add_node("var.set", Some("frames".into()), 1820.0, -60.0);
    link(&mut g, back, "then", reset, "in");

    // Second: move forward while W is held, otherwise drift sideways with time.
    let w = g.add_node("input.key_down", None, 260.0, 400.0);
    set(&mut g, w, "key", Value::Str("KeyW".into()));
    let held = g.add_node("flow.branch", None, 520.0, 320.0);
    link(&mut g, w, "down", held, "condition");
    link(&mut g, seq, "second", held, "in");

    let speed = g.add_node("math.mul", None, 780.0, 420.0);
    set(&mut g, speed, "b", Value::Number(-3.0));
    link(&mut g, update, "delta", speed, "a");
    let forward = g.add_node("transform.translate", None, 1040.0, 320.0);
    link(&mut g, speed, "result", forward, "z");
    link(&mut g, held, "true", forward, "in");
    let mark = g.add_node("var.set", Some("moved".into()), 1300.0, 320.0);
    set(&mut g, mark, "value", Value::Bool(true));
    link(&mut g, forward, "then", mark, "in");

    let time = g.add_node("time.elapsed", None, 780.0, 600.0);
    let wave = g.add_node("math.sin", None, 1040.0, 600.0);
    let step = g.add_node("math.mul", None, 1300.0, 600.0);
    link(&mut g, time, "seconds", wave, "x");
    link(&mut g, wave, "result", step, "a");
    link(&mut g, update, "delta", step, "b");
    let drift = g.add_node("transform.translate", None, 1560.0, 480.0);
    link(&mut g, step, "result", drift, "x");
    link(&mut g, held, "false", drift, "in");

    // On Key Pressed R: say whether W was ever used (a boolean into text).
    let r = g.add_node("event.key", None, 0.0, 760.0);
    set(&mut g, r, "key", Value::Str("KeyR".into()));
    let moved = g.add_node("var.get", Some("moved".into()), 0.0, 880.0);
    let tell = g.add_node("action.print", None, 260.0, 760.0);
    link(&mut g, moved, "value", tell, "value"); // a boolean into text
    link(&mut g, r, "then", tell, "in");

    g
}

/// Every pure node, wired so that literals land on both sides (including the receiver of a
/// method, as in `min`), plus constant subgraphs the Rust backend folds. Drives the position and
/// rotation, so the differential test sees every result.
fn math() -> Graph {
    let mut g = Graph::default();
    g.variables.push(Variable { name: "phase".into(), value: Value::Number(0.5) });
    let update = g.add_node("event.update", None, 0.0, 0.0);
    let time = g.add_node("time.elapsed", None, 0.0, 200.0);
    let phase = g.add_node("var.get", Some("phase".into()), 0.0, 300.0);

    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);
    let num = |g: &mut Graph, n: u32, pin: &str, v: f64| set(g, n, pin, Value::Number(v));

    // x = min(2.0, abs(sin(time + phase)) * 3.0) - max(cos(time), -0.5) / 4.0
    let t = node(&mut g, "math.add");
    link(&mut g, time, "seconds", t, "a");
    link(&mut g, phase, "value", t, "b");
    let sin = node(&mut g, "math.sin");
    link(&mut g, t, "result", sin, "x");
    let abs = node(&mut g, "math.abs");
    link(&mut g, sin, "result", abs, "x");
    let mul = node(&mut g, "math.mul");
    link(&mut g, abs, "result", mul, "a");
    num(&mut g, mul, "b", 3.0);
    let min = node(&mut g, "math.min");
    num(&mut g, min, "a", 2.0);
    link(&mut g, mul, "result", min, "b");
    let cos = node(&mut g, "math.cos");
    link(&mut g, time, "seconds", cos, "x");
    let max = node(&mut g, "math.max");
    link(&mut g, cos, "result", max, "a");
    num(&mut g, max, "b", -0.5);
    let div = node(&mut g, "math.div");
    link(&mut g, max, "result", div, "a");
    num(&mut g, div, "b", 4.0);
    let x = node(&mut g, "math.sub");
    link(&mut g, min, "result", x, "a");
    link(&mut g, div, "result", x, "b");

    // A constant subgraph: number(1.5) * sin(0.25), folded to one literal in Rust.
    let c = node(&mut g, "math.number");
    num(&mut g, c, "value", 1.5);
    let csin = node(&mut g, "math.sin");
    num(&mut g, csin, "x", 0.25);
    let y = node(&mut g, "math.mul");
    link(&mut g, c, "result", y, "a");
    link(&mut g, csin, "result", y, "b");

    // z = 1.0 - time, used twice, so it gets a `let`.
    let z = node(&mut g, "math.sub");
    num(&mut g, z, "a", 1.0);
    link(&mut g, time, "seconds", z, "b");

    let place = node(&mut g, "transform.set_position");
    link(&mut g, update, "then", place, "in");
    link(&mut g, x, "result", place, "x");
    link(&mut g, y, "result", place, "y");
    link(&mut g, z, "result", place, "z");
    let spin = node(&mut g, "transform.rotate");
    link(&mut g, place, "then", spin, "in");
    link(&mut g, z, "result", spin, "x");
    num(&mut g, spin, "z", 45.0);

    // Branch on (time > 1 or time < 0.5) and not (x == x), with eq on NaN-free values.
    let gt = node(&mut g, "logic.gt");
    link(&mut g, time, "seconds", gt, "a");
    num(&mut g, gt, "b", 1.0);
    let lt = node(&mut g, "logic.lt");
    link(&mut g, time, "seconds", lt, "a");
    num(&mut g, lt, "b", 0.5);
    let or = node(&mut g, "logic.or");
    link(&mut g, gt, "result", or, "a");
    link(&mut g, lt, "result", or, "b");
    let eq = node(&mut g, "logic.eq");
    link(&mut g, x, "result", eq, "a");
    num(&mut g, eq, "b", 0.0);
    let not = node(&mut g, "logic.not");
    link(&mut g, eq, "result", not, "value");
    let and = node(&mut g, "logic.and");
    link(&mut g, or, "result", and, "a");
    link(&mut g, not, "result", and, "b");
    let branch = node(&mut g, "flow.branch");
    link(&mut g, spin, "then", branch, "in");
    link(&mut g, and, "result", branch, "condition");
    let grow = node(&mut g, "transform.set_scale");
    link(&mut g, branch, "true", grow, "in");
    link(&mut g, z, "result", grow, "y");
    let shrink = node(&mut g, "transform.set_scale");
    for axis in ["x", "y", "z"] {
        num(&mut g, shrink, axis, 0.5);
    }
    link(&mut g, branch, "false", shrink, "in");

    // On Key Pressed Space: print every text node.
    let key = g.add_node("event.key", None, 0.0, 600.0);
    let from_bool = node(&mut g, "text.from_bool");
    link(&mut g, and, "result", from_bool, "value");
    let from_number = node(&mut g, "text.from_number");
    link(&mut g, x, "result", from_number, "value");
    let join = node(&mut g, "text.join");
    link(&mut g, from_bool, "result", join, "a");
    link(&mut g, from_number, "result", join, "b");
    let print = node(&mut g, "action.print");
    link(&mut g, key, "then", print, "in");
    link(&mut g, join, "result", print, "value");

    g
}

/// Every pure node in the catalog, built from the catalog itself, so a new node is covered as soon
/// as this runs again. Each node appears twice: once with inputs that change every frame (and
/// literals on every other input, so literal receivers show up), once with literals only (which the
/// Rust backend folds). Each result is reduced to a number and moves the entity, so the
/// differential test sees it. Text results are printed instead.
fn coverage() -> Graph {
    use vs_graph::PinType;
    use vs_graph::library::pin_type;

    let mut g = Graph::default();
    let update = g.add_node("event.update", None, 0.0, 0.0);
    let time = g.add_node("time.elapsed", None, 0.0, 200.0);
    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);

    // Live sources, one per type, all driven by the time.
    let wave = node(&mut g, "math.sin");
    link(&mut g, time, "seconds", wave, "x");
    let vector = node(&mut g, "vector.new");
    link(&mut g, time, "seconds", vector, "x");
    set(&mut g, vector, "y", Value::Number(0.5));
    link(&mut g, wave, "result", vector, "z");
    let rotation = node(&mut g, "rotation.from_euler");
    link(&mut g, wave, "result", rotation, "x");
    set(&mut g, rotation, "y", Value::Number(30.0));
    let color = node(&mut g, "color.srgba");
    link(&mut g, wave, "result", color, "red");
    set(&mut g, color, "green", Value::Number(0.4));
    link(&mut g, time, "seconds", color, "blue");
    set(&mut g, color, "alpha", Value::Number(1.0));
    let flag = node(&mut g, "logic.gt");
    link(&mut g, wave, "result", flag, "a");
    let text = node(&mut g, "text.from_number");
    link(&mut g, time, "seconds", text, "value");
    let live = |ty: PinType| -> (u32, &'static str) {
        match ty {
            PinType::Number => (time, "seconds"),
            PinType::Vec3 => (vector, "result"),
            PinType::Quat => (rotation, "result"),
            PinType::Color => (color, "result"),
            PinType::Bool => (flag, "result"),
            _ => (text, "result"),
        }
    };
    let literal = |ty: PinType| match ty {
        PinType::Number => Value::Number(0.75),
        PinType::Vec3 => Value::Vec3([1.0, -2.0, 0.5]),
        PinType::Quat => Value::Quat { euler: [10.0, 20.0, 30.0] },
        PinType::Color => Value::Color { rgba: [0.2, 0.4, 0.6, 0.8] },
        PinType::Bool => Value::Bool(true),
        _ => Value::Str("text".into()),
    };

    let mut last = (update, "then");
    for pure in vs_graph::vs_nodes::catalog() {
        for wired in [true, false] {
            let n = node(&mut g, pure.kind);
            for (i, (pin, kind)) in pure.inputs.iter().enumerate() {
                let ty = pin_type(*kind);
                if wired && i % 2 == 0 {
                    let (from, out) = live(ty);
                    link(&mut g, from, out, n, pin);
                } else {
                    set(&mut g, n, pin, literal(ty));
                }
            }
            last = consume(&mut g, last, (n, "result"), pin_type(pure.output));
        }
    }
    g
}

/// Reads and writes component fields: Bevy's `Transform` and the game's own `Wobble`.
fn components() -> Graph {
    let mut g = Graph::default();
    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);
    let get = |g: &mut Graph, field: &str| g.add_field_node("component.get", field, 0.0, 0.0);
    let put = |g: &mut Graph, field: &str| g.add_field_node("component.set", field, 0.0, 0.0);

    let update = node(&mut g, "event.update");
    let time = node(&mut g, "time.elapsed");
    let wave = node(&mut g, "math.sin");
    link(&mut g, time, "seconds", wave, "x");

    // amount += sin(t) / 4
    let amount = get(&mut g, "Wobble.amount");
    let quarter = node(&mut g, "math.mul");
    link(&mut g, wave, "result", quarter, "a");
    set(&mut g, quarter, "b", Value::Number(0.25));
    let more = node(&mut g, "math.add");
    link(&mut g, amount, "value", more, "a");
    link(&mut g, quarter, "result", more, "b");
    let set_amount = put(&mut g, "Wobble.amount");
    link(&mut g, update, "then", set_amount, "in");
    link(&mut g, more, "result", set_amount, "value");

    // axis = (sin(t), 1, 0.5)
    let axis = node(&mut g, "vector.new");
    link(&mut g, wave, "result", axis, "x");
    set(&mut g, axis, "y", Value::Number(1.0));
    set(&mut g, axis, "z", Value::Number(0.5));
    let set_axis = put(&mut g, "Wobble.axis");
    link(&mut g, set_amount, "then", set_axis, "in");
    link(&mut g, axis, "result", set_axis, "value");

    // translation += axis * amount / 10, read back from the component.
    let position = get(&mut g, "Transform.translation");
    let read_axis = get(&mut g, "Wobble.axis");
    let read_amount = get(&mut g, "Wobble.amount");
    let tenth = node(&mut g, "math.mul");
    link(&mut g, read_amount, "value", tenth, "a");
    set(&mut g, tenth, "b", Value::Number(0.1));
    let step = node(&mut g, "vector.scale");
    link(&mut g, read_axis, "value", step, "v");
    link(&mut g, tenth, "result", step, "factor");
    let moved = node(&mut g, "vector.add");
    link(&mut g, position, "value", moved, "a");
    link(&mut g, step, "result", moved, "b");
    let set_position = put(&mut g, "Transform.translation");
    link(&mut g, set_axis, "then", set_position, "in");
    link(&mut g, moved, "result", set_position, "value");

    // tint follows the wave; its red channel nudges the entity sideways.
    let tint = node(&mut g, "color.srgba");
    link(&mut g, wave, "result", tint, "red");
    set(&mut g, tint, "green", Value::Number(0.5));
    set(&mut g, tint, "blue", Value::Number(0.25));
    set(&mut g, tint, "alpha", Value::Number(1.0));
    let set_tint = put(&mut g, "Wobble.tint");
    link(&mut g, set_position, "then", set_tint, "in");
    link(&mut g, tint, "result", set_tint, "value");
    let red = node(&mut g, "color.red");
    let read_tint = get(&mut g, "Wobble.tint");
    link(&mut g, read_tint, "value", red, "color");
    let nudge = node(&mut g, "transform.translate");
    link(&mut g, set_tint, "then", nudge, "in");
    link(&mut g, red, "result", nudge, "x");

    // active = amount > 0; active ? grow : turn.
    let positive = node(&mut g, "logic.gt");
    let amount_now = get(&mut g, "Wobble.amount");
    link(&mut g, amount_now, "value", positive, "a");
    let set_active = put(&mut g, "Wobble.active");
    link(&mut g, nudge, "then", set_active, "in");
    link(&mut g, positive, "result", set_active, "value");
    let branch = node(&mut g, "flow.branch");
    link(&mut g, set_active, "then", branch, "in");
    let active = get(&mut g, "Wobble.active");
    link(&mut g, active, "value", branch, "condition");
    let grow = put(&mut g, "Transform.scale");
    set(&mut g, grow, "value", Value::Vec3([1.5, 1.0, 1.25]));
    link(&mut g, branch, "true", grow, "in");
    let turn = put(&mut g, "Transform.rotation");
    let angle = node(&mut g, "rotation.from_euler");
    link(&mut g, wave, "result", angle, "y");
    link(&mut g, branch, "false", turn, "in");
    link(&mut g, angle, "result", turn, "value");
    g
}

/// Adds the steps that use a value of type `ty`, after `last`: moves the entity by it, or for
/// text prints it. Returns the new last exec output.
fn consume(g: &mut Graph, last: (u32, &'static str), value: (u32, &'static str), ty: vs_graph::PinType) -> (u32, &'static str) {
    use vs_graph::PinType;
    let node = |g: &mut Graph, kind: &str| g.add_node(kind, None, 0.0, 0.0);
    // A number from the value.
    let number: (u32, &'static str) = match ty {
        PinType::Number => value,
        PinType::Vec3 | PinType::Quat => {
            let v = if ty == PinType::Quat {
                let r = node(g, "vector.new");
                set(g, r, "x", Value::Number(1.0));
                set(g, r, "y", Value::Number(2.0));
                set(g, r, "z", Value::Number(3.0));
                let rotate = node(g, "rotation.rotate");
                link(g, value.0, value.1, rotate, "rotation");
                link(g, r, "result", rotate, "v");
                (rotate, "result")
            } else {
                value
            };
            let (x, y, z) = (node(g, "vector.x"), node(g, "vector.y"), node(g, "vector.z"));
            for c in [x, y, z] {
                link(g, v.0, v.1, c, "v");
            }
            let xy = node(g, "math.add");
            link(g, x, "result", xy, "a");
            link(g, y, "result", xy, "b");
            let xyz = node(g, "math.add");
            link(g, xy, "result", xyz, "a");
            link(g, z, "result", xyz, "b");
            (xyz, "result")
        }
        PinType::Color => {
            let parts = ["red", "green", "blue", "alpha"].map(|c| node(g, &format!("color.{c}")));
            let mut sum = (parts[0], "result");
            for p in parts {
                link(g, value.0, value.1, p, "color");
            }
            for p in &parts[1..] {
                let add = node(g, "math.add");
                link(g, sum.0, sum.1, add, "a");
                link(g, *p, "result", add, "b");
                sum = (add, "result");
            }
            sum
        }
        PinType::Bool => {
            // first: branch on the value; second: carry on, whichever arm ran.
            let seq = node(g, "flow.sequence");
            link(g, last.0, last.1, seq, "in");
            let branch = node(g, "flow.branch");
            link(g, seq, "first", branch, "in");
            link(g, value.0, value.1, branch, "condition");
            let up = node(g, "transform.translate");
            set(g, up, "y", Value::Number(0.125));
            link(g, branch, "true", up, "in");
            let down = node(g, "transform.translate");
            set(g, down, "y", Value::Number(-0.0625));
            link(g, branch, "false", down, "in");
            return (seq, "second");
        }
        _ => {
            let print = node(g, "action.print");
            link(g, last.0, last.1, print, "in");
            link(g, value.0, value.1, print, "value");
            return (print, "then");
        }
    };
    // Bounded, so hundreds of moves per frame stay readable numbers. `max` also turns a NaN (the
    // length of a zero vector normalized, say) into -1, so one doesn't hide every node after it.
    let wave = node(g, "math.sin");
    link(g, number.0, number.1, wave, "x");
    let bounded = node(g, "math.max");
    link(g, wave, "result", bounded, "a");
    set(g, bounded, "b", Value::Number(-1.0));
    let step = node(g, "transform.translate");
    link(g, last.0, last.1, step, "in");
    link(g, bounded, "result", step, "x");
    set(g, step, "z", Value::Number(0.0));
    (step, "then")
}
