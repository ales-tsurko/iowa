//! Compiler library for Gobbledygook.
//!
//! This crate owns language semantics: compiler messages, operator tables,
//! assignment rewrites, type and trait declarations, type checking, and code
//! generation. It consumes raw message trees produced by `gg-parser`.
