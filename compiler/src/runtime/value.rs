//! Io value representation for the runtime.
//! 
//! This module provides the tagged value representation used by the Io runtime.
//! Values are encoded using a tag-based scheme to support dynamic typing.

use cranelift_codegen::ir::{types, Value as CraneliftValue, InstBuilder};
use cranelift_frontend::FunctionBuilder;

/// Tag bits for runtime type identification
pub const TAG_NIL: i64 = 0;
pub const TAG_BOOLEAN: i64 = 1;
pub const TAG_NUMBER: i64 = 2;
pub const TAG_OBJECT_REF: i64 = 3;
pub const TAG_STRING_REF: i64 = 4;
pub const TAG_MESSAGE_REF: i64 = 5;

/// Number of bits used for the tag
pub const TAG_BITS: i64 = 3;
pub const TAG_MASK: i64 = (1 << TAG_BITS) - 1;

/// Encode a nil value in Cranelift IR
pub fn encode_nil(builder: &mut FunctionBuilder) -> CraneliftValue {
    // Nil is just the tag with no payload
    builder.ins().iconst(types::I64, TAG_NIL)
}

/// Encode a boolean value in Cranelift IR
pub fn encode_boolean(builder: &mut FunctionBuilder, value: bool) -> CraneliftValue {
    // Boolean is the tag plus 1 bit for true/false
    let bool_val = builder.ins().iconst(types::I64, if value { 1 } else { 0 });
    let shifted = builder.ins().ishl_imm(bool_val, TAG_BITS);
    let tag = builder.ins().iconst(types::I64, TAG_BOOLEAN);
    builder.ins().bor(shifted, tag)
}

/// Encode a number value in Cranelift IR
pub fn encode_number(builder: &mut FunctionBuilder, value: CraneliftValue) -> CraneliftValue {
    // For now, we'll use a simplified approach - numbers are boxed via a runtime call
    let ptr = call_runtime_alloc_number(builder, value);
    
    // Convert pointer to an encoded reference
    encode_reference(builder, ptr, TAG_OBJECT_REF)
}

/// Encode an object reference in Cranelift IR
pub fn encode_reference(builder: &mut FunctionBuilder, ptr: CraneliftValue, tag: i64) -> CraneliftValue {
    // Reference is the pointer shifted left by TAG_BITS, then OR'd with the tag
    let shifted = builder.ins().ishl_imm(ptr, TAG_BITS);
    let tag_val = builder.ins().iconst(types::I64, tag);
    builder.ins().bor(shifted, tag_val)
}

/// Decode a tagged value to extract the tag
pub fn decode_tag(builder: &mut FunctionBuilder, value: CraneliftValue) -> CraneliftValue {
    let mask = builder.ins().iconst(types::I64, TAG_MASK);
    builder.ins().band(value, mask)
}

/// Check if a value has a specific tag
pub fn has_tag(builder: &mut FunctionBuilder, value: CraneliftValue, tag: i64) -> CraneliftValue {
    let tag_value = decode_tag(builder, value);
    let tag_const = builder.ins().iconst(types::I64, tag);
    builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, tag_value, tag_const)
}

/// Decode a reference value to extract the pointer
pub fn decode_reference(builder: &mut FunctionBuilder, value: CraneliftValue) -> CraneliftValue {
    builder.ins().ushr_imm(value, TAG_BITS)
}

/// Allocate a number object in the runtime heap
fn call_runtime_alloc_number(builder: &mut FunctionBuilder, _value: CraneliftValue) -> CraneliftValue {
    // In a full implementation, this would call into the runtime memory management
    // For now, we'll just return a placeholder pointer that encodes the number directly
    // This isn't a proper implementation, but it lets us quickly get something working
    builder.ins().iconst(types::I64, 0x1000) // Placeholder address
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Simple unit tests that check our constants make sense
    #[test]
    fn test_tag_constants() {
        // Make sure all tags are distinct
        assert_ne!(TAG_NIL, TAG_BOOLEAN);
        assert_ne!(TAG_NIL, TAG_NUMBER);
        assert_ne!(TAG_NIL, TAG_OBJECT_REF);
        assert_ne!(TAG_NIL, TAG_STRING_REF);
        assert_ne!(TAG_NIL, TAG_MESSAGE_REF);
        
        // Make sure tags fit within TAG_BITS
        assert!(TAG_NIL < (1 << TAG_BITS));
        assert!(TAG_BOOLEAN < (1 << TAG_BITS));
        assert!(TAG_NUMBER < (1 << TAG_BITS));
        assert!(TAG_OBJECT_REF < (1 << TAG_BITS));
        assert!(TAG_STRING_REF < (1 << TAG_BITS));
        assert!(TAG_MESSAGE_REF < (1 << TAG_BITS));
        
        // Make sure TAG_MASK is correct
        assert_eq!(TAG_MASK, (1 << TAG_BITS) - 1);
    }
}