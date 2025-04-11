//! Memory allocation functions for the Io runtime.
//!
//! This module provides functions for allocating objects, strings,
//! and other data structures in the Io runtime memory.

use cranelift_codegen::ir::{types, Value as CraneliftValue, InstBuilder, MemFlags};
use cranelift_frontend::FunctionBuilder;
use cranelift_codegen::isa::CallConv;
use super::layout;

/// External symbols for runtime memory allocation functions
pub const RUNTIME_ALLOC_OBJECT: &str = "runtime_alloc_object";
pub const RUNTIME_ALLOC_STRING: &str = "runtime_alloc_string";
pub const RUNTIME_ALLOC_NUMBER: &str = "runtime_alloc_number";
pub const RUNTIME_ALLOC_SLOTS_TABLE: &str = "runtime_alloc_slots_table";

/// Allocate a new Io object with the given prototype
pub fn alloc_object(
    builder: &mut FunctionBuilder, 
    prototype: CraneliftValue, 
    initial_slots: u32
) -> CraneliftValue {
    // Call the runtime allocation function
    let size = builder.ins().iconst(types::I32, initial_slots as i64 * 
        layout::slots_table::ENTRY_SIZE as i64 + 
        layout::object::HEADER_SIZE as i64);
    
    let ptr = call_alloc_function(builder, RUNTIME_ALLOC_OBJECT, &[size]);
    
    // Store the prototype in the object header
    builder.ins().store(
        MemFlags::trusted(),
        prototype,
        ptr,
        layout::object::PROTOTYPE_OFFSET
    );
    
    // Initialize the slots table (initially empty)
    let slots_table = alloc_slots_table(builder, initial_slots);
    
    // Store the slots table reference in the object
    builder.ins().store(
        MemFlags::trusted(),
        slots_table,
        ptr,
        layout::object::SLOTS_TABLE_OFFSET
    );
    
    ptr
}

/// Allocate a string object with the given content
/// 
/// This is a placeholder implementation. In a real system, we would:
/// 1. Allocate memory for the string content
/// 2. Copy the string content to the allocated memory
/// 3. Set the length and hash fields
pub fn alloc_string(builder: &mut FunctionBuilder, length: CraneliftValue) -> CraneliftValue {
    // Add header size to get total allocation size
    let header_size = builder.ins().iconst(types::I32, layout::string::HEADER_SIZE as i64);
    let size = builder.ins().iadd(length, header_size);
    
    let ptr = call_alloc_function(builder, RUNTIME_ALLOC_STRING, &[size]);
    
    // Store the length in the string header
    builder.ins().store(
        MemFlags::trusted(),
        length,
        ptr,
        layout::string::LENGTH_OFFSET
    );
    
    // Initialize hash to 0 (will be computed when needed)
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().store(
        MemFlags::trusted(),
        zero,
        ptr,
        layout::string::HASH_OFFSET
    );
    
    ptr
}

/// Allocate a number object to store a numeric value
pub fn alloc_number(builder: &mut FunctionBuilder, value: CraneliftValue) -> CraneliftValue {
    let size = builder.ins().iconst(types::I32, layout::number::SIZE as i64);
    
    let ptr = call_alloc_function(builder, RUNTIME_ALLOC_NUMBER, &[size]);
    
    // Store the numeric value
    builder.ins().store(
        MemFlags::trusted(),
        value,
        ptr,
        layout::number::VALUE_OFFSET
    );
    
    ptr
}

/// Allocate a slots table with the given capacity
fn alloc_slots_table(builder: &mut FunctionBuilder, capacity: u32) -> CraneliftValue {
    let size = builder.ins().iconst(
        types::I32, 
        layout::slots_table::HEADER_SIZE as i64 + 
            capacity as i64 * layout::slots_table::ENTRY_SIZE as i64
    );
    
    let ptr = call_alloc_function(builder, RUNTIME_ALLOC_SLOTS_TABLE, &[size]);
    
    // Store the capacity in the header
    let capacity_val = builder.ins().iconst(types::I32, capacity as i64);
    builder.ins().store(
        MemFlags::trusted(),
        capacity_val,
        ptr,
        layout::slots_table::CAPACITY_OFFSET
    );
    
    // Initialize count to 0
    let zero = builder.ins().iconst(types::I32, 0);
    builder.ins().store(
        MemFlags::trusted(),
        zero,
        ptr,
        layout::slots_table::COUNT_OFFSET
    );
    
    ptr
}

/// Helper function to call a runtime allocation function
fn call_alloc_function(
    builder: &mut FunctionBuilder, 
    func_name: &str, 
    args: &[CraneliftValue]
) -> CraneliftValue {
    // Create a function signature
    let mut sig = cranelift_codegen::ir::Signature::new(
        CallConv::Fast
    );
    
    // Add parameters (typically size)
    for _ in args {
        sig.params.push(cranelift_codegen::ir::AbiParam::new(types::I32));
    }
    
    // Return type is a pointer (i64)
    sig.returns.push(cranelift_codegen::ir::AbiParam::new(types::I64));
    
    // Import the signature first
    let sig_ref = builder.func.import_signature(sig);
    
    // Then import the function
    let callee = builder.import_function(cranelift_codegen::ir::ExtFuncData {
        name: cranelift_codegen::ir::ExternalName::testcase(func_name), // Use testcase as it accepts a &str
        signature: sig_ref,
        colocated: false,
    });
    
    // Call the function
    let inst = builder.ins().call(callee, args);
    builder.inst_results(inst)[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocation_constants() {
        // Test that runtime function names are set correctly
        assert_eq!(RUNTIME_ALLOC_OBJECT, "runtime_alloc_object");
        assert_eq!(RUNTIME_ALLOC_STRING, "runtime_alloc_string");
        assert_eq!(RUNTIME_ALLOC_NUMBER, "runtime_alloc_number");
        assert_eq!(RUNTIME_ALLOC_SLOTS_TABLE, "runtime_alloc_slots_table");
        
        // Test function names are all different
        assert_ne!(RUNTIME_ALLOC_OBJECT, RUNTIME_ALLOC_STRING);
        assert_ne!(RUNTIME_ALLOC_OBJECT, RUNTIME_ALLOC_NUMBER);
        assert_ne!(RUNTIME_ALLOC_STRING, RUNTIME_ALLOC_NUMBER);
    }
}