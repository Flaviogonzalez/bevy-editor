//! The IR both backends read: a flat list of node calls in evaluation order.
//!
//! The editor's graph (here, an expression tree) is lowered once. Each call gets its own
//! variable and runs exactly once, in the listed order, so neither backend decides evaluation
//! order or whether to reuse a value.

use crate::catalog::{NodeSpec, PinType};

/// A data expression as the graph describes it.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Input(usize),
    Number(f32),
    Bool(bool),
    Text(String),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Input(usize),
    Var(usize),
    Number(f32),
    Bool(bool),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub var: usize,
    pub node: String,
    pub args: Vec<Operand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub name: String,
    pub inputs: Vec<(String, PinType)>,
    pub output: PinType,
    pub calls: Vec<Call>,
    pub result: Operand,
}

#[derive(Debug, PartialEq)]
pub enum LowerError {
    UnknownNode(String),
    WrongArity { node: String, expected: usize, got: usize },
    WrongType { node: String, arg: String },
}

/// Lowers an expression to calls in post-order, checking every call against the catalog.
pub fn lower(
    name: &str,
    inputs: Vec<(String, PinType)>,
    body: &Expr,
    catalog: &[NodeSpec],
) -> Result<Program, LowerError> {
    let mut calls = Vec::new();
    let (result, output) = lower_expr(body, &inputs, catalog, &mut calls)?;
    Ok(Program { name: name.to_string(), inputs, output, calls, result })
}

fn lower_expr(
    expr: &Expr,
    inputs: &[(String, PinType)],
    catalog: &[NodeSpec],
    calls: &mut Vec<Call>,
) -> Result<(Operand, PinType), LowerError> {
    Ok(match expr {
        Expr::Input(i) => (Operand::Input(*i), inputs[*i].1),
        Expr::Number(n) => (Operand::Number(*n), PinType::Number),
        Expr::Bool(b) => (Operand::Bool(*b), PinType::Bool),
        Expr::Text(t) => (Operand::Text(t.clone()), PinType::Text),
        Expr::Call(node, args) => {
            let spec = catalog
                .iter()
                .find(|s| &s.name == node)
                .ok_or_else(|| LowerError::UnknownNode(node.clone()))?;
            if spec.inputs.len() != args.len() {
                return Err(LowerError::WrongArity {
                    node: node.clone(),
                    expected: spec.inputs.len(),
                    got: args.len(),
                });
            }
            let mut operands = Vec::new();
            for (arg, (arg_name, ty)) in args.iter().zip(&spec.inputs) {
                let (operand, got) = lower_expr(arg, inputs, catalog, calls)?;
                if got != *ty {
                    return Err(LowerError::WrongType { node: node.clone(), arg: arg_name.clone() });
                }
                operands.push(operand);
            }
            let var = calls.len() + 1;
            calls.push(Call { var, node: node.clone(), args: operands });
            (Operand::Var(var), spec.output)
        }
    })
}

/// A value at runtime, shared by the reference interpreter and the tests.
#[derive(Clone, Debug)]
pub enum Value {
    Number(f32),
    Bool(bool),
    Text(String),
}

impl PartialEq for Value {
    /// Bit-exact for numbers, so `-0.0 != 0.0` and NaN equals NaN: "the same result" means the
    /// same bits.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
            }
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            _ => false,
        }
    }
}
