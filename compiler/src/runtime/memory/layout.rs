//! Memory layout definitions for Io objects.
//!
//! This module defines the structure and layout of Io objects in memory,
//! including field offsets and size calculations.

/// Object header sizes and field offsets
pub mod object {
    /// Size of the object header in bytes
    pub const HEADER_SIZE: usize = 16;

    /// Offset of the prototype field in the object header
    pub const PROTOTYPE_OFFSET: i32 = 0;

    /// Offset of the slots table reference in the object header
    pub const SLOTS_TABLE_OFFSET: i32 = 8;
}

/// String object layout
pub mod string {
    /// Size of the string header in bytes (length + hash)
    pub const HEADER_SIZE: usize = 16;

    /// Offset of the length field in the string header
    pub const LENGTH_OFFSET: i32 = 0;

    /// Offset of the hash field in the string header
    pub const HASH_OFFSET: i32 = 8;

    /// Offset where the actual string data starts
    pub const DATA_OFFSET: i32 = 16;
}

/// Number object layout
pub mod number {
    /// Size of the number object in bytes
    pub const SIZE: usize = 16;

    /// Offset of the double value in the number object
    pub const VALUE_OFFSET: i32 = 8;
}

/// Slots table layout (hash table for object slots)
pub mod slots_table {
    /// Size of the slots table header in bytes
    pub const HEADER_SIZE: usize = 16;

    /// Offset of the capacity field in the slots table header
    pub const CAPACITY_OFFSET: i32 = 0;

    /// Offset of the count field in the slots table header
    pub const COUNT_OFFSET: i32 = 8;

    /// Offset where the actual slot entries start
    pub const ENTRIES_OFFSET: i32 = 16;

    /// Size of each slot entry in bytes (name + value + next)
    pub const ENTRY_SIZE: usize = 24;

    /// Offset of the name field within a slot entry
    pub const ENTRY_NAME_OFFSET: i32 = 0;

    /// Offset of the value field within a slot entry
    pub const ENTRY_VALUE_OFFSET: i32 = 8;

    /// Offset of the next pointer within a slot entry (for collision chains)
    pub const ENTRY_NEXT_OFFSET: i32 = 16;
}

/// Memory management metadata for garbage collection
pub mod gc {
    /// Size of the GC header in bytes
    pub const HEADER_SIZE: usize = 8;

    /// Offset of the mark bit in the GC header
    pub const MARK_OFFSET: i32 = 0;

    /// Offset of the object size in the GC header
    pub const SIZE_OFFSET: i32 = 4;
}
