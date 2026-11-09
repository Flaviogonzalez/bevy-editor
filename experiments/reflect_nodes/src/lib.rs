//! Experiment: one definition per node, two backends.
//!
//! Nodes are Rust functions (`nodes`). Reflection turns them into the editor's catalog
//! (`catalog`). A graph is lowered to an IR of ordered calls (`ir`), which becomes either Lua glue
//! that calls the Rust functions through reflection (`lua_backend`), or Rust that calls them
//! directly (`rust_backend`). Since both run the same compiled functions, only the glue can
//! differ, and the tests check that it doesn't.

pub mod catalog;
pub mod demo;
pub mod generated;
pub mod interpreter;
pub mod ir;
pub mod lua_backend;
pub mod nodes;
pub mod rust_backend;
