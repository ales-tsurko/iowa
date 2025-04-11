# Io Language Compiler Implementation

This document outlines the design and implementation of the Iowa compiler for the Io programming language.


## Overview

The compiler takes Io source code, parses it into an Abstract Syntax Tree (AST), and compiles it to either:
1. Native code via JIT compilation (for direct execution)
2. WebAssembly (for portable deployment)

Both targets use Cranelift as the IR and code generation backend.


## Architecture

The compiler is organized into the following main components:

1. **Parser** (in `parser` crate)
   - Parses Io source code into a message-based AST
   - Handles operator precedence and desugaring

2. **Compiler** (in `compiler` crate)
   - Core compilation from AST to Cranelift IR
   - Value encoding/tagging
   - Message dispatch system
   - Memory management
   - Runtime support

3. **Runtime Backends** (in `compiler/src/backend`)
   - JIT compilation target
   - WebAssembly compilation target

4. **CLI Interface** (in `cli` crate)
   - Command-line interface for the compiler


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

- **Runtime Methods**
  - Implementation of core Io methods
  - Support for primitive methods

- **Garbage Collection**
  - Integration with the object system
  - Mark and sweep implementation

- **Call Frame**
  - Context for method activation
  - Call stack handling

### Future Work:

- **Standard Library**
  - Implementation of core Io objects (Number, String, List, etc.)
  - I/O primitives
  - Core methods

- **WebAssembly Target**
  - Proper memory management for WASM
  - Interop with JavaScript

- **Optimizations**
  - Method caching
  - Inline caching for slot lookups
  - Specialized primitive operations

- **Error Reporting**
  - Improved error messages
  - Source location tracking


## Compilation Process

1. **Parsing**: Source code is parsed using Nom parser combinators, producing a tree of `MessageChain` objects.

2. **Compilation**: The compiler walks the AST, generating Cranelift IR for:
   - Literal values (numbers, strings)
   - Message sends
   - Method calls
   - Control flow

3. **Code Generation**: The Cranelift IR is compiled to either:
   - Native code (via JIT)
   - WebAssembly module

4. **Execution**: For JIT, the resulting code is executed directly. For WASM, the module is returned for external execution.


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

- **Explicit Allocation**: Objects, strings, etc.
- **Garbage Collection**: Mark and sweep GC
- **Layout Definition**: Precise field offsets and sizes


## Future Extensions

1. **Concurrency Support**
   - Coroutines
   - Actors

2. **Interface with Other Languages**
   - FFI for C/C++ libraries

3. **Bytecode Interpreter**
   - For debugging and faster startup

4. **REPL Environment**
   - Interactive development