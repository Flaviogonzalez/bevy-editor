//! Do the Lua preview and the generated Rust give the same bits?

use std::rc::Rc;

use proptest::prelude::*;
use reflect_nodes::catalog::{self, NodeSpec, PinType};
use reflect_nodes::ir::{Expr, Program, Value, lower};
use reflect_nodes::{demo, generated, interpreter, lua_backend, nodes, rust_backend};

struct Setup {
    registry: Rc<bevy_reflect::func::FunctionRegistry>,
    specs: Vec<NodeSpec>,
    /// Nodes called through reflection.
    lua: mlua::Lua,
    /// Nodes called through the typed bindings `nodes!` generates.
    typed: mlua::Lua,
}

fn setup() -> Setup {
    let registry = Rc::new(nodes::registry());
    let (specs, skipped) = catalog::build(&registry);
    assert!(skipped.is_empty(), "{skipped:?}");
    let lua = lua_backend::runtime(registry.clone(), &specs).unwrap();
    let typed = lua_backend::runtime_typed().unwrap();
    Setup { registry, specs, lua, typed }
}

/// Runs a program in both Lua runtimes and checks they agree before returning the result.
fn run_lua(s: &Setup, program: &Program, inputs: &[Value]) -> Value {
    let reflected = {
        let f = lua_backend::load(&s.lua, program).unwrap();
        lua_backend::call(&s.lua, &f, inputs).unwrap()
    };
    let typed = {
        let f = lua_backend::load(&s.typed, program).unwrap();
        lua_backend::call(&s.typed, &f, inputs).unwrap()
    };
    assert_eq!(reflected, typed, "reflected and typed bindings disagree on {}", program.name);
    typed
}

/// The generated Rust, called by program name.
fn run_generated(name: &str, x: &[f32]) -> Value {
    match name {
        "bob" => Value::Number(generated::bob(x[0])),
        "ramp" => Value::Number(generated::ramp(x[0])),
        "larger" => Value::Number(generated::larger(x[0], x[1])),
        "sum3" => Value::Number(generated::sum3(x[0], x[1], x[2])),
        "label" => Value::Text(generated::label(x[0])),
        other => panic!("no generated function for {other}"),
    }
}

#[test]
fn generated_rust_is_current() {
    let s = setup();
    let fresh = rust_backend::emit_module(&demo::programs(&s.specs));
    let on_disk = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/generated.rs")).unwrap();
    assert_eq!(fresh.replace("\r\n", "\n"), on_disk.replace("\r\n", "\n"), "run `cargo run --example codegen`");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// The demo programs: Lua preview against compiled, generated Rust. `any::<f32>()` includes
    /// NaN, infinities, zeros of both signs and subnormals.
    #[test]
    fn demo_programs_match(x in prop::array::uniform3(any::<f32>())) {
        let s = setup();
        for program in demo::programs(&s.specs) {
            let n = program.inputs.len();
            let inputs: Vec<Value> = x[..n].iter().map(|v| Value::Number(*v)).collect();
            let lua = run_lua(&s, &program, &inputs);
            let rust = run_generated(&program.name, &x[..n]);
            prop_assert_eq!(&lua, &rust, "{} with {:?}", program.name, &x[..n]);
        }
    }

    /// Random programs: Lua preview against the Rust reference interpreter.
    #[test]
    fn random_programs_match(seed in any::<u64>(), a in any::<f32>(), b in any::<f32>()) {
        let s = setup();
        let mut rng = Rng(seed | 1);
        let output = [PinType::Number, PinType::Bool, PinType::Text][rng.below(3)];
        let body = random_expr(&mut rng, &s.specs, output, 5);
        let inputs = vec![("a".to_string(), PinType::Number), ("b".to_string(), PinType::Number)];
        let program: Program = lower("random", inputs, &body, &s.specs).unwrap();
        let values = [Value::Number(a), Value::Number(b)];

        let lua = run_lua(&s, &program, &values);
        let reference = interpreter::run(&s.registry, &program, &values);
        prop_assert_eq!(&lua, &reference, "{}", lua_backend::emit(&program));
    }
}

/// Why the nodes run as Rust even in the preview: Lua's own arithmetic is f64, so summing the
/// same numbers natively in Lua gives a different answer than Rust's f32.
#[test]
fn native_lua_arithmetic_would_diverge() {
    let s = setup();
    let (a, b, c) = (16_777_216.0_f32, 1.0, 1.0); // 2^24: the first integer where f32 drops +1
    let native: f64 = s.lua.load(format!("return {a} + {b} + {c}")).eval().unwrap();
    let rust = generated::sum3(a, b, c);
    assert_eq!(rust, 16_777_216.0);
    assert_eq!(native, 16_777_218.0);
    assert_ne!(native as f32, rust);
}

/// Small deterministic generator, so a failing seed reproduces.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_expr(rng: &mut Rng, specs: &[NodeSpec], ty: PinType, depth: u32) -> Expr {
    let producers: Vec<&NodeSpec> = specs.iter().filter(|s| s.output == ty).collect();
    if depth == 0 || producers.is_empty() || rng.below(4) == 0 {
        return match ty {
            PinType::Number => match rng.below(4) {
                0 => Expr::Input(0),
                1 => Expr::Input(1),
                2 => Expr::Number(f32::from_bits(rng.next() as u32)),
                _ => Expr::Number([0.0, -0.0, 1.0, 0.5, 3.0, f32::MAX][rng.below(6)]),
            },
            PinType::Bool => Expr::Bool(rng.below(2) == 0),
            PinType::Text => Expr::Text(["", "x", "ñ", "a b"][rng.below(4)].to_string()),
        };
    }
    let spec = producers[rng.below(producers.len())];
    let args = spec.inputs.iter().map(|(_, t)| random_expr(rng, specs, *t, depth - 1)).collect();
    Expr::Call(spec.name.clone(), args)
}
