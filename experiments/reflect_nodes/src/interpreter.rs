//! A reference interpreter: runs the IR in Rust by calling the registry dynamically.
//!
//! The differential tests compare the Lua preview against this for random programs, since
//! random programs can't be compiled as Rust while the tests run.

use bevy_reflect::func::FunctionRegistry;
use bevy_reflect::func::args::ArgList;

use crate::ir::{Operand, Program, Value};

pub fn run(registry: &FunctionRegistry, program: &Program, inputs: &[Value]) -> Value {
    let mut vars: Vec<Value> = Vec::with_capacity(program.calls.len());
    let get = |op: &Operand, vars: &[Value]| -> Value {
        match op {
            Operand::Input(i) => inputs[*i].clone(),
            Operand::Var(v) => vars[*v - 1].clone(),
            Operand::Number(n) => Value::Number(*n),
            Operand::Bool(b) => Value::Bool(*b),
            Operand::Text(t) => Value::Text(t.clone()),
        }
    };
    for call in &program.calls {
        let mut args = ArgList::default();
        for op in &call.args {
            match get(op, &vars) {
                Value::Number(n) => args.push_owned(n),
                Value::Bool(b) => args.push_owned(b),
                Value::Text(t) => args.push_owned(t),
            }
        }
        let result = registry.call(&call.node, args).expect("lowered calls exist").unwrap();
        let result = result.unwrap_owned();
        let value = if let Some(n) = result.try_downcast_ref::<f32>() {
            Value::Number(*n)
        } else if let Some(b) = result.try_downcast_ref::<bool>() {
            Value::Bool(*b)
        } else {
            Value::Text(result.try_downcast_ref::<String>().expect("catalog types only").clone())
        };
        vars.push(value);
    }
    get(&program.result, &vars)
}
