//! A few graphs, as the editor would hand them to the compiler.

use crate::catalog::{NodeSpec, PinType};
use crate::ir::{Expr, Program, lower};

fn call(node: &str, args: Vec<Expr>) -> Expr {
    Expr::Call(node.to_string(), args)
}

fn num(n: f32) -> Expr {
    Expr::Number(n)
}

pub fn programs(catalog: &[NodeSpec]) -> Vec<Program> {
    use Expr::Input;
    let number = |name: &str| (name.to_string(), PinType::Number);
    let graphs = vec![
        // The demo cube's bob: sin(t × 2) × 0.25 + 1
        ("bob", vec![number("t")], call("add", vec![
            call("mul", vec![call("sin", vec![call("mul", vec![Input(0), num(2.0)])]), num(0.25)]),
            num(1.0),
        ])),
        // A 0..1 ramp over three seconds.
        ("ramp", vec![number("x")], call("clamp", vec![call("div", vec![Input(0), num(3.0)]), num(0.0), num(1.0)])),
        // The larger of two numbers.
        ("larger", vec![number("a"), number("b")], call("select", vec![
            call("greater", vec![Input(0), Input(1)]),
            Input(0),
            Input(1),
        ])),
        // Three numbers summed in f32, where rounding shows.
        ("sum3", vec![number("a"), number("b"), number("c")], call("add", vec![
            call("add", vec![Input(0), Input(1)]),
            Input(2),
        ])),
        // Text built from a number: formatting has to match too.
        ("label", vec![number("x")], call("join", vec![
            Expr::Text("value: ".into()),
            call("number_to_text", vec![call("mul", vec![Input(0), num(-0.5)])]),
        ])),
    ];
    graphs
        .into_iter()
        .map(|(name, inputs, body)| lower(name, inputs, &body, catalog).expect("demo graphs are valid"))
        .collect()
}
