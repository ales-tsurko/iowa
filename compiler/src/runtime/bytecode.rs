//! Bytecode virtual machine for Io language.
//!
//! This module provides a bytecode-based execution model for Io methods.
//! It compiles AST to bytecode at method definition time and executes
//! bytecode instructions at runtime.

use crate::runtime::runtime::{Runtime, Value, MethodData};
use iowa_parser::{MessageChain, Message, Symbol};

/// Opcode definitions for the bytecode VM
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    /// Load a constant value onto the stack (index into constant pool)
    LoadConst = 0x01,
    /// Load a local variable onto the stack (index into locals)
    LoadLocal = 0x02,
    /// Store a value into a local variable (index into locals)
    StoreLocal = 0x03,
    /// Get a slot from an object (slot name index into constant pool)
    GetSlot = 0x04,
    /// Set a slot on an object (slot name index into constant pool)
    SetSlot = 0x05,
    /// Send a message (message name index into constant pool, arg count)
    SendMessage = 0x06,
    /// Return the top value of the stack
    Return = 0x07,
    /// Jump by the given offset
    Jump = 0x08,
    /// Jump if the top value of the stack is false or nil
    JumpIfFalse = 0x09,
    /// Jump if the top value of the stack is true (not nil or false)
    JumpIfTrue = 0x0A,
    /// Push nil onto the stack
    PushNil = 0x0B,
    /// Push self (the receiver) onto the stack
    PushSelf = 0x0C,
    /// Push true onto the stack
    PushTrue = 0x0D,
    /// Push false onto the stack
    PushFalse = 0x0E,
    /// Call a primitive method (index into primitive table, arg count)
    CallPrimitive = 0x0F,
    /// End of bytecode
    End = 0xFF,
}

/// Instruction encoding
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Opcode for the instruction
    pub opcode: Opcode,
    /// First operand (meaning depends on opcode)
    pub operand1: u16,
    /// Second operand (meaning depends on opcode)
    pub operand2: u16,
}

impl Instruction {
    /// Create a new instruction
    pub fn new(opcode: Opcode, operand1: u16, operand2: u16) -> Self {
        Self { opcode, operand1, operand2 }
    }
    
    /// Create a new instruction with only one operand
    pub fn with_operand1(opcode: Opcode, operand1: u16) -> Self {
        Self { opcode, operand1, operand2: 0 }
    }
    
    /// Create a new instruction with no operands
    pub fn simple(opcode: Opcode) -> Self {
        Self { opcode, operand1: 0, operand2: 0 }
    }
}

/// Bytecode representation of an Io method
#[derive(Debug, Clone)]
pub struct Bytecode {
    /// Instructions in the bytecode
    pub instructions: Vec<Instruction>,
    /// Constant pool for the bytecode (literals and names)
    pub constant_pool: Vec<Value>,
    /// Number of local variables needed for execution
    pub local_count: u16,
}

impl Bytecode {
    /// Create a new empty bytecode
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constant_pool: Vec::new(),
            local_count: 0,
        }
    }
    
    /// Generate bytecode from a message chain
    pub fn from_message_chain(chain: &MessageChain, method_data: &MethodData) -> Self {
        let mut compiler = BytecodeCompiler::new(method_data);
        compiler.compile_chain(chain);
        compiler.finalize()
    }
    
    /// Serialize bytecode to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Write magic number to identify bytecode format
        bytes.extend_from_slice(&[b'I', b'O', b'B', b'C']);
        
        // Write format version
        bytes.push(1); // Version 1
        
        // Write local count
        bytes.extend_from_slice(&self.local_count.to_le_bytes());
        
        // Write constant pool size
        let const_pool_len = self.constant_pool.len() as u16;
        bytes.extend_from_slice(&const_pool_len.to_le_bytes());
        
        // Write constant pool (simplified for now)
        for constant in &self.constant_pool {
            match constant {
                Value::Nil => {
                    bytes.push(0); // Nil type tag
                }
                Value::Boolean(b) => {
                    bytes.push(1); // Boolean type tag
                    bytes.push(if *b { 1 } else { 0 });
                }
                Value::Number(n) => {
                    bytes.push(2); // Number type tag
                    bytes.extend_from_slice(&n.to_le_bytes());
                }
                Value::String(s) => {
                    bytes.push(3); // String type tag
                    let str_len = s.len() as u16;
                    bytes.extend_from_slice(&str_len.to_le_bytes());
                    bytes.extend_from_slice(s.as_bytes());
                }
                Value::Object(_) => {
                    bytes.push(4); // Object type tag (not serializable directly)
                    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // 8 bytes of zeros
                }
            }
        }
        
        // Write instruction count
        let instr_count = self.instructions.len() as u32;
        bytes.extend_from_slice(&instr_count.to_le_bytes());
        
        // Write instructions
        for instruction in &self.instructions {
            bytes.push(instruction.opcode as u8);
            bytes.extend_from_slice(&instruction.operand1.to_le_bytes());
            bytes.extend_from_slice(&instruction.operand2.to_le_bytes());
        }
        
        bytes
    }
    
    /// Deserialize bytecode from bytes
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None; // Too short
        }
        
        // Check magic number
        if bytes[0..4] != [b'I', b'O', b'B', b'C'] {
            return None; // Invalid format
        }
        
        // Check version
        if bytes[4] != 1 {
            return None; // Unsupported version
        }
        
        let mut offset = 5;
        
        // Read local count
        if offset + 2 > bytes.len() {
            return None;
        }
        let local_count = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        
        // Read constant pool size
        if offset + 2 > bytes.len() {
            return None;
        }
        let const_pool_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;
        
        // Read constant pool
        let mut constant_pool = Vec::with_capacity(const_pool_len);
        for _ in 0..const_pool_len {
            if offset >= bytes.len() {
                return None;
            }
            
            let tag = bytes[offset];
            offset += 1;
            
            match tag {
                0 => {
                    // Nil
                    constant_pool.push(Value::Nil);
                }
                1 => {
                    // Boolean
                    if offset >= bytes.len() {
                        return None;
                    }
                    let value = bytes[offset] != 0;
                    offset += 1;
                    constant_pool.push(Value::Boolean(value));
                }
                2 => {
                    // Number
                    if offset + 8 > bytes.len() {
                        return None;
                    }
                    let value = f64::from_le_bytes([
                        bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3],
                        bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7],
                    ]);
                    offset += 8;
                    constant_pool.push(Value::Number(value));
                }
                3 => {
                    // String
                    if offset + 2 > bytes.len() {
                        return None;
                    }
                    let str_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
                    offset += 2;
                    
                    if offset + str_len > bytes.len() {
                        return None;
                    }
                    
                    let str_bytes = &bytes[offset..offset + str_len];
                    match std::str::from_utf8(str_bytes) {
                        Ok(s) => {
                            constant_pool.push(Value::String(s.to_string()));
                        }
                        Err(_) => {
                            return None; // Invalid UTF-8
                        }
                    }
                    offset += str_len;
                }
                4 => {
                    // Object (not serializable directly)
                    offset += 8; // Skip 8 bytes
                    constant_pool.push(Value::Nil); // Use nil as placeholder
                }
                _ => {
                    return None; // Invalid tag
                }
            }
        }
        
        // Read instruction count
        if offset + 4 > bytes.len() {
            return None;
        }
        let instr_count = u32::from_le_bytes([
            bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3],
        ]) as usize;
        offset += 4;
        
        // Read instructions
        let mut instructions = Vec::with_capacity(instr_count);
        for _ in 0..instr_count {
            if offset + 5 > bytes.len() {
                return None;
            }
            
            let opcode_byte = bytes[offset];
            offset += 1;
            
            // Map byte to opcode
            let opcode = match opcode_byte {
                0x01 => Opcode::LoadConst,
                0x02 => Opcode::LoadLocal,
                0x03 => Opcode::StoreLocal,
                0x04 => Opcode::GetSlot,
                0x05 => Opcode::SetSlot,
                0x06 => Opcode::SendMessage,
                0x07 => Opcode::Return,
                0x08 => Opcode::Jump,
                0x09 => Opcode::JumpIfFalse,
                0x0A => Opcode::JumpIfTrue,
                0x0B => Opcode::PushNil,
                0x0C => Opcode::PushSelf,
                0x0D => Opcode::PushTrue,
                0x0E => Opcode::PushFalse,
                0x0F => Opcode::CallPrimitive,
                0xFF => Opcode::End,
                _ => return None, // Invalid opcode
            };
            
            let operand1 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;
            
            let operand2 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;
            
            instructions.push(Instruction { opcode, operand1, operand2 });
        }
        
        Some(Self {
            instructions,
            constant_pool,
            local_count,
        })
    }
}

/// Compiler to generate bytecode from Io AST
struct BytecodeCompiler<'a> {
    /// Generated bytecode
    bytecode: Bytecode,
    /// Method data for parameter information
    method_data: &'a MethodData,
    /// Jump targets for labels
    jump_targets: Vec<(String, usize)>,
    /// Pending jumps to resolve
    pending_jumps: Vec<(String, usize)>,
    /// Current local variable index
    next_local: u16,
    /// Map from local name to index
    local_indices: std::collections::HashMap<String, u16>,
}

impl<'a> BytecodeCompiler<'a> {
    /// Create a new bytecode compiler
    fn new(method_data: &'a MethodData) -> Self {
        let mut compiler = Self {
            bytecode: Bytecode::new(),
            method_data,
            jump_targets: Vec::new(),
            pending_jumps: Vec::new(),
            next_local: 0,
            local_indices: std::collections::HashMap::new(),
        };
        
        // Set up local variables for method arguments
        for (i, arg) in method_data.args.iter().enumerate() {
            compiler.local_indices.insert(arg.clone(), i as u16);
            compiler.next_local = compiler.next_local.max(i as u16 + 1);
        }
        
        compiler
    }
    
    /// Compile a message chain to bytecode
    fn compile_chain(&mut self, chain: &MessageChain) {
        for message in &chain.messages {
            self.compile_message(message);
        }
        
        // If the last instruction is not a Return or Jump, ensure we have a value on the stack
        if !self.bytecode.instructions.is_empty() {
            let last_instr = &self.bytecode.instructions[self.bytecode.instructions.len() - 1];
            if last_instr.opcode != Opcode::Return && last_instr.opcode != Opcode::Jump {
                // Push nil as default return value if nothing else was returned
                self.emit(Instruction::simple(Opcode::PushNil));
            }
        }
    }
    
    /// Compile a single message to bytecode
    fn compile_message(&mut self, message: &Message) {
        match &message.symbol {
            Symbol::Identifier(id) => {
                let name = id.name();
                
                // Handle control flow instructions
                if name == "return" {
                    // IMPORTANT: For "return", don't treat it as a message send
                    // Instead, evaluate the argument and emit a direct return instruction
                    if !message.args.is_empty() && !message.args[0].chains.is_empty() {
                        // Compile the return expression directly
                        self.compile_chain(&message.args[0].chains[0]);
                    } else {
                        // Return nil if no argument provided
                        self.emit(Instruction::simple(Opcode::PushNil));
                    }
                    
                    // Debug print the stack at return point
                    println!("Emitting direct return instruction");
                    
                    // Emit return instruction (not a message send)
                    self.emit(Instruction::simple(Opcode::Return));
                    return;
                } else if name == "if" {
                    self.compile_if_statement(message);
                    return;
                }
                
                // Handle regular identifier
                if let Some(local_idx) = self.local_indices.get(name) {
                    // Load local variable
                    self.emit(Instruction::with_operand1(Opcode::LoadLocal, *local_idx));
                } else {
                    // Push receiver
                    self.emit(Instruction::simple(Opcode::PushSelf));
                    
                    // Add name to constant pool
                    let name_idx = self.add_constant(Value::String(name.to_string()));
                    
                    // Compile arguments
                    let arg_count = self.compile_arguments(message);
                    
                    // Send message
                    self.emit(Instruction::new(Opcode::SendMessage, name_idx, arg_count));
                }
            },
            Symbol::Number(n) => {
                // Load number constant
                match n {
                    iowa_parser::Number::Decimal(value) => {
                        let const_idx = self.add_constant(Value::Number(*value));
                        self.emit(Instruction::with_operand1(Opcode::LoadConst, const_idx));
                    },
                    iowa_parser::Number::Hex(value) => {
                        let const_idx = self.add_constant(Value::Number(*value as f64));
                        self.emit(Instruction::with_operand1(Opcode::LoadConst, const_idx));
                    },
                }
            },
            Symbol::Quote(q) => {
                // Load string constant
                let const_idx = self.add_constant(Value::String(q.content().to_string()));
                self.emit(Instruction::with_operand1(Opcode::LoadConst, const_idx));
            },
            Symbol::Operator(op) => {
                let symbol = op.symbol();
                
                // Handle special operators
                if symbol == ":=" {
                    self.compile_assignment(message);
                    return;
                }
                
                // For other operators, treat as a message send to the receiver
                // First compile arguments
                let arg_count = self.compile_arguments(message);
                
                // Add operator name to constant pool
                let name_idx = self.add_constant(Value::String(symbol.to_string()));
                
                // Send message
                self.emit(Instruction::new(Opcode::SendMessage, name_idx, arg_count));
            },
        }
    }
    
    /// Compile arguments for a message
    fn compile_arguments(&mut self, message: &Message) -> u16 {
        let mut arg_count = 0;
        
        for arg in &message.args {
            for chain in &arg.chains {
                self.compile_chain(chain);
                arg_count += 1;
            }
        }
        
        arg_count
    }
    
    /// Compile an if statement
    fn compile_if_statement(&mut self, message: &Message) {
        if message.args.len() < 2 {
            // If statement requires at least a condition and a then branch
            self.emit(Instruction::simple(Opcode::PushNil));
            return;
        }
        
        // Compile condition
        if !message.args[0].chains.is_empty() {
            self.compile_chain(&message.args[0].chains[0]);
        } else {
            self.emit(Instruction::simple(Opcode::PushNil));
        }
        
        // Emit jump if false
        let else_label = format!("else_{}", self.bytecode.instructions.len());
        let end_label = format!("end_if_{}", self.bytecode.instructions.len());
        
        self.emit_jump_if_false(else_label.clone());
        
        // Compile then branch
        if !message.args[1].chains.is_empty() {
            self.compile_chain(&message.args[1].chains[0]);
        } else {
            self.emit(Instruction::simple(Opcode::PushNil));
        }
        
        // Jump to end
        self.emit_jump(end_label.clone());
        
        // Emit else branch if it exists
        self.define_label(else_label);
        
        if message.args.len() >= 3 && !message.args[2].chains.is_empty() {
            self.compile_chain(&message.args[2].chains[0]);
        } else {
            self.emit(Instruction::simple(Opcode::PushNil));
        }
        
        // Define end label
        self.define_label(end_label);
    }
    
    /// Compile an assignment statement
    fn compile_assignment(&mut self, message: &Message) {
        // Get variable name
        let var_name = match &message.symbol {
            Symbol::Identifier(id) => id.name(),
            _ => return, // Should be an identifier
        };
        
        // Compile right side value
        if !message.args.is_empty() && !message.args[0].chains.is_empty() {
            self.compile_chain(&message.args[0].chains[0]);
        } else {
            self.emit(Instruction::simple(Opcode::PushNil));
        }
        
        // Check if variable exists in locals
        if let Some(local_idx) = self.local_indices.get(var_name) {
            // Store to existing local
            self.emit(Instruction::with_operand1(Opcode::StoreLocal, *local_idx));
        } else {
            // Create new local
            let local_idx = self.next_local;
            self.next_local += 1;
            self.local_indices.insert(var_name.to_string(), local_idx);
            
            // Store to new local
            self.emit(Instruction::with_operand1(Opcode::StoreLocal, local_idx));
        }
    }
    
    /// Emit an instruction
    fn emit(&mut self, instruction: Instruction) {
        self.bytecode.instructions.push(instruction);
    }
    
    /// Add a constant to the constant pool
    fn add_constant(&mut self, value: Value) -> u16 {
        // Check if constant already exists
        for (i, existing) in self.bytecode.constant_pool.iter().enumerate() {
            if *existing == value {
                return i as u16;
            }
        }
        
        // Add new constant
        let idx = self.bytecode.constant_pool.len();
        self.bytecode.constant_pool.push(value);
        idx as u16
    }
    
    /// Emit a jump instruction
    fn emit_jump(&mut self, label: String) {
        let jump_idx = self.bytecode.instructions.len();
        self.emit(Instruction::simple(Opcode::Jump));
        self.pending_jumps.push((label, jump_idx));
    }
    
    /// Emit a jump-if-false instruction
    fn emit_jump_if_false(&mut self, label: String) {
        let jump_idx = self.bytecode.instructions.len();
        self.emit(Instruction::simple(Opcode::JumpIfFalse));
        self.pending_jumps.push((label, jump_idx));
    }
    
    /// Define a label at the current position
    fn define_label(&mut self, label: String) {
        let pos = self.bytecode.instructions.len();
        self.jump_targets.push((label, pos));
    }
    
    /// Resolve all pending jumps
    fn resolve_jumps(&mut self) {
        for (label, jump_idx) in &self.pending_jumps {
            // Find the target for this label
            let target_idx = self.jump_targets.iter()
                .find(|(l, _)| l == label)
                .map(|(_, pos)| *pos)
                .unwrap_or_else(|| panic!("Undefined label: {}", label));
            
            // Calculate the offset
            let offset = target_idx as isize - *jump_idx as isize - 1;
            
            // Update the instruction's operand
            self.bytecode.instructions[*jump_idx].operand1 = (offset as i16) as u16;
        }
    }
    
    /// Finalize the bytecode
    fn finalize(mut self) -> Bytecode {
        // Resolve all jumps
        self.resolve_jumps();
        
        // Add end instruction
        self.emit(Instruction::simple(Opcode::End));
        
        // Set local count
        self.bytecode.local_count = self.next_local;
        
        self.bytecode
    }
}

/// Bytecode virtual machine that executes bytecode instructions
pub struct BytecodeVM {
    /// Runtime instance
    runtime: *mut Runtime,
    /// Stack for values
    stack: Vec<Value>,
    /// Local variables
    locals: Vec<Value>,
    /// Program counter
    pc: usize,
    /// Current bytecode
    bytecode: Option<Bytecode>,
}

impl BytecodeVM {
    /// Create a new bytecode VM
    pub fn new(runtime: *mut Runtime) -> Self {
        Self {
            runtime,
            stack: Vec::with_capacity(64),
            locals: Vec::new(),
            pc: 0,
            bytecode: None,
        }
    }
    
    /// Execute a bytecode method
    pub unsafe fn execute(&mut self, bytecode: &Bytecode, args: &[Value], receiver: Value) -> Value {
        // Debug bytecode details
        println!("Bytecode instructions:");
        for (i, instr) in bytecode.instructions.iter().enumerate() {
            println!("  {}: {:?} op1={} op2={}", i, instr.opcode, instr.operand1, instr.operand2);
        }
        
        println!("Constant pool:");
        for (i, val) in bytecode.constant_pool.iter().enumerate() {
            println!("  {}: {:?}", i, val);
        }
        
        // Set up the VM for execution
        self.bytecode = Some(bytecode.clone());
        self.pc = 0;
        self.stack.clear();
        
        // Resize locals array
        self.locals.resize(bytecode.local_count as usize, Value::Nil);
        
        // Initialize locals with arguments
        for (i, arg) in args.iter().enumerate() {
            if i < self.locals.len() {
                self.locals[i] = arg.clone();
            }
        }
        
        // Execute instructions until return or end
        loop {
            if self.pc >= bytecode.instructions.len() {
                break;
            }
            
            let instruction = &bytecode.instructions[self.pc];
            
            // println!("Executing instruction: {:?} at pc={}", instruction.opcode, self.pc);
            match instruction.opcode {
                Opcode::LoadConst => {
                    let const_idx = instruction.operand1 as usize;
                    if const_idx < bytecode.constant_pool.len() {
                        let value = bytecode.constant_pool[const_idx].clone();
                        println!("  LoadConst: idx={}, value={:?}", const_idx, value);
                        self.stack.push(value);
                    } else {
                        println!("  LoadConst: idx={} (out of bounds)", const_idx);
                        self.stack.push(Value::Nil);
                    }
                    self.pc += 1;
                },
                Opcode::LoadLocal => {
                    let local_idx = instruction.operand1 as usize;
                    if local_idx < self.locals.len() {
                        self.stack.push(self.locals[local_idx].clone());
                    } else {
                        self.stack.push(Value::Nil);
                    }
                    self.pc += 1;
                },
                Opcode::StoreLocal => {
                    let local_idx = instruction.operand1 as usize;
                    if !self.stack.is_empty() && local_idx < self.locals.len() {
                        // Clone the value so it stays on the stack
                        let value = self.stack.last().unwrap().clone();
                        self.locals[local_idx] = value;
                    }
                    self.pc += 1;
                },
                Opcode::GetSlot => {
                    let name_idx = instruction.operand1 as usize;
                    if !self.stack.is_empty() && name_idx < bytecode.constant_pool.len() {
                        let obj = self.stack.pop().unwrap();
                        let name = match &bytecode.constant_pool[name_idx] {
                            Value::String(s) => s.as_str(),
                            _ => "",
                        };
                        
                        // Unsafe is required for raw pointer dereferencing
                        let result = unsafe {
                            let runtime = &mut *self.runtime;
                            runtime.dispatch_message(obj, name, vec![])
                        };
                        self.stack.push(result);
                    } else {
                        self.stack.push(Value::Nil);
                    }
                    self.pc += 1;
                },
                Opcode::SetSlot => {
                    // TODO: Implement
                    self.pc += 1;
                },
                Opcode::SendMessage => {
                    let name_idx = instruction.operand1 as usize;
                    let arg_count = instruction.operand2 as usize;
                    
                    // println!("  SendMessage: name_idx={}, arg_count={}, stack_size={}", 
                    //          name_idx, arg_count, self.stack.len());
                    
                    if name_idx < bytecode.constant_pool.len() && self.stack.len() > arg_count {
                        // Get message name
                        let name = match &bytecode.constant_pool[name_idx] {
                            Value::String(s) => s.clone(),
                            _ => "".to_string(),
                        };
                        
                        // println!("  Message name: {}", name);
                        
                        // Pop arguments in reverse order
                        let mut args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            args.push(self.stack.pop().unwrap());
                        }
                        args.reverse();
                        
                        // println!("  Arguments: {:?}", args);
                        
                        // Pop receiver
                        let receiver = self.stack.pop().unwrap();
                        // println!("  Receiver: {:?}", receiver);
                        
                        // Send message
                        // Unsafe is required for raw pointer dereferencing
                        let result = unsafe {
                            let runtime = &mut *self.runtime;
                            runtime.dispatch_message(receiver, &name, args)
                        };
                        
                        // println!("  Result: {:?}", result);
                        
                        // Push result
                        self.stack.push(result);
                    } else {
                        // println!("  Unable to send message (invalid indices or not enough stack items)");
                        self.stack.push(Value::Nil);
                    }
                    self.pc += 1;
                },
                Opcode::Return => {
                    // If stack is empty, return nil
                    if self.stack.is_empty() {
                        println!("  Return: stack empty, returning nil");
                        return Value::Nil;
                    }
                    
                    // Return the top value
                    let result = self.stack.pop().unwrap();
                    println!("  Return: returning value {:?}", result);
                    return result;
                },
                Opcode::Jump => {
                    let offset = instruction.operand1 as i16;
                    // println!("Jump: offset={}, current_pc={}", offset, self.pc);
                    let new_pc = (self.pc as isize + offset as isize) as usize;
                    // println!("  Jumping to pc={}", new_pc);
                    self.pc = new_pc;
                },
                Opcode::JumpIfFalse => {
                    let offset = instruction.operand1 as i16;
                    let is_false = if let Some(value) = self.stack.last() {
                        match value {
                            Value::Nil => true,
                            Value::Boolean(b) => !*b,
                            _ => false,
                        }
                    } else {
                        true
                    };
                    
                    // println!("JumpIfFalse: value={:?}, is_false={}, offset={}, current_pc={}", 
                    //          self.stack.last(), is_false, offset, self.pc);
                    
                    if is_false {
                        let new_pc = (self.pc as isize + offset as isize) as usize;
                        // println!("  Jumping to pc={}", new_pc);
                        self.pc = new_pc;
                    } else {
                        // println!("  Not jumping, moving to pc={}", self.pc + 1);
                        self.pc += 1;
                    }
                },
                Opcode::JumpIfTrue => {
                    let offset = instruction.operand1 as i16;
                    let is_true = if let Some(value) = self.stack.last() {
                        match value {
                            Value::Nil => false,
                            Value::Boolean(b) => *b,
                            _ => true,
                        }
                    } else {
                        false
                    };
                    
                    if is_true {
                        self.pc = (self.pc as isize + offset as isize) as usize;
                    } else {
                        self.pc += 1;
                    }
                },
                Opcode::PushNil => {
                    self.stack.push(Value::Nil);
                    self.pc += 1;
                },
                Opcode::PushSelf => {
                    self.stack.push(receiver.clone());
                    self.pc += 1;
                },
                Opcode::PushTrue => {
                    self.stack.push(Value::Boolean(true));
                    self.pc += 1;
                },
                Opcode::PushFalse => {
                    self.stack.push(Value::Boolean(false));
                    self.pc += 1;
                },
                Opcode::CallPrimitive => {
                    // TODO: Implement primitive method calls
                    self.pc += 1;
                },
                Opcode::End => {
                    // Return nil if stack is empty, otherwise return top value
                    if self.stack.is_empty() {
                        println!("  End: stack empty, returning nil");
                        return Value::Nil;
                    } else {
                        let result = self.stack.pop().unwrap();
                        println!("  End: returning value {:?}", result);
                        return result;
                    }
                },
            }
        }
        
        // Default return value if we reach the end
        if self.stack.is_empty() {
            Value::Nil
        } else {
            self.stack.pop().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    
    #[test]
    fn test_simple_bytecode() {
        // Create bytecode that pushes a number and returns it
        let mut bytecode = Bytecode::new();
        
        // Add constant
        bytecode.constant_pool.push(Value::Number(42.0));
        
        // Add instructions
        bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
        bytecode.instructions.push(Instruction::simple(Opcode::Return));
        
        // Create runtime
        let runtime = Arc::new(Mutex::new(Runtime::new()));
        let runtime_ptr = runtime.clone();
        
        // Execute bytecode
        let result = {
            let mut rt = runtime.lock().unwrap();
            let mut vm = BytecodeVM::new(&mut *rt);
            unsafe { vm.execute(&bytecode, &[], Value::Nil) }
        };
        
        // Check result
        assert_eq!(result, Value::Number(42.0));
    }
}