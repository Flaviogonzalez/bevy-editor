//! IR → Lua, for the live preview.
//!
//! The Lua is glue only. Every computation is a call into Rust: pure nodes through the `node`
//! table (`vs_nodes::lua_table`), intrinsics and effects through `vs`. Lua numbers just carry
//! values between those calls, and every consumer takes them as `f32`, so the preview computes
//! with exactly the functions and the precision the generated Rust uses.

use std::fmt::Write as _;

use crate::ir::{Action, ComponentUse, EffectOp, Event, Intrinsic, Lit, Module, Operand, Step};

pub fn emit(module: &Module) -> String {
    let mut lua = String::new();
    if module.variables.is_empty() {
        lua.push_str("local vars = {}\n");
    } else {
        lua.push_str("local vars = {\n");
        for var in &module.variables {
            let _ = writeln!(lua, "    {}, -- {}", lit(&var.initial), var.name);
        }
        lua.push_str("}\n");
    }
    if module.states.is_empty() {
        lua.push_str("local state = {}\n");
    } else {
        let _ = writeln!(lua, "local state = {{ {} }}", vec!["false"; module.states.len()].join(", "));
    }

    if module.has_start() {
        lua.push_str("\nfunction on_script_loaded()\n");
        for block in module.blocks.iter().filter(|b| b.event == Event::Start) {
            let _ = writeln!(lua, "    -- {}", block.comment);
            steps(&mut lua, &block.body, 1, &module.components);
        }
        lua.push_str("end\n");
    }
    if module.has_update() {
        lua.push_str("\nfunction on_update()\n");
        for block in &module.blocks {
            match &block.event {
                Event::Start => {}
                Event::Update => {
                    let _ = writeln!(lua, "    -- {}", block.comment);
                    steps(&mut lua, &block.body, 1, &module.components);
                }
                Event::Key(key) => {
                    let _ = writeln!(lua, "    -- {}", block.comment);
                    let _ = writeln!(lua, "    if vs.input(\"key\", \"pressed\", {key:?}) then");
                    steps(&mut lua, &block.body, 2, &module.components);
                    lua.push_str("    end\n");
                }
            }
        }
        lua.push_str("end\n");
    }
    lua
}

fn pad(depth: usize) -> String {
    "    ".repeat(depth)
}

fn steps(out: &mut String, steps: &[Step], depth: usize, components: &[ComponentUse]) {
    let operand = |op: &Operand| operand(op, components);
    for step in steps {
        let mut p = pad(depth);
        if !step.comment.is_empty() {
            let _ = writeln!(out, "{p}-- {}", step.comment);
        }
        // A `do` block per step keeps locals scoped, so long scripts stay under Lua's limit of
        // 200 locals per function.
        let scoped = !step.lets.is_empty();
        if scoped {
            let _ = writeln!(out, "{p}do");
            p = pad(depth + 1);
            for l in &step.lets {
                let args: Vec<String> = l.args.iter().map(operand).collect();
                let (module, function) = l.node.split_once('.').expect("node kinds are module.function");
                let _ = writeln!(out, "{p}local t{} = node.{module}{}({})", l.local, field(function), args.join(", "));
            }
        }
        let inner = if scoped { depth + 1 } else { depth };
        match &step.action {
            Action::If { cond, then, otherwise } => {
                let _ = writeln!(out, "{p}if {} then", operand(cond));
                self::steps(out, then, inner + 1, components);
                if !otherwise.is_empty() {
                    let _ = writeln!(out, "{p}else");
                    self::steps(out, otherwise, inner + 1, components);
                }
                let _ = writeln!(out, "{p}end");
            }
            Action::SetVar { var, value } => {
                let _ = writeln!(out, "{p}vars[{}] = {}", var + 1, operand(value));
            }
            Action::SetState { slot, value } => {
                let _ = writeln!(out, "{p}state[{}] = {}", slot + 1, operand(value));
            }
            Action::Despawn => {
                let _ = writeln!(out, "{p}vs.despawn(entity)");
            }
            Action::SetField { component, path, value } => {
                let name = &components[*component].name;
                let _ = writeln!(out, "{p}vs.set(entity, {name:?}, {path:?}, {})", operand(value));
            }
            Action::Effect { op, args } => {
                let args: Vec<String> = args.iter().map(operand).collect();
                let args = args.join(", ");
                let _ = match op {
                    EffectOp::Print => writeln!(out, "{p}print({args})"),
                    EffectOp::Translate => writeln!(out, "{p}vs.translate(entity, {args})"),
                    EffectOp::Rotate => writeln!(out, "{p}vs.rotate(entity, {args})"),
                    EffectOp::SetPosition => writeln!(out, "{p}vs.set_position(entity, {args})"),
                    EffectOp::SetScale => writeln!(out, "{p}vs.set_scale(entity, {args})"),
                };
            }
        }
        if scoped {
            let _ = writeln!(out, "{}end", pad(depth));
        }
    }
}

/// `.name`, or `["name"]` when the name is a Lua keyword (`logic.not`, `logic.and`).
fn field(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
        "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    ];
    if KEYWORDS.contains(&name) { format!("[{name:?}]") } else { format!(".{name}") }
}

fn operand(op: &Operand, components: &[ComponentUse]) -> String {
    match op {
        Operand::Field { component, path, .. } => {
            format!("vs.get(entity, {:?}, {path:?})", components[*component].name)
        }
        Operand::Lit(l) => lit(l),
        Operand::Local(n) => format!("t{n}"),
        Operand::Var(i) => format!("vars[{}]", i + 1),
        Operand::State(i) => format!("state[{}]", i + 1),
        Operand::Intrinsic(Intrinsic::Delta) => "vs.delta()".into(),
        Operand::Intrinsic(Intrinsic::Time) => "vs.time()".into(),
        Operand::Intrinsic(Intrinsic::Button { device, press, name }) => {
            format!("vs.input({:?}, {:?}, {name:?})", device.name(), press.name())
        }
        Operand::Intrinsic(Intrinsic::Position(axis)) => format!("vs.position(entity).{}", axis.name()),
    }
}

/// A literal as Lua source. Text uses Rust's escaping, which Lua 5.4 reads the same way.
pub fn lit(l: &Lit) -> String {
    let list = |v: &[f32]| v.iter().map(|n| number(*n)).collect::<Vec<_>>().join(", ");
    match l {
        Lit::Bool(b) => b.to_string(),
        Lit::Text(s) => format!("{s:?}"),
        Lit::Number(n) => number(*n),
        Lit::Vec3(v) => format!("vs.vec3({})", list(v)),
        Lit::Quat(euler) => format!("vs.quat_euler({})", list(euler)),
        Lit::Color(rgba) => format!("vs.color({})", list(rgba)),
    }
}

/// An `f32` as a Lua number that converts back to exactly the same `f32`. The short form
/// (`1.6`) is used when it survives the trip through Lua's `f64`; otherwise the exact `f64`
/// value is written out.
pub fn number(n: f32) -> String {
    if n.is_nan() {
        return "(0/0)".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "math.huge".into() } else { "(-math.huge)".into() };
    }
    let short = if n == n.trunc() && n.abs() < 1e15 {
        // Integers print without a fraction; -0.0 keeps its sign.
        if n == 0.0 && n.is_sign_negative() { "-0.0".to_string() } else { format!("{}", n as i64) }
    } else {
        format!("{n}")
    };
    let back = short.parse::<f64>().map(|v| v as f32);
    if back.is_ok_and(|v| v.to_bits() == n.to_bits()) { short } else { format!("{:?}", n as f64) }
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn numbers_survive_the_trip_through_lua() {
        let lua = mlua::Lua::new();
        let samples = [0.0, -0.0, 1.0, 1.6, 0.1, -2.5, 90.0, 16_777_217.0, f32::MAX, f32::MIN_POSITIVE, 1e-45, 3.4e38];
        for n in samples {
            let text = number(n);
            let back: f32 = lua.load(format!("return {text}")).eval().unwrap();
            assert_eq!(back.to_bits(), n.to_bits(), "{n} written as {text}");
        }
        assert_eq!(number(1.6), "1.6");
        assert_eq!(number(90.0), "90");
    }

    /// Every pure node can be called from the emitted Lua, keywords included.
    #[test]
    fn every_node_is_callable() {
        let lua = mlua::Lua::new();
        lua.globals().set("node", vs_nodes::lua_table(&lua).unwrap()).unwrap();
        for n in vs_nodes::catalog() {
            let (module, function) = n.kind.split_once('.').unwrap();
            let call = format!("return type(node.{module}{})", super::field(function));
            let ty: String = lua.load(call).eval().unwrap();
            assert_eq!(ty, "function", "{}", n.kind);
        }
    }
}
