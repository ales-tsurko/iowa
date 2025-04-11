//! Garbage collection for the Io runtime.
//!
//! This module provides functions for garbage collection in the Io runtime,
//! including marking and sweeping objects, and handling memory pressure.

use super::layout;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value as CraneliftValue, types};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::FunctionBuilder;

/// External symbols for runtime garbage collection functions
pub const RUNTIME_GC_COLLECT: &str = "runtime_gc_collect";
pub const RUNTIME_GC_MARK: &str = "runtime_gc_mark";
pub const RUNTIME_GC_CHECK_THRESHOLD: &str = "runtime_gc_check_threshold";

/// Trigger a garbage collection cycle
pub fn collect(builder: &mut FunctionBuilder) -> CraneliftValue {
    // Call the runtime GC function
    call_gc_function(builder, RUNTIME_GC_COLLECT, &[])
}

/// Mark an object as reachable during garbage collection
pub fn mark(builder: &mut FunctionBuilder, object: CraneliftValue) -> CraneliftValue {
    // Call the runtime mark function
    call_gc_function(builder, RUNTIME_GC_MARK, &[object])
}

/// Check if the GC threshold has been reached and run collection if needed
pub fn check_threshold(
    builder: &mut FunctionBuilder,
    allocated_bytes: CraneliftValue,
) -> CraneliftValue {
    // Call the runtime GC check function
    call_gc_function(builder, RUNTIME_GC_CHECK_THRESHOLD, &[allocated_bytes])
}

/// Mark an object's GC bit directly
pub fn mark_object(builder: &mut FunctionBuilder, object: CraneliftValue) {
    // Calculate the address of the mark bit
    let mark_addr = builder
        .ins()
        .iadd_imm(object, layout::gc::MARK_OFFSET as i64);

    // Load the current mark byte
    let mark_byte = builder
        .ins()
        .load(types::I8, MemFlags::trusted(), mark_addr, 0);

    // Set the mark bit
    let mark_bit = builder.ins().iconst(types::I8, 1);
    let new_mark = builder.ins().bor(mark_byte, mark_bit);

    // Store the updated mark byte
    builder
        .ins()
        .store(MemFlags::trusted(), new_mark, mark_addr, 0);
}

/// Check if an object is marked
pub fn is_marked(builder: &mut FunctionBuilder, object: CraneliftValue) -> CraneliftValue {
    // Calculate the address of the mark bit
    let mark_addr = builder
        .ins()
        .iadd_imm(object, layout::gc::MARK_OFFSET as i64);

    // Load the current mark byte
    let mark_byte = builder
        .ins()
        .load(types::I8, MemFlags::trusted(), mark_addr, 0);

    // Check if the mark bit is set
    let mark_bit = builder.ins().iconst(types::I8, 1);
    builder.ins().band(mark_byte, mark_bit)
}

/// Helper function to call a runtime GC function
fn call_gc_function(
    builder: &mut FunctionBuilder,
    func_name: &str,
    args: &[CraneliftValue],
) -> CraneliftValue {
    // Create a function signature
    let mut sig = cranelift_codegen::ir::Signature::new(CallConv::Fast);

    // Add parameters
    for _ in args {
        sig.params
            .push(cranelift_codegen::ir::AbiParam::new(types::I64));
    }

    // Return type is a status code (i32)
    sig.returns
        .push(cranelift_codegen::ir::AbiParam::new(types::I32));

    // Import the signature first
    let sig_ref = builder.func.import_signature(sig);

    // Then import the function
    let callee = builder.import_function(cranelift_codegen::ir::ExtFuncData {
        name: cranelift_codegen::ir::ExternalName::testcase(func_name), /* Use testcase as it
                                                                         * accepts a &str */
        signature: sig_ref,
        colocated: false,
    });

    // Call the function
    let inst = builder.ins().call(callee, args);
    builder.inst_results(inst)[0]
}

#[cfg(test)]
mod tests {
    use super::layout;
    use super::*;

    #[test]
    fn test_gc_constants() {
        // Test that runtime function names are set correctly
        assert_eq!(RUNTIME_GC_COLLECT, "runtime_gc_collect");
        assert_eq!(RUNTIME_GC_MARK, "runtime_gc_mark");
        assert_eq!(RUNTIME_GC_CHECK_THRESHOLD, "runtime_gc_check_threshold");

        // Test function names are all different
        assert_ne!(RUNTIME_GC_COLLECT, RUNTIME_GC_MARK);
        assert_ne!(RUNTIME_GC_COLLECT, RUNTIME_GC_CHECK_THRESHOLD);
        assert_ne!(RUNTIME_GC_MARK, RUNTIME_GC_CHECK_THRESHOLD);
    }

    #[test]
    fn test_gc_layout() {
        // Test that layout values are consistent
        assert_eq!(layout::gc::MARK_OFFSET, 0);
        assert_eq!(layout::gc::SIZE_OFFSET, 4);

        // Size offset should be after mark offset
        assert!(layout::gc::SIZE_OFFSET > layout::gc::MARK_OFFSET);

        // Header size should be at least 8 bytes to hold mark and size fields
        assert!(layout::gc::HEADER_SIZE >= 8);
    }
}
