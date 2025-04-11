//! Runtime components for the Io language compiler.
//!
//! This module provides the runtime support needed for Io language execution,
//! including memory management, object representation, message dispatch, and
//! value encoding/decoding.

// Export submodules
pub mod value;
pub mod object;
pub mod dispatch;
pub mod memory;