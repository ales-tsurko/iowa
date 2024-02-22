//! Compiler for Io programming language.

#![warn(
    clippy::all,
    deprecated_in_future,
    missing_docs,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unreachable_pub
)]

use iowa_parser::{Argument, Message, MessageChain};

/// Compile the io message chain into the bytecode.
pub fn compile(chain: MessageChain) {
    todo!()
}
