//! The core Riptide syntax implementation.
//!
//! The provided Riptide parser parses source code into a high-level abstract
//! syntax tree, which can be used for evaluation directly, optimization,
//! formatting tools, etc.

pub mod ast;
pub mod error;
mod grammar;
mod parser;
pub mod source;

pub use parser::parse;
