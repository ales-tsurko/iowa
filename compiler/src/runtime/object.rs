//! Object representation for the Io runtime.
//!
//! This module defines the structure of Io objects and provides functions for
//! working with the prototype-based object system.

use cranelift_codegen::ir::{types, Value as CraneliftValue, InstBuilder, MemFlags};
use cranelift_frontend::FunctionBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use super::{value, memory};

/// Create a new object with the given prototype
pub fn create_object(builder: &mut FunctionBuilder, prototype: CraneliftValue, initial_slots: u32) -> CraneliftValue {
    // Allocate an object using our memory management system
    let ptr = memory::allocation::alloc_object(builder, prototype, initial_slots);
    
    // Return the tagged object reference
    value::encode_reference(builder, ptr, value::TAG_OBJECT_REF)
}

/// Get the prototype of an object
pub fn get_prototype(builder: &mut FunctionBuilder, object: CraneliftValue) -> CraneliftValue {
    // Check if this is an object
    let is_object = value::has_tag(builder, object, value::TAG_OBJECT_REF);
    
    // Create blocks for the two paths
    let load_block = builder.create_block();
    let nil_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add a parameter to done block for the result
    builder.append_block_param(done_block, types::I64);
    
    // Branch based on the tag
    builder.ins().brif(is_object, load_block, &[], nil_block, &[]);
    
    // Load prototype block
    builder.switch_to_block(load_block);
    
    // Extract the object pointer
    let ptr = value::decode_reference(builder, object);
    
    // Load the prototype field using our layout constants
    let prototype = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        ptr,
        memory::layout::object::PROTOTYPE_OFFSET
    );
    
    // Jump to done block with the prototype
    builder.ins().jump(done_block, &[prototype]);
    
    // Nil block for non-object values
    builder.switch_to_block(nil_block);
    let nil_value = value::encode_nil(builder);
    builder.ins().jump(done_block, &[nil_value]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Lookup a slot in an object
pub fn lookup_slot(
    builder: &mut FunctionBuilder, 
    object: CraneliftValue, 
    slot_name: CraneliftValue
) -> CraneliftValue {
    // Create blocks for our lookup logic
    let start_block = builder.create_block();
    let prototype_block = builder.create_block();
    let prototype_lookup_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add parameters to blocks
    builder.append_block_param(done_block, types::I64);
    builder.append_block_param(prototype_lookup_block, types::I64); // prototype
    
    // Jump to start block
    builder.ins().jump(start_block, &[]);
    builder.switch_to_block(start_block);
    
    // Try to look up the slot in this object
    let slot_value = lookup_slot_in_object(builder, object, slot_name);
    
    // Create a nil value to compare against
    let nil_value = value::encode_nil(builder);
    
    // Check if the slot was found
    let is_nil = builder.ins().icmp(IntCC::Equal, slot_value, nil_value);
    
    // If nil, try the prototype chain, otherwise return the value
    builder.ins().brif(is_nil, prototype_block, &[], done_block, &[slot_value]);
    
    // Prototype lookup block
    builder.switch_to_block(prototype_block);
    
    // Get the prototype
    let prototype = get_prototype(builder, object);
    
    // Check if prototype is nil (end of chain)
    let proto_is_nil = builder.ins().icmp(IntCC::Equal, prototype, nil_value);
    
    // If prototype is nil, we've reached the end, otherwise continue with prototype lookup
    builder.ins().brif(proto_is_nil, done_block, &[nil_value], prototype_lookup_block, &[prototype]);
    
    // Prototype lookup continuation block - avoids recursion
    builder.switch_to_block(prototype_lookup_block);
    let proto = builder.block_params(prototype_lookup_block)[0];
    
    // Call lookup_slot_in_object on the prototype
    let proto_slot = lookup_slot_in_object(builder, proto, slot_name);
    let proto_slot_is_nil = builder.ins().icmp(IntCC::Equal, proto_slot, nil_value);
    
    // Create blocks for recursive lookup
    let proto_of_proto_block = builder.create_block();
    
    // If slot not found in prototype, check prototype of prototype
    builder.ins().brif(proto_slot_is_nil, proto_of_proto_block, &[], done_block, &[proto_slot]);
    
    // Prototype of prototype block
    builder.switch_to_block(proto_of_proto_block);
    
    // Get prototype of prototype
    let proto_of_proto = get_prototype(builder, proto);
    
    // If there's no more prototype, we're done, otherwise continue lookup loop
    let proto_of_proto_is_nil = builder.ins().icmp(IntCC::Equal, proto_of_proto, nil_value);
    builder.ins().brif(proto_of_proto_is_nil, done_block, &[nil_value], prototype_lookup_block, &[proto_of_proto]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Look up a slot in a specific object (not traversing prototype chain)
fn lookup_slot_in_object(
    builder: &mut FunctionBuilder,
    object: CraneliftValue,
    slot_name: CraneliftValue
) -> CraneliftValue {
    // Create blocks for our lookup logic
    let start_lookup_block = builder.create_block();
    let not_object_block = builder.create_block();
    let get_slots_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add parameter to done block for the result
    builder.append_block_param(done_block, types::I64);
    
    // First check if this is an object
    let is_object = value::has_tag(builder, object, value::TAG_OBJECT_REF);
    builder.ins().brif(is_object, start_lookup_block, &[], not_object_block, &[]);
    
    // Start lookup block
    builder.switch_to_block(start_lookup_block);
    
    // Get the object pointer
    let obj_ptr = value::decode_reference(builder, object);
    
    // Load the slots table pointer
    let slots_ptr = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        obj_ptr,
        memory::layout::object::SLOTS_TABLE_OFFSET
    );
    
    // Jump to get slots block
    builder.ins().jump(get_slots_block, &[slots_ptr]);
    
    // Not object block - return nil for non-objects
    builder.switch_to_block(not_object_block);
    let nil_value = value::encode_nil(builder);
    builder.ins().jump(done_block, &[nil_value]);
    
    // Get slots block - find slot in the slots table
    builder.switch_to_block(get_slots_block);
    builder.append_block_param(get_slots_block, types::I64); // slots_ptr param
    
    // Extract the slot name as string (assumed to be a string reference)
    let name_ptr = value::decode_reference(builder, slot_name);
    
    // Implement hash table lookup for slots
    // For simplicity in this implementation, we'll do a sequential scan
    // In a real implementation, this would use the hash value
    
    // Get the slots table pointer from the block params
    let slots_ptr = builder.block_params(get_slots_block)[0];
    
    // Load capacity
    let capacity = builder.ins().load(
        types::I32,
        MemFlags::trusted(),
        slots_ptr,
        memory::layout::slots_table::CAPACITY_OFFSET
    );
    
    // Set up for scan loop
    let entry_start = builder.ins().iadd_imm(
        slots_ptr,
        memory::layout::slots_table::ENTRIES_OFFSET as i64
    );
    
    // Scan loop blocks
    let loop_header = builder.create_block();
    let loop_body = builder.create_block();
    let name_match = builder.create_block();
    let loop_next = builder.create_block();
    
    // Add parameters to blocks
    builder.append_block_param(loop_header, types::I64); // current entry
    builder.append_block_param(loop_header, types::I32); // index
    
    builder.append_block_param(loop_body, types::I64); // current entry
    builder.append_block_param(loop_body, types::I32); // index
    
    builder.append_block_param(loop_next, types::I64); // current entry
    builder.append_block_param(loop_next, types::I32); // index
    
    // Start the loop
    let zero_idx = builder.ins().iconst(types::I32, 0);
    builder.ins().jump(loop_header, &[entry_start, zero_idx]);
    
    // Loop header
    builder.switch_to_block(loop_header);
    let curr_idx = builder.block_params(loop_header)[1];
    let curr_entry = builder.block_params(loop_header)[0];
    
    // Check if we've reached the end of the table
    let at_end = builder.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, curr_idx, capacity);
    builder.ins().brif(at_end, done_block, &[nil_value], loop_body, &[curr_entry, curr_idx]);
    
    // Loop body
    builder.switch_to_block(loop_body);
    let entry_ptr = builder.block_params(loop_body)[0];
    let idx = builder.block_params(loop_body)[1];
    
    // Load the slot name
    let entry_name = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        entry_ptr,
        memory::layout::slots_table::ENTRY_NAME_OFFSET
    );
    
    // Check if entry is empty (name is 0)
    let is_empty = builder.ins().icmp_imm(IntCC::Equal, entry_name, 0);
    
    // If empty, skip to next entry, otherwise check name
    builder.ins().brif(is_empty, loop_next, &[entry_ptr, idx], name_match, &[]);
    
    // Name match block - compare names
    builder.switch_to_block(name_match);
    
    // In a real implementation, we would compare the strings
    // For simplicity, compare the pointers directly here
    let name_equals = builder.ins().icmp(IntCC::Equal, entry_name, name_ptr);
    
    // Load the value first to avoid nested builder calls
    let entry_value = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        entry_ptr,
        memory::layout::slots_table::ENTRY_VALUE_OFFSET
    );
    
    // If match found, return the value
    builder.ins().brif(name_equals, done_block, &[entry_value], loop_next, &[entry_ptr, idx]);
    
    // Loop next block - move to next entry
    builder.switch_to_block(loop_next);
    
    // Get the current entry and index from the block params
    let curr_entry = builder.block_params(loop_next)[0];
    let curr_idx = builder.block_params(loop_next)[1];
    
    // Calculate the next index and entry pointer
    let next_idx = builder.ins().iadd_imm(curr_idx, 1);
    let next_entry = builder.ins().iadd_imm(
        curr_entry, 
        memory::layout::slots_table::ENTRY_SIZE as i64
    );
    
    // Jump to loop header with the next entry and index
    builder.ins().jump(loop_header, &[next_entry, next_idx]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Set a slot in an object
pub fn set_slot(
    builder: &mut FunctionBuilder, 
    object: CraneliftValue, 
    slot_name: CraneliftValue, 
    value: CraneliftValue
) -> CraneliftValue {
    // Check if this is an object
    let is_object = value::has_tag(builder, object, value::TAG_OBJECT_REF);
    
    // Create blocks for the two paths
    let set_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add a parameter to done block for the result
    builder.append_block_param(done_block, types::I64);
    
    // Branch based on the tag
    builder.ins().brif(is_object, set_block, &[], done_block, &[value]);
    
    // Set slot block
    builder.switch_to_block(set_block);
    
    // Extract the object pointer
    let obj_ptr = value::decode_reference(builder, object);
    
    // Load the slots table pointer
    let slots_ptr = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        obj_ptr,
        memory::layout::object::SLOTS_TABLE_OFFSET
    );
    
    // Get slot name pointer
    let name_ptr = value::decode_reference(builder, slot_name);
    
    // First, try to find if the slot already exists
    // Same logic as lookup_slot_in_object, but modified to update existing or add new
    
    // Load capacity and count
    let capacity = builder.ins().load(
        types::I32,
        MemFlags::trusted(),
        slots_ptr,
        memory::layout::slots_table::CAPACITY_OFFSET
    );
    
    // Load count (will be used in full implementation)
    let _count = builder.ins().load(
        types::I32,
        MemFlags::trusted(),
        slots_ptr,
        memory::layout::slots_table::COUNT_OFFSET
    );
    
    // Set up for scan loop
    let entry_start = builder.ins().iadd_imm(
        slots_ptr,
        memory::layout::slots_table::ENTRIES_OFFSET as i64
    );
    
    // Scan loop blocks
    let loop_header = builder.create_block();
    let loop_body = builder.create_block();
    let name_match = builder.create_block();
    let loop_next = builder.create_block();
    let add_new_slot = builder.create_block();
    
    // Add parameters to blocks
    builder.append_block_param(loop_header, types::I64); // current entry
    builder.append_block_param(loop_header, types::I32); // index
    builder.append_block_param(loop_header, types::I64); // first empty slot
    
    builder.append_block_param(loop_body, types::I64); // current entry
    builder.append_block_param(loop_body, types::I32); // index
    builder.append_block_param(loop_body, types::I64); // first empty slot
    
    builder.append_block_param(loop_next, types::I64); // current entry
    builder.append_block_param(loop_next, types::I32); // index
    builder.append_block_param(loop_next, types::I64); // first empty slot
    
    // Start the loop
    let zero_idx = builder.ins().iconst(types::I32, 0);
    let zero_ptr = builder.ins().iconst(types::I64, 0);
    builder.ins().jump(loop_header, &[entry_start, zero_idx, zero_ptr]);
    
    // Loop header
    builder.switch_to_block(loop_header);
    let curr_idx = builder.block_params(loop_header)[1];
    let curr_entry = builder.block_params(loop_header)[0];
    let first_empty = builder.block_params(loop_header)[2];
    
    // Check if we've reached the end of the table
    let at_end = builder.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, curr_idx, capacity);
    builder.ins().brif(at_end, add_new_slot, &[first_empty], loop_body, &[curr_entry, curr_idx, first_empty]);
    
    // Loop body
    builder.switch_to_block(loop_body);
    let entry_ptr = builder.block_params(loop_body)[0];
    let idx = builder.block_params(loop_body)[1];
    let empty_slot = builder.block_params(loop_body)[2];
    
    // Load the slot name
    let entry_name = builder.ins().load(
        types::I64,
        MemFlags::trusted(),
        entry_ptr,
        memory::layout::slots_table::ENTRY_NAME_OFFSET
    );
    
    // Check if entry is empty (name is 0)
    let is_empty = builder.ins().icmp_imm(IntCC::Equal, entry_name, 0);
    
    // Update first empty slot if needed
    let updated_empty = update_first_empty(builder, empty_slot, entry_ptr, is_empty);
    
    // If empty, skip to next entry, otherwise check name
    builder.ins().brif(is_empty, loop_next, &[entry_ptr, idx, updated_empty], name_match, &[]);
    
    // Name match block - compare names
    builder.switch_to_block(name_match);
    
    // In a real implementation, we would compare the strings
    // For simplicity, compare the pointers directly here
    let name_equals = builder.ins().icmp(IntCC::Equal, entry_name, name_ptr);
    
    // Update slot value if the name matches
    let updated_value = update_slot_value(builder, entry_ptr, value);
    
    // If match found, update the value and return
    builder.ins().brif(name_equals, done_block, &[updated_value], loop_next, &[entry_ptr, idx, empty_slot]);
    
    // Loop next block - move to next entry
    builder.switch_to_block(loop_next);
    
    // Get the current values from block params
    let curr_entry = builder.block_params(loop_next)[0];
    let curr_idx = builder.block_params(loop_next)[1];
    let curr_empty = builder.block_params(loop_next)[2];
    
    // Calculate the next index and entry pointer
    let next_idx = builder.ins().iadd_imm(curr_idx, 1);
    let next_entry = builder.ins().iadd_imm(
        curr_entry, 
        memory::layout::slots_table::ENTRY_SIZE as i64
    );
    
    // Jump to loop header with updated values
    builder.ins().jump(loop_header, &[next_entry, next_idx, curr_empty]);
    
    // Add new slot block - add to first empty or resize if needed
    builder.switch_to_block(add_new_slot);
    let empty_slot_ptr = builder.block_params(add_new_slot)[0];
    
    // Check if we found an empty slot
    let has_empty = builder.ins().icmp_imm(IntCC::NotEqual, empty_slot_ptr, 0);
    
    // Create blocks for adding vs expanding
    let add_to_empty = builder.create_block();
    let expand_table = builder.create_block();
    
    // Branch based on whether we have an empty slot
    builder.ins().brif(has_empty, add_to_empty, &[], expand_table, &[]);
    
    // Add to empty slot block
    builder.switch_to_block(add_to_empty);
    let result = add_slot_to_entry(builder, empty_slot_ptr, slots_ptr, name_ptr, value);
    builder.ins().jump(done_block, &[result]);
    
    // Expand table block (not implemented in detail)
    builder.switch_to_block(expand_table);
    // In a real implementation, we would resize the table
    // For now, just return the value without setting the slot
    builder.ins().jump(done_block, &[value]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Helper function to update the first empty slot tracker
fn update_first_empty(
    builder: &mut FunctionBuilder, 
    current: CraneliftValue, 
    entry: CraneliftValue, 
    is_empty: CraneliftValue
) -> CraneliftValue {
    // Create select blocks
    let select_block = builder.create_block();
    let compute_block = builder.create_block();
    
    // Add parameters to blocks
    builder.append_block_param(select_block, types::I64);
    builder.append_block_param(compute_block, types::I64); // is_set
    
    // Check if current is already set
    let is_set = builder.ins().icmp_imm(IntCC::NotEqual, current, 0);
    
    // Jump to compute block to continue computation
    builder.ins().jump(compute_block, &[is_set]);
    
    // Compute if we should use current
    builder.switch_to_block(compute_block);
    let is_set = builder.block_params(compute_block)[0];
    
    // If already set or if entry is not empty, use current
    let is_entry_not_empty = builder.ins().icmp_imm(IntCC::Equal, is_empty, 0);
    let use_current = builder.ins().bor(is_set, is_entry_not_empty);
    
    // Branch based on computed flag
    builder.ins().brif(use_current, select_block, &[current], select_block, &[entry]);
    
    // Select block - return the chosen value
    builder.switch_to_block(select_block);
    builder.block_params(select_block)[0]
}

/// Helper function to update a slot value
fn update_slot_value(
    builder: &mut FunctionBuilder, 
    entry_ptr: CraneliftValue, 
    value: CraneliftValue
) -> CraneliftValue {
    // Store the new value
    builder.ins().store(
        MemFlags::trusted(),
        value,
        entry_ptr,
        memory::layout::slots_table::ENTRY_VALUE_OFFSET
    );
    
    // Return the value
    value
}

/// Helper function to add a slot to an empty entry
fn add_slot_to_entry(
    builder: &mut FunctionBuilder,
    entry_ptr: CraneliftValue,
    slots_ptr: CraneliftValue,
    name_ptr: CraneliftValue,
    value: CraneliftValue
) -> CraneliftValue {
    // Store the name
    builder.ins().store(
        MemFlags::trusted(),
        name_ptr,
        entry_ptr,
        memory::layout::slots_table::ENTRY_NAME_OFFSET
    );
    
    // Store the value
    builder.ins().store(
        MemFlags::trusted(),
        value,
        entry_ptr,
        memory::layout::slots_table::ENTRY_VALUE_OFFSET
    );
    
    // Clear the next pointer
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().store(
        MemFlags::trusted(),
        zero,
        entry_ptr,
        memory::layout::slots_table::ENTRY_NEXT_OFFSET
    );
    
    // Increment count
    let old_count = builder.ins().load(
        types::I32,
        MemFlags::trusted(),
        slots_ptr,
        memory::layout::slots_table::COUNT_OFFSET
    );
    let new_count = builder.ins().iadd_imm(old_count, 1);
    builder.ins().store(
        MemFlags::trusted(),
        new_count,
        slots_ptr,
        memory::layout::slots_table::COUNT_OFFSET
    );
    
    // Return the value
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::types;
    use memory::layout;
    
    #[test]
    fn test_memory_layout_consistency() {
        // Test that layout offsets are consistent
        assert_eq!(layout::object::PROTOTYPE_OFFSET, 0);
        assert_eq!(layout::object::SLOTS_TABLE_OFFSET, 8);
        
        // Test that slot entry layout makes sense
        assert!(layout::slots_table::ENTRY_SIZE >= 24); // Ensure entries can hold 3 pointers
        
        // Offsets shouldn't be negative
        assert!(layout::slots_table::ENTRIES_OFFSET > 0);
        assert!(layout::slots_table::CAPACITY_OFFSET >= 0);
        assert!(layout::slots_table::COUNT_OFFSET >= 0);
    }
}