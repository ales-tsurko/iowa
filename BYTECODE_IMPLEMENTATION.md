# Iowa Bytecode VM Implementation

This document describes the bytecode-based virtual machine implemented for the Iowa compiler.

## Overview

The bytecode VM provides an efficient execution model for Iowa methods by converting Io code into a compact bytecode format at method definition time and executing it at runtime using a stack-based virtual machine.

## Architecture

The implementation consists of the following components:

1. **Bytecode Format**: A compact binary representation of Io code that includes opcodes, operands, and a constant pool.
2. **Bytecode Compiler**: Converts Io AST (message chains) to bytecode instructions.
3. **Bytecode VM**: A stack-based virtual machine that executes bytecode instructions.
4. **Runtime Integration**: Connects the bytecode execution model with the existing Iowa runtime system.

## Bytecode Format

The bytecode format consists of:

- **Instruction Sequence**: A series of bytecode instructions, each with an opcode and optional operands.
- **Constant Pool**: A table of constant values (numbers, strings, etc.) referenced by instructions.
- **Local Variable Count**: The number of local variables needed for method execution.

Bytecode is serialized to a binary format with a header containing:
- Magic number "IOBC" to identify Iowa bytecode
- Format version (currently 1)
- Local count
- Constant pool size and entries
- Instruction count and entries

## Instruction Set

The VM implements a comprehensive instruction set for Io:

| Opcode | Description |
|--------|-------------|
| LoadConst | Load a constant value onto the stack |
| LoadLocal | Load a local variable onto the stack |
| StoreLocal | Store a value into a local variable |
| GetSlot | Get a slot from an object |
| SetSlot | Set a slot on an object |
| SendMessage | Send a message to an object |
| Return | Return from method execution |
| Jump | Unconditional jump |
| JumpIfFalse | Conditional jump if top of stack is false |
| JumpIfTrue | Conditional jump if top of stack is true |
| PushNil | Push nil onto the stack |
| PushSelf | Push the receiver onto the stack |
| PushTrue | Push true onto the stack |
| PushFalse | Push false onto the stack |
| CallPrimitive | Call a primitive method |
| End | End of bytecode |

## Bytecode Compilation

The bytecode compiler translates Io AST into bytecode instructions:

1. **Constant Pool Generation**: Literals and message names are collected into a constant pool.
2. **Local Variable Mapping**: Method arguments and local variables are assigned indices.
3. **Instruction Generation**: AST nodes are mapped to corresponding bytecode instructions.
4. **Jump Resolution**: Jump targets for control flow statements are resolved.

## Bytecode VM

The bytecode VM executes bytecode instructions:

1. **Stack-Based Execution**: Values are pushed onto and popped from a stack.
2. **Local Variable Storage**: Local variables are stored in an array.
3. **Message Dispatch**: Messages are dispatched to objects through the Iowa runtime.
4. **Control Flow**: Conditional and unconditional jumps for control flow statements.

## Runtime Integration

The VM integrates with the Iowa runtime in the following ways:

1. **Method Body Storage**: Methods can now be stored as either source code or bytecode.
2. **Method Creation**: Source code is parsed and compiled to bytecode at method definition time.
3. **Method Execution**: The VM is invoked to execute bytecode instructions at method call time.
4. **Value Exchange**: Values are exchanged between the VM and the runtime through method arguments and return values.

## Benefits

The bytecode VM provides several benefits over AST interpretation:

1. **Performance**: Bytecode execution is faster than AST interpretation because the parsing and analysis steps are performed once at method definition time.
2. **Memory Efficiency**: Bytecode is a more compact representation than AST.
3. **Optimization Opportunities**: The bytecode format enables various optimization techniques like inline caching and specialized instruction handlers.

## Future Improvements

Potential future improvements to the bytecode VM include:

1. **Inline Caching**: Cache method lookup results to avoid repeated prototype chain traversals.
2. **Just-In-Time Compilation**: Convert hot bytecode sequences to native machine code.
3. **Optimization Passes**: Apply bytecode optimization passes to eliminate redundant operations.
4. **Type Specialization**: Generate specialized bytecode for common types to avoid runtime checks.
5. **Register-Based VM**: Convert from stack-based to register-based VM for better performance.

## Implementation Status

The current implementation includes:

- Full bytecode instruction set
- Bytecode compiler for Io AST
- Stack-based bytecode VM
- Serialization and deserialization of bytecode
- Runtime integration for method creation and execution
- Support for control flow statements
- Local variable handling
- Primitive operation dispatch

### Known Issues

The implementation currently has a few known issues:

1. **AST to Bytecode Conversion**: The conversion from AST to bytecode needs refinement, especially for certain control flow constructs like return statements.

2. **Stack Management**: Stack handling during control flow operations (jumps, calls) needs careful attention.

3. **Nested Message Chain Handling**: Complex nested message chains may not be properly compiled to bytecode.

4. **Error Handling**: The VM doesn't currently have robust error handling for bytecode errors.

### Testing Status

Basic functionality is tested with unit tests covering:
- Simple constant operations
- Arithmetic operations
- Local variable operations
- Conditional logic
- Method arguments

Manual bytecode tests work correctly, but AST-to-bytecode compilation for more complex cases needs improvement.

### Next Steps

1. **Fix AST to Bytecode Conversion**: Improve the compiler to better handle "return" statements, nested control flow, and complex message chains.

2. **Add Comprehensive Test Suite**: Develop more realistic tests covering complex cases like recursive functions, object manipulation, and nested control flow.

3. **Implement Proper Error Handling**: Add error reporting for bytecode compilation and execution problems.

4. **Add Optimization Passes**: Implement bytecode optimization for better performance.