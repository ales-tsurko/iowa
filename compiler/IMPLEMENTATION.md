# Io Language Compiler Implementation

This document outlines the design and implementation of the Iowa compiler for the Io programming language.


## Overview

Iowa (Io WebAssembly) is a Rust implementation of the Io programming language that uses WebAssembly as its primary intermediate representation. The compiler takes Io source code, parses it into an Abstract Syntax Tree (AST), and compiles it to WebAssembly, which can then be:

1. JIT-compiled and executed directly (for scripting use)
2. Output as a standalone WASM module (for portable deployment)
3. Compiled to native binaries (for optimized distribution)

The WASM-centric approach makes WebAssembly a first-class citizen in the ecosystem and enables seamless interoperation with other WASM modules.


## Architecture

The compiler is organized into the following main components:

1. **Parser** (in `parser` crate)
   - Parses Io source code into a message-based AST
   - Handles operator precedence and desugaring

2. **Compiler** (in `compiler` crate)
   - Translates AST to WebAssembly via Cranelift
   - Implements runtime support for dynamic typing in WASM
   - Handles object system and message dispatch
   - Manages memory and garbage collection

3. **Execution Backends** (in `compiler/src/backend`)
   - Wasmer-based runtime for executing WASM
   - JIT compilation via Cranelift
   - Native binary generation via Cranelift

4. **CLI Interface** (in `cli` crate)
   - Command-line interface for the compiler


## WASM-centric Approach

The iowa project uses WebAssembly as its primary intermediate representation rather than a custom bytecode format. This approach offers several benefits:

1. **Unified IR**: WASM serves as a common format for all compilation targets
2. **Ecosystem Integration**: Seamless interop with other WASM modules
3. **Runtime Reuse**: Leverages existing WASM runtimes (Wasmer) rather than implementing a custom VM
4. **Optimization**: Benefits from optimizations in WASM runtimes and toolchains
5. **Portability**: WASM modules run in standalone runtimes across platforms

The focus is on WASM as a standalone format for dedicated runtimes like Wasmer, not specifically targeting browsers or JavaScript environments. This allows for better integration with native systems while maintaining portability.

### Implementation Details

1. **Io to WASM Mapping**:
   - Dynamic types are represented through runtime type checking in WASM
   - Objects and the prototype system are mapped to appropriate WASM memory structures
   - Message dispatch is implemented through a combination of direct calls and runtime lookups

2. **Runtime Support**:
   - A runtime library implemented in WASM provides core functionality:
     - Type checking and dynamic dispatch
     - Memory management
     - Object system primitives

3. **Wasmer Integration**:
   - Wasmer is used as the runtime for executing and interoperating with WASM modules
   - Provides memory management, function calls, and module loading

4. **JIT and AOT Compilation**:
   - JIT: WASM modules are compiled on-demand for direct execution
   - AOT: WASM modules are pre-compiled to native code for distribution


## Current Implementation Status

### Completed:

- **Parser**
  - AST representation with message chains, arguments, and symbols
  - Operator desugaring and precedence handling
  - Comments and whitespace handling
  - Unicode support

- **Compiler Core**
  - Basic structure for compiling message chains
  - Value representation with tagged pointers
  - Target-agnostic compilation pipeline

- **Message Dispatch**
  - Prototype-based slot lookup system
  - Support for Io's semantics where operators are regular methods
  - Message forwarding support

- **Memory Management**
  - Memory layout definitions for objects, strings, numbers
  - Allocation functions for runtime objects
  - Garbage collection framework with mark and sweep

- **Runtime Support**
  - Value encoding/decoding
  - Object structure with prototype chain

- **Unit Tests**
  - Parser tests
  - Value encoding tests
  - Memory layout tests
  - Message dispatch tests

### In Progress:

- **WASM Generation**
  - ✓ Translating Io AST to WebAssembly
  - ✓ Runtime support for dynamic typing in WASM
  - ✓ Memory management for WebAssembly
  - ✓ IR to WASM instruction translation
  - ✓ Argument handling and message chain compilation
  - ◯ Type compatibility in WASM generation (fixing type mismatch errors)

- **Wasmer Integration**
  - ✓ WASM module loading and execution
  - ✓ Basic runtime imports
  - ◯ Type conversions between i32/i64 for WASM compatibility
  - ◯ Complex argument passing
  - ◯ Interoperation with external WASM modules

- **Runtime Methods**
  - ◯ Implementation of core Io methods
  - ◯ Support for primitive methods

- **Garbage Collection**
  - ◯ Integration with the object system
  - ◯ Mark and sweep implementation

- **Call Frame**
  - ◯ Context for method activation
  - ◯ Call stack handling

### Future Work:

- **Standard Library**
  - Implementation of core Io objects (Number, String, List, etc.)
  - I/O primitives
  - Core methods

- **Optimizations**
  - Method caching
  - Inline caching for slot lookups
  - Specialized primitive operations

- **Native Compilation**
  - AOT compilation from WASM to native binaries

- **Error Reporting**
  - Improved error messages
  - Source location tracking


## Compilation Process

1. **Parsing**: Source code is parsed using Nom parser combinators, producing a tree of `MessageChain` objects.

2. **WASM Generation**: The AST is translated directly to WebAssembly (or through Cranelift IR as an intermediate step):
   - Literal values (numbers, strings)
   - Message sends 
   - Method calls
   - Control flow

3. **Execution Paths**:
   - **JIT**: WASM is JIT-compiled via Cranelift and executed directly
   - **Module**: WASM is output as a standalone module
   - **Native**: WASM is AOT-compiled to native code via Cranelift

4. **Interoperation**: Wasmer enables seamless interoperation with external WASM modules.


## Runtime Components

### Value Representation

Values in the Io language are represented using a tagged pointer scheme:

- **Tag Bits**: Lower 3 bits of the pointer
- **Value Types**:
  - Nil
  - Boolean
  - Number (boxed)
  - Object reference
  - String reference
  - Message reference

### Object System

Objects in Io are represented as:

- **Header**: Containing:
  - Prototype pointer
  - Slots table pointer
- **Slots Table**: A hash table mapping slot names to values
- **Slot Entry**: Contains:
  - Name
  - Value
  - Next pointer (for collision chains)

### Message Dispatch

Message dispatch follows Io's prototype-based semantics:

1. Look up message in receiver's slots
2. If not found, look in receiver's prototype
3. Continue up prototype chain until found or reach an end
4. If not found, try the "forward" slot
5. If forward not found, return nil

### Memory Management

Memory is managed using:

- **WASM Memory**: Linear memory for all allocations
- **Garbage Collection**: Mark and sweep GC
- **Layout Definition**: Precise field offsets and sizes

#### WASM Memory Management Implementation

This section details the implementation of WebAssembly memory management for the Io language compiler:

##### Completed:

1. **Improved WasmMemoryManager with Wasmer 5.x API**
   - ✓ Added support for Wasmer 5.x store-based API
   - ✓ Implemented proper memory read/write operations
   - ✓ Added string caching for common strings

2. **Implemented Memory Layout and Organization**
   - ✓ Added separate memory regions for string and object data
   - ✓ Implemented proper 8-byte alignment for all allocations
   - ✓ Added memory capacity management with automatic growth
   - ✓ Created proper memory initialization for common strings

3. **String and Value Handling**
   - ✓ Created proper string serialization in linear memory
   - ✓ Added string interning for common strings to reduce duplication
   - ✓ Implemented value array serialization for arguments
   - ✓ Added read/write functions for values and strings

4. **Cranelift IR to WASM Translation**
   - ✓ Implementation of IR translation to WASM instructions
   - ✓ Support for basic operations (constants, arithmetic, etc.)
   - ✓ Local variable allocation and management
   - ◯ Type compatibility (need to fix i32/i64 mismatches)

##### In Progress:

1. **Argument Handling**
   - ✓ Implementing proper argument array serialization
   - ✓ Basic support for literals in arguments
   - ✓ Handling nested message chains in arguments
   - ✓ Supporting operator arguments
   - ✓ Passing arguments with proper tags

2. **Message Chain Compilation**
   - ✓ Improving message dispatch to WebAssembly
   - ✓ Proper handling of receiver chaining
   - ✓ Handling complex nested argument chains
   - ◯ Optimizing common patterns
   
3. **Enhanced Nested Argument Handling Requirements**
   - ✓ Create a dedicated argument compilation function that supports all expression types
   - ✓ Support for arbitrarily deep nested message chains within arguments
   - ✓ Memory management system for complex argument structures
   - ✓ Proper scoping and evaluation order for nested expressions
   - ✓ Consistent argument passing convention across different message types
   - ✓ Runtime support for complex argument evaluation in WASM
   
4. **Further Optimization Requirements** (New)
   - ◯ Caching common argument expressions
   - ◯ Specialized compilation paths for common message patterns
   - ◯ Method call inlining for performance-critical code paths
   - ◯ Memory allocation optimization for argument arrays
   
5. **Type Compatibility Issues** (New)
   - ◯ Fix type mismatch errors in WASM validation (i32/i64)
   - ◯ Ensure proper type conversions between Cranelift IR and WASM
   - ◯ Consistent type usage across function calls
   - ◯ Proper handling of return types


## WebAssembly Module Integration

A key feature of iowa is seamless integration with external WASM modules:

1. **Module Import**:
   - Io code can import external WASM modules 
   - Imports appear as normal Io objects

2. **Function Calling**:
   - WASM functions can be called like regular Io methods
   - Type conversion happens automatically

3. **Memory Sharing**:
   - Shared memory between Io runtime and imported modules
   - Safe access through appropriate abstractions

4. **Standalone Execution**:
   - Runs in Wasmer and other standalone WASM runtimes
   - No dependency on browser environments
   - Direct integration with native system resources


## Future Extensions

1. **Extended WASM Features**
   - WASI support for filesystem access
   - WASM component model integration
   - Multi-value returns

2. **Interface with Other Languages**
   - Interoperation with languages that compile to WASM
   - FFI for native libraries

3. **REPL Environment**
   - Interactive development with JIT compilation
   - Debugging and introspection

4. **Embedded Use Cases**
   - Embedding in Rust applications
   - Use as configuration or extension language
   - IoT and edge computing targets