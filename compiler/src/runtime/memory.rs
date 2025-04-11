//! Memory management system for the Io language runtime.
//!
//! This module provides functions for allocating and managing memory in the Io runtime.
//! It handles object allocation, garbage collection, and memory layout of Io objects.

// Re-export submodules
pub mod allocation;
pub mod gc;
pub mod layout;
