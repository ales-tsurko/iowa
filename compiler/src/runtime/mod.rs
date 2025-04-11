//! Runtime components for the Io language compiler.
//!
//! This module provides the runtime support needed for Io language execution,
//! including memory management, object representation, message dispatch, and
//! value encoding/decoding. These components are designed to work with
//! WebAssembly as the primary intermediate representation.

// Export submodules
pub mod dispatch;
pub mod memory;
pub mod object;
pub mod value;

// Runtime implementation that provides actual functions used during execution
pub mod runtime;

// WebAssembly execution module for method execution
pub mod wasm_runtime;
