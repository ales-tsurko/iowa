//! Runtime implementation for the Io language.
//!
//! This module provides the actual implementation of runtime functions
//! for executing Io code and managing its objects, memory, and garbage collection.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{memory, value};

/// Global object ID counter
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Runtime state for an Io execution context
pub struct Runtime {
    /// Memory manager for runtime objects
    memory: Memory,
    /// The Lobby object (root context)
    lobby: ObjectRef,
    /// Core prototype objects
    prototypes: Prototypes,
    /// Current call frame
    current_frame: Option<CallFrame>,
}

impl Runtime {
    /// Create a new runtime instance
    pub fn new() -> Self {
        let mut runtime = Self {
            memory: Memory::new(),
            lobby: 0,
            prototypes: Prototypes::default(),
            current_frame: None,
        };
        
        runtime.initialize();
        runtime
    }
    
    /// Get a reference to the memory manager
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
    
    /// Get a mutable reference to the memory manager
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }
    
    /// Get the Lobby object reference
    pub fn lobby(&self) -> ObjectRef {
        self.lobby
    }
    
    /// Get the prototypes
    pub fn prototypes(&self) -> &Prototypes {
        &self.prototypes
    }
    
    /// Get the current call frame
    pub fn current_frame(&self) -> &Option<CallFrame> {
        &self.current_frame
    }
    
    /// Initialize the runtime
    fn initialize(&mut self) {
        // Create memory manager
        self.memory = Memory::new();
        
        // Create core prototypes
        self.prototypes.object = self.create_object_prototype();
        self.prototypes.number = self.create_number_prototype();
        self.prototypes.string = self.create_string_prototype();
        self.prototypes.list = self.create_list_prototype();
        self.prototypes.map = self.create_map_prototype();
        
        // Create Lobby object
        self.lobby = self.create_lobby();
    }
    
    /// Create the Object prototype
    fn create_object_prototype(&mut self) -> ObjectRef {
        // Object is the root object with no prototype
        let obj_id = self.memory.alloc_object(0);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Set its own prototype to itself (completes the chain)
        obj.prototype = obj_id;
        
        // Add type slot
        obj.set_slot("type".to_string(), Value::String("Object".to_string()));
        
        obj_id
    }
    
    /// Create the Number prototype
    fn create_number_prototype(&mut self) -> ObjectRef {
        let obj_id = self.memory.alloc_object(self.prototypes.object);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Add type slot
        obj.set_slot("type".to_string(), Value::String("Number".to_string()));
        
        // Add primitive methods
        self.add_number_methods(obj_id);
        
        obj_id
    }
    
    /// Create the String prototype
    fn create_string_prototype(&mut self) -> ObjectRef {
        let obj_id = self.memory.alloc_object(self.prototypes.object);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Add type slot
        obj.set_slot("type".to_string(), Value::String("String".to_string()));
        
        // Add primitive methods
        self.add_string_methods(obj_id);
        
        obj_id
    }
    
    /// Create the List prototype
    fn create_list_prototype(&mut self) -> ObjectRef {
        let obj_id = self.memory.alloc_object(self.prototypes.object);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Add type slot
        obj.set_slot("type".to_string(), Value::String("List".to_string()));
        
        // Add primitive methods
        self.add_list_methods(obj_id);
        
        obj_id
    }
    
    /// Create the Map prototype
    fn create_map_prototype(&mut self) -> ObjectRef {
        let obj_id = self.memory.alloc_object(self.prototypes.object);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Add type slot
        obj.set_slot("type".to_string(), Value::String("Map".to_string()));
        
        // Add primitive methods
        self.add_map_methods(obj_id);
        
        obj_id
    }
    
    /// Create the Lobby object
    fn create_lobby(&mut self) -> ObjectRef {
        let obj_id = self.memory.alloc_object(self.prototypes.object);
        
        // Get mutable reference to configure
        let obj = self.memory.get_object_mut(obj_id).unwrap();
        
        // Add prototype objects as slots
        obj.set_slot("Object".to_string(), Value::Object(self.prototypes.object));
        obj.set_slot("Number".to_string(), Value::Object(self.prototypes.number));
        obj.set_slot("String".to_string(), Value::Object(self.prototypes.string));
        obj.set_slot("List".to_string(), Value::Object(self.prototypes.list));
        obj.set_slot("Map".to_string(), Value::Object(self.prototypes.map));
        
        obj_id
    }
    
    /// Add primitive methods to Number prototype
    fn add_number_methods(&mut self, proto_id: ObjectRef) {
        // Create primitive methods first
        let add_method = self.memory.alloc_method(MethodType::Primitive(1));
        let sub_method = self.memory.alloc_method(MethodType::Primitive(2));
        let mul_method = self.memory.alloc_method(MethodType::Primitive(3));
        let div_method = self.memory.alloc_method(MethodType::Primitive(4));
        
        // Then add them to the object
        if let Some(obj) = self.memory.get_object_mut(proto_id) {
            // Add methods as slots
            obj.set_slot("+".to_string(), Value::Object(add_method));
            obj.set_slot("-".to_string(), Value::Object(sub_method));
            obj.set_slot("*".to_string(), Value::Object(mul_method));
            obj.set_slot("/".to_string(), Value::Object(div_method));
        }
    }
    
    /// Add primitive methods to String prototype
    fn add_string_methods(&mut self, proto_id: ObjectRef) {
        // Create primitive methods first
        let size_method = self.memory.alloc_method(MethodType::Primitive(10));
        let at_method = self.memory.alloc_method(MethodType::Primitive(11));
        let slice_method = self.memory.alloc_method(MethodType::Primitive(12));
        
        // Then add them to the object
        if let Some(obj) = self.memory.get_object_mut(proto_id) {
            // Add methods as slots
            obj.set_slot("size".to_string(), Value::Object(size_method));
            obj.set_slot("at".to_string(), Value::Object(at_method));
            obj.set_slot("slice".to_string(), Value::Object(slice_method));
        }
    }
    
    /// Add primitive methods to List prototype
    fn add_list_methods(&mut self, proto_id: ObjectRef) {
        // Create primitive methods first
        let size_method = self.memory.alloc_method(MethodType::Primitive(20));
        let at_method = self.memory.alloc_method(MethodType::Primitive(21));
        let append_method = self.memory.alloc_method(MethodType::Primitive(22));
        
        // Then add them to the object
        if let Some(obj) = self.memory.get_object_mut(proto_id) {
            // Add methods as slots
            obj.set_slot("size".to_string(), Value::Object(size_method));
            obj.set_slot("at".to_string(), Value::Object(at_method));
            obj.set_slot("append".to_string(), Value::Object(append_method));
        }
    }
    
    /// Add primitive methods to Map prototype
    fn add_map_methods(&mut self, proto_id: ObjectRef) {
        // Create primitive methods first
        let size_method = self.memory.alloc_method(MethodType::Primitive(30));
        let at_method = self.memory.alloc_method(MethodType::Primitive(31));
        let at_put_method = self.memory.alloc_method(MethodType::Primitive(32));
        
        // Then add them to the object
        if let Some(obj) = self.memory.get_object_mut(proto_id) {
            // Add methods as slots
            obj.set_slot("size".to_string(), Value::Object(size_method));
            obj.set_slot("at".to_string(), Value::Object(at_method));
            obj.set_slot("atPut".to_string(), Value::Object(at_put_method));
        }
    }
    
    /// Push a new call frame onto the call stack
    pub fn push_call_frame(
        &mut self,
        method: ObjectRef,
        receiver: ObjectRef,
        message: String,
        args: Vec<Value>,
    ) {
        // Create new frame with current as parent
        let parent = self.current_frame.take().map(Box::new);
        let new_frame = CallFrame::new(method, receiver, message, args, parent);
        
        // Set as current frame
        self.current_frame = Some(new_frame);
    }
    
    /// Pop the current call frame
    pub fn pop_call_frame(&mut self) -> Option<CallFrame> {
        if let Some(mut frame) = self.current_frame.take() {
            // Extract the parent frame
            let parent = frame.parent.take();
            
            // Restore parent frame as current
            self.current_frame = parent.map(|boxed| *boxed);
            
            Some(frame)
        } else {
            None
        }
    }
    
    /// Dispatch a message to an object
    pub fn dispatch_message(&mut self, receiver: Value, message_name: &str, args: Vec<Value>) -> Value {
        println!("Dispatching message: {} to {:?}", message_name, receiver);
        // Get receiver object
        let receiver_id = match receiver {
            Value::Object(id) => id,
            _ => {
                // For non-object receivers, try to coerce to appropriate type
                return self.handle_primitive_message(receiver, message_name, args);
            }
        };
        
        // Look up the slot in the receiver's prototype chain
        println!("Looking up slot '{}' in receiver {}", message_name, receiver_id);
        let method = match self.memory.get_object(receiver_id) {
            Some(obj) => {
                println!("Found receiver object with type: {}", obj.type_name());
                let slot_value = obj.lookup_slot(message_name, self);
                println!("Slot lookup result: {:?}", slot_value);
                slot_value
            },
            None => {
                println!("Receiver object not found");
                Value::Nil
            },
        };
        
        // If no method found, try forward
        let method = match method {
            Value::Nil => {
                // Try forward
                if let Some(obj) = self.memory.get_object(receiver_id) {
                    let forward = obj.lookup_slot("forward", self);
                    if let Value::Object(_) = forward {
                        // Create arguments with the message name
                        let mut forward_args = vec![Value::String(message_name.to_string())];
                        forward_args.extend(args);
                        
                        // Call the forward method
                        return self.dispatch_message(Value::Object(receiver_id), "forward", forward_args);
                    }
                }
                return Value::Nil;
            }
            m => m,
        };
        
        // Call the method
        self.call_method(method, receiver, message_name, args)
    }
    
    /// Handle primitive message for non-object receivers
    fn handle_primitive_message(&mut self, receiver: Value, message_name: &str, args: Vec<Value>) -> Value {
        match receiver {
            Value::Number(n) => {
                // Handle number operations
                match message_name {
                    "+" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Number(n + other)
                        } else {
                            Value::Nil
                        }
                    }
                    "-" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Number(n - other)
                        } else {
                            Value::Nil
                        }
                    }
                    "*" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Number(n * other)
                        } else {
                            Value::Nil
                        }
                    }
                    "/" => {
                        if let Some(Value::Number(other)) = args.first() {
                            if *other == 0.0 {
                                Value::Nil
                            } else {
                                Value::Number(n / other)
                            }
                        } else {
                            Value::Nil
                        }
                    }
                    "asString" => Value::String(n.to_string()),
                    "<" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n < *other)
                        } else {
                            Value::Nil
                        }
                    },
                    ">" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n > *other)
                        } else {
                            Value::Nil
                        }
                    },
                    ">=" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n >= *other)
                        } else {
                            Value::Nil
                        }
                    },
                    "<=" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n <= *other)
                        } else {
                            Value::Nil
                        }
                    },
                    "==" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n == *other)
                        } else {
                            Value::Nil
                        }
                    },
                    "!=" => {
                        if let Some(Value::Number(other)) = args.first() {
                            Value::Boolean(n != *other)
                        } else {
                            Value::Nil
                        }
                    },
                    _ => Value::Nil,
                }
            }
            Value::String(s) => {
                // Handle string operations
                match message_name {
                    "size" => Value::Number(s.len() as f64),
                    "at" => {
                        if let Some(Value::Number(idx)) = args.first() {
                            let idx = *idx as usize;
                            if idx < s.len() {
                                if let Some(c) = s.chars().nth(idx) {
                                    Value::String(c.to_string())
                                } else {
                                    Value::Nil
                                }
                            } else {
                                Value::Nil
                            }
                        } else {
                            Value::Nil
                        }
                    }
                    "slice" => {
                        if args.len() >= 2 {
                            if let (Some(Value::Number(start)), Some(Value::Number(end))) = (args.first(), args.get(1)) {
                                let start = *start as usize;
                                let end = *end as usize;
                                if start <= end && end <= s.len() {
                                    Value::String(s[start..end].to_string())
                                } else {
                                    Value::Nil
                                }
                            } else {
                                Value::Nil
                            }
                        } else {
                            Value::Nil
                        }
                    }
                    "asString" => Value::String(s.clone()),
                    _ => Value::Nil,
                }
            }
            _ => {
                // Handle other types
                match message_name {
                    "asString" => {
                        match receiver {
                            Value::Nil => Value::String("nil".to_string()),
                            Value::Boolean(b) => Value::String(b.to_string()),
                            _ => Value::String("unknown".to_string()),
                        }
                    }
                    _ => Value::Nil,
                }
            }
        }
    }
    
    /// Call a method
    fn call_method(&mut self, method: Value, receiver: Value, message_name: &str, args: Vec<Value>) -> Value {
        println!("Calling method: method={:?}, receiver={:?}, message={}", method, receiver, message_name);
        // Get method object ID
        let method_id = match method {
            Value::Object(id) => id,
            _ => return Value::Nil, // Not callable
        };
        
        // Get receiver ID
        let receiver_id = match receiver {
            Value::Object(id) => id,
            _ => {
                // Box primitive value into an object
                match receiver.clone() {
                    Value::Number(n) => self.memory.alloc_number(Value::Number(n)),
                    Value::String(s) => self.memory.alloc_string(Value::String(s.clone())),
                    Value::Boolean(b) => {
                        // Create a boolean object
                        let id = self.memory.alloc_object(self.prototypes.object);
                        if let Some(obj) = self.memory.get_object_mut(id) {
                            obj.set_slot("value".to_string(), Value::Boolean(b));
                        }
                        id
                    }
                    _ => {
                        // Create a nil object
                        let id = self.memory.alloc_object(self.prototypes.object);
                        if let Some(obj) = self.memory.get_object_mut(id) {
                            obj.set_slot("value".to_string(), Value::Nil);
                        }
                        id
                    }
                }
            }
        };
        
        // Need to break up the borrow patterns to avoid conflicts
        // First check if the method is valid and get its type
        println!("Looking up method ID: {}", method_id);
        let method_type = match self.memory.get_object(method_id) {
            Some(obj) => {
                println!("Found method object with data type: {:?}", obj.data);
                match &obj.data {
                    ObjectData::Method(method_type) => {
                        println!("Method type: {:?}", method_type);
                        Some(method_type.clone())
                    },
                    _ => {
                        println!("Not a method object");
                        None // Not a method
                    }
                }
            }
            None => {
                println!("Method object not found");
                None
            },
        };
        
        // Then process based on the method type
        match method_type {
            Some(MethodType::Primitive(index)) => {
                // Call primitive method
                self.call_primitive_method(index, receiver_id, args)
            }
            Some(MethodType::UserDefined(method_data)) => {
                // Push call frame
                self.push_call_frame(method_id, receiver_id, message_name.to_string(), args.clone());
                
                // Set up arguments in locals
                if let Some(frame) = &mut self.current_frame {
                    for (i, arg_name) in method_data.args.iter().enumerate() {
                        if let Some(arg_value) = args.get(i) {
                            frame.set_local(arg_name.clone(), arg_value.clone());
                        }
                    }
                }
                
                // Execute the method body
                // Get the current frame
                let _current_frame = self.current_frame.as_ref().expect("Frame must exist");
                println!("Executing method body: {:?}", &method_data.body);
                let result = self.execute_method_body(&method_data.body);
                
                // If there's no explicit return value, return the receiver
                let result = match result {
                    Some(val) => val,
                    None => receiver.clone(),
                };
                
                // Pop call frame
                self.pop_call_frame();
                
                result
            }
            None => {
                // Not a method or method not found
                Value::Nil
            }
        }
    }
    
    /// Execute a method body
    fn execute_method_body(&mut self, body: &MethodBody) -> Option<Value> {
        match body {
            MethodBody::Source(source) => {
                println!("Executing source: {}", source);
                // Parse the source on-demand
                match iowa_parser::parse(source) {
                    Ok((_, chains)) if !chains.is_empty() => {
                        println!("Successfully parsed, chain count: {}", chains.len());
                        // Execute the first message chain
                        let result = self.execute_message_chain(&chains[0]);
                        println!("Chain execution result: {:?}", result);
                        result
                    }
                    Ok((_, _chains)) => {
                        println!("Parse succeeded but no chains found");
                        None
                    }
                    Err(e) => {
                        println!("Parse error: {:?}", e);
                        None
                    }
                }
            },
            MethodBody::Bytecode(bytecode_data) => {
                println!("Executing bytecode with {} bytes", bytecode_data.len());
                
                // Deserialize bytecode from raw bytes
                let bytecode = self.deserialize_bytecode(bytecode_data);
                
                // Get current frame information for arguments
                let (args, receiver) = match &self.current_frame {
                    Some(frame) => {
                        let args = frame.args().to_vec();
                        let receiver = Value::Object(frame.receiver());
                        (args, receiver)
                    },
                    None => (vec![], Value::Nil),
                };
                
                // Create a bytecode VM and execute the bytecode
                let mut vm = crate::runtime::bytecode::BytecodeVM::new(self as *mut Runtime);
                let result = unsafe { vm.execute(&bytecode, &args, receiver) };
                
                println!("Bytecode execution result: {:?}", result);
                Some(result)
            }
        }
    }
    
    /// Deserialize a bytecode object from raw bytes
    fn deserialize_bytecode(&self, data: &[u8]) -> crate::runtime::bytecode::Bytecode {
        // Use the bytecode's deserialize method
        match crate::runtime::bytecode::Bytecode::deserialize(data) {
            Some(bytecode) => bytecode,
            None => {
                println!("Failed to deserialize bytecode, falling back to empty bytecode");
                // Fall back to empty bytecode if deserialization fails
                crate::runtime::bytecode::Bytecode::new()
            }
        }
    }
    
    /// Execute a message chain in the context of a method
    fn execute_message_chain(&mut self, chain: &iowa_parser::MessageChain) -> Option<Value> {
        // Get the current frame
        let current_frame = if let Some(frame) = self.current_frame.as_ref() {
            frame
        } else {
            return None;
        };
        
        // Check if there's already a return value
        if let Some(return_value) = current_frame.return_value.as_ref() {
            return Some(return_value.clone());
        }
        
        println!("Executing chain with {} messages", chain.messages.len());
        
        // Execute each message in the chain
        let mut result = Value::Nil;
        let mut context = Value::Object(current_frame.receiver());
        
        for message in &chain.messages {
            // Process control flow statements 
            println!("Processing message with symbol: {:?}", message.symbol);
            
            // Handle return statements - can be operators or identifiers
            let is_return = match &message.symbol {
                iowa_parser::Symbol::Identifier(id) => id.name() == "return",
                iowa_parser::Symbol::Operator(op) => op.symbol() == "return",
                _ => false
            };
            
            if is_return {
                println!("Processing return statement");
                // Handle return statement
                if !message.args.is_empty() && !message.args[0].chains.is_empty() {
                    // Execute the return expression
                    println!("Executing return expression");
                    let return_value = self.execute_message_chain(&message.args[0].chains[0])?;
                    println!("Return expression result: {:?}", return_value);
                    
                    // Set the return value in the frame
                    if let Some(frame) = self.current_frame.as_mut() {
                        frame.set_return_value(return_value.clone());
                        println!("Set return value in frame: {:?}", return_value);
                    }
                    
                    return Some(return_value);
                }
                
                // Return the current result if no argument provided
                if let Some(frame) = self.current_frame.as_mut() {
                    frame.set_return_value(result.clone());
                }
                
                return Some(result);
            } else if let iowa_parser::Symbol::Identifier(id) = &message.symbol {
                if id.name() == "if" {
                    // Handle if statement
                    if message.args.len() >= 2 {
                        // Execute the condition
                        if !message.args[0].chains.is_empty() {
                            let condition = self.execute_message_chain(&message.args[0].chains[0])?;
                            
                            // Check if the condition is true (non-nil and non-false)
                            let is_true = match condition {
                                Value::Nil => false,
                                Value::Boolean(b) => b,
                                _ => true,
                            };
                            
                            if is_true {
                                // Execute the then branch
                                if !message.args[1].chains.is_empty() {
                                    result = self.execute_message_chain(&message.args[1].chains[0])?;
                                    
                                    // Check if there's now a return value
                                    if let Some(frame) = self.current_frame.as_ref() {
                                        if let Some(return_value) = frame.return_value.as_ref() {
                                            return Some(return_value.clone());
                                        }
                                    }
                                }
                            } else if message.args.len() >= 3 {
                                // Execute the else branch if it exists
                                if !message.args[2].chains.is_empty() {
                                    result = self.execute_message_chain(&message.args[2].chains[0])?;
                                    
                                    // Check if there's now a return value
                                    if let Some(frame) = self.current_frame.as_ref() {
                                        if let Some(return_value) = frame.return_value.as_ref() {
                                            return Some(return_value.clone());
                                        }
                                    }
                                }
                            }
                            
                            continue;
                        }
                    }
                }
            }
            
            // For regular messages, evaluate the receiver and arguments
            let receiver_value = match &message.symbol {
                iowa_parser::Symbol::Identifier(id) => {
                    // Look up the identifier in locals first
                    if let Some(frame) = self.current_frame.as_ref() {
                        println!("Looking up identifier: {}", id.name());
                        
                        // First try to find it in locals
                        if let Some(local) = frame.get_local(id.name()) {
                            println!("Found in locals: {:?}", local);
                            local.clone()
                        } else if id.name() == "method" {
                            // Handle method definition
                            // Arguments are the parameter names
                            let mut arg_names = Vec::new();
                            for (i, arg) in message.args.iter().enumerate() {
                                // Skip the method body (last argument)
                                if i < message.args.len() - 1 && !arg.chains.is_empty() {
                                    // Get parameter name from identifier
                                    if let Some(param_msg) = arg.chains[0].messages.first() {
                                        if let iowa_parser::Symbol::Identifier(param_id) = &param_msg.symbol {
                                            arg_names.push(param_id.name().to_string());
                                        }
                                    }
                                }
                            }
                            
                            // Get method body (last argument)
                            if !message.args.is_empty() {
                                let last_arg = &message.args[message.args.len() - 1];
                                
                                if !last_arg.chains.is_empty() {
                                    // We need to convert the body argument to source code
                                    // In a real implementation, we'd serialize the AST back to source
                                    // For now, just use a placeholder empty body
                                    
                                    // Create the method data
                                    let method_data = MethodData {
                                        args: arg_names,
                                        body: MethodBody::Source("".to_string()),
                                    };
                                    
                                    // Allocate the method object
                                    let method_id = self.memory.alloc_method(MethodType::UserDefined(method_data));
                                    Value::Object(method_id)
                                } else {
                                    // Empty body
                                    Value::Object(0)
                                }
                            } else {
                                // Return a default method if creation failed
                                Value::Object(0)
                            }
                        } else {
                            // Not found in locals, use as message to the context object
                            context.clone()
                        }
                    } else {
                        Value::Nil
                    }
                }
                iowa_parser::Symbol::Number(n) => {
                    match n {
                        iowa_parser::Number::Decimal(value) => Value::Number(*value),
                        iowa_parser::Number::Hex(value) => Value::Number(*value as f64),
                    }
                }
                iowa_parser::Symbol::Quote(q) => Value::String(q.content().to_string()),
                iowa_parser::Symbol::Operator(_op) => {
                    // For operators, the context (previous result) is the receiver
                    context.clone()
                }
            };
            
            // Compile arguments
            let mut arg_values = Vec::new();
            for arg in &message.args {
                for chain in &arg.chains {
                    if let Some(arg_value) = self.execute_message_chain(chain) {
                        arg_values.push(arg_value);
                    }
                }
            }
            
            // For numbers and strings, return them directly without message dispatch
            if let iowa_parser::Symbol::Number(n) = &message.symbol {
                println!("Got number: {:?}", n);
                match n {
                    iowa_parser::Number::Decimal(val) => return Some(Value::Number(*val)),
                    iowa_parser::Number::Hex(val) => return Some(Value::Number(*val as f64)),
                }
            }
            
            if let iowa_parser::Symbol::Quote(q) = &message.symbol {
                println!("Got string: {:?}", q);
                return Some(Value::String(q.content().to_string()));
            }
            
            // Get the message name
            let message_name = match &message.symbol {
                iowa_parser::Symbol::Identifier(id) => id.name(),
                iowa_parser::Symbol::Operator(op) => op.symbol(),
                iowa_parser::Symbol::Number(_) => "asNumber",
                iowa_parser::Symbol::Quote(_) => "asString",
            };
            
            // Special handling for assignment operator
            if message_name == ":=" && !arg_values.is_empty() {
                // For := operator, get the variable name from the previous message symbol
                if let iowa_parser::Symbol::Identifier(id) = &message.symbol {
                    let var_name = id.name().to_string();
                    println!("Assigning variable: {} = {:?}", var_name, arg_values[0]);
                    
                    // Set the local variable
                    if let Some(frame) = self.current_frame.as_mut() {
                        frame.set_local(var_name, arg_values[0].clone());
                        result = arg_values[0].clone();
                    } else {
                        result = Value::Nil;
                    }
                } else {
                    // For non-identifier receivers, dispatch as regular message
                    result = self.dispatch_message(receiver_value, message_name, arg_values);
                }
            } else {
                // Regular message dispatch
                result = self.dispatch_message(receiver_value, message_name, arg_values);
            }
            
            // Check if there's now a return value from the dispatch
            if let Some(frame) = self.current_frame.as_ref() {
                if let Some(return_value) = frame.return_value.as_ref() {
                    return Some(return_value.clone());
                }
            }
            
            // Update the context for the next message in the chain
            context = result.clone();
        }
        
        Some(result)
    }
    
    /// Call a primitive method
    fn call_primitive_method(&mut self, index: u32, receiver: ObjectRef, args: Vec<Value>) -> Value {
        match index {
            // Number methods
            1 => self.number_add(receiver, args),
            2 => self.number_subtract(receiver, args),
            3 => self.number_multiply(receiver, args),
            4 => self.number_divide(receiver, args),
            
            // String methods
            10 => self.string_size(receiver, args),
            11 => self.string_at(receiver, args),
            12 => self.string_slice(receiver, args),
            
            // List methods
            20 => self.list_size(receiver, args),
            21 => self.list_at(receiver, args),
            22 => self.list_append(receiver, args),
            
            // Map methods
            30 => self.map_size(receiver, args),
            31 => self.map_at(receiver, args),
            32 => self.map_at_put(receiver, args),
            
            // Unknown primitive method
            _ => Value::Nil,
        }
    }
    
    /// Number addition primitive
    fn number_add(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver value
        let n1 = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Number(n) => *n,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get argument value
        let n2 = match &args[0] {
            Value::Number(n) => *n,
            Value::Object(id) => {
                match self.memory.get_object(*id) {
                    Some(obj) => {
                        match &obj.data {
                            ObjectData::Number(n) => *n,
                            _ => return Value::Nil,
                        }
                    }
                    None => return Value::Nil,
                }
            }
            _ => return Value::Nil,
        };
        
        Value::Number(n1 + n2)
    }
    
    /// Number subtraction primitive
    fn number_subtract(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver value
        let n1 = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Number(n) => *n,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get argument value
        let n2 = match &args[0] {
            Value::Number(n) => *n,
            Value::Object(id) => {
                match self.memory.get_object(*id) {
                    Some(obj) => {
                        match &obj.data {
                            ObjectData::Number(n) => *n,
                            _ => return Value::Nil,
                        }
                    }
                    None => return Value::Nil,
                }
            }
            _ => return Value::Nil,
        };
        
        Value::Number(n1 - n2)
    }
    
    /// Number multiplication primitive
    fn number_multiply(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver value
        let n1 = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Number(n) => *n,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get argument value
        let n2 = match &args[0] {
            Value::Number(n) => *n,
            Value::Object(id) => {
                match self.memory.get_object(*id) {
                    Some(obj) => {
                        match &obj.data {
                            ObjectData::Number(n) => *n,
                            _ => return Value::Nil,
                        }
                    }
                    None => return Value::Nil,
                }
            }
            _ => return Value::Nil,
        };
        
        Value::Number(n1 * n2)
    }
    
    /// Number division primitive
    fn number_divide(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver value
        let n1 = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Number(n) => *n,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get argument value
        let n2 = match &args[0] {
            Value::Number(n) => *n,
            Value::Object(id) => {
                match self.memory.get_object(*id) {
                    Some(obj) => {
                        match &obj.data {
                            ObjectData::Number(n) => *n,
                            _ => return Value::Nil,
                        }
                    }
                    None => return Value::Nil,
                }
            }
            _ => return Value::Nil,
        };
        
        // Check for division by zero
        if n2 == 0.0 {
            return Value::Nil;
        }
        
        Value::Number(n1 / n2)
    }
    
    /// String size primitive
    fn string_size(&self, receiver: ObjectRef, _args: Vec<Value>) -> Value {
        // Get receiver string
        match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::String(s) => Value::Number(s.len() as f64),
                    _ => Value::Number(0.0),
                }
            }
            None => Value::Number(0.0),
        }
    }
    
    /// String at primitive
    fn string_at(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver string
        let s = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::String(s) => s,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get index
        let idx = match &args[0] {
            Value::Number(n) => *n as usize,
            _ => return Value::Nil,
        };
        
        // Check bounds
        if idx >= s.len() {
            return Value::Nil;
        }
        
        // Get character at index
        if let Some(c) = s.chars().nth(idx) {
            Value::String(c.to_string())
        } else {
            Value::Nil
        }
    }
    
    /// String slice primitive
    fn string_slice(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.len() < 2 {
            return Value::Nil;
        }
        
        // Get receiver string
        let s = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::String(s) => s,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get start and end indices
        let start = match &args[0] {
            Value::Number(n) => *n as usize,
            _ => return Value::Nil,
        };
        
        let end = match &args[1] {
            Value::Number(n) => *n as usize,
            _ => return Value::Nil,
        };
        
        // Check bounds
        if start > end || end > s.len() {
            return Value::Nil;
        }
        
        // Return slice
        Value::String(s[start..end].to_string())
    }
    
    /// List size primitive
    fn list_size(&self, receiver: ObjectRef, _args: Vec<Value>) -> Value {
        // Get receiver list
        match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::List(items) => Value::Number(items.len() as f64),
                    _ => Value::Number(0.0),
                }
            }
            None => Value::Number(0.0),
        }
    }
    
    /// List at primitive
    fn list_at(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver list
        let items = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::List(items) => items,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get index
        let idx = match &args[0] {
            Value::Number(n) => *n as usize,
            _ => return Value::Nil,
        };
        
        // Check bounds
        if idx >= items.len() {
            return Value::Nil;
        }
        
        // Return item at index
        items[idx].clone()
    }
    
    /// List append primitive
    fn list_append(&mut self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Object(receiver);
        }
        
        // Get receiver list
        let obj = match self.memory.get_object_mut(receiver) {
            Some(obj) => obj,
            None => return Value::Nil,
        };
        
        // Append item
        match &mut obj.data {
            ObjectData::List(items) => {
                items.push(args[0].clone());
                Value::Object(receiver)
            }
            _ => Value::Nil,
        }
    }
    
    /// Map size primitive
    fn map_size(&self, receiver: ObjectRef, _args: Vec<Value>) -> Value {
        // Get receiver map
        match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Map(entries) => Value::Number(entries.len() as f64),
                    _ => Value::Number(0.0),
                }
            }
            None => Value::Number(0.0),
        }
    }
    
    /// Map at primitive
    fn map_at(&self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Nil;
        }
        
        // Get receiver map
        let entries = match self.memory.get_object(receiver) {
            Some(obj) => {
                match &obj.data {
                    ObjectData::Map(entries) => entries,
                    _ => return Value::Nil,
                }
            }
            None => return Value::Nil,
        };
        
        // Get key
        let key = match &args[0] {
            Value::String(s) => s,
            _ => return Value::Nil,
        };
        
        // Return value for key
        entries.get(key).cloned().unwrap_or(Value::Nil)
    }
    
    /// Map atPut primitive
    fn map_at_put(&mut self, receiver: ObjectRef, args: Vec<Value>) -> Value {
        if args.len() < 2 {
            return Value::Object(receiver);
        }
        
        // Get receiver map
        let obj = match self.memory.get_object_mut(receiver) {
            Some(obj) => obj,
            None => return Value::Nil,
        };
        
        // Get key and value
        let key = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Value::Nil,
        };
        
        // Set key/value pair
        match &mut obj.data {
            ObjectData::Map(entries) => {
                entries.insert(key, args[1].clone());
                Value::Object(receiver)
            }
            _ => Value::Nil,
        }
    }
    
    /// Encode a reference value without needing a FunctionBuilder
    fn runtime_encode_reference(&self, ptr: u64, tag: u64) -> u64 {
        // Reference is the pointer shifted left by TAG_BITS, then OR'd with the tag
        (ptr << value::TAG_BITS) | tag
    }
    
    /// Convert a value to a tagged u64 for runtime
    pub fn to_tagged_value(&self, val: &Value) -> u64 {
        match val {
            Value::Nil => value::TAG_NIL,
            Value::Boolean(b) => {
                let bool_val = if *b { 1 } else { 0 };
                (bool_val << value::TAG_BITS) | value::TAG_BOOLEAN
            }
            Value::Number(n) => {
                // Allocate a number object
                let num_id = self.memory.alloc_number_const(*n);
                self.runtime_encode_reference(num_id, value::TAG_OBJECT_REF)
            }
            Value::String(s) => {
                // Allocate a string object
                let str_id = self.memory.alloc_string_const(s);
                self.runtime_encode_reference(str_id, value::TAG_STRING_REF)
            }
            Value::Object(id) => {
                self.runtime_encode_reference(*id, value::TAG_OBJECT_REF)
            }
        }
    }
    
    /// Decode a reference value to extract the pointer
    fn runtime_decode_reference(&self, value: u64) -> u64 {
        value >> value::TAG_BITS
    }
    
    /// Create a value from a tagged u64
    pub fn from_tagged_value(&self, val: u64) -> Value {
        let tag = val & value::TAG_MASK;
        match tag {
            value::TAG_NIL => Value::Nil,
            value::TAG_BOOLEAN => {
                let bool_val = (val >> value::TAG_BITS) != 0;
                Value::Boolean(bool_val)
            }
            value::TAG_OBJECT_REF => {
                let obj_id = self.runtime_decode_reference(val);
                Value::Object(obj_id)
            }
            value::TAG_STRING_REF => {
                let str_id = self.runtime_decode_reference(val);
                // Extract string from object
                match self.memory.get_object(str_id) {
                    Some(obj) => {
                        match &obj.data {
                            ObjectData::String(s) => Value::String(s.clone()),
                            _ => Value::String("".to_string()),
                        }
                    }
                    None => Value::String("".to_string()),
                }
            }
            _ => Value::Nil,
        }
    }
    
    /// Allocate a new object
    pub fn alloc_object(&mut self, prototype: u64) -> u64 {
        self.memory.alloc_object(prototype)
    }
    
    /// Allocate a string
    pub fn alloc_string(&mut self, value: &str) -> u64 {
        self.memory.alloc_string(Value::String(value.to_string()))
    }
    
    /// Allocate a number
    pub fn alloc_number(&mut self, value: f64) -> u64 {
        self.memory.alloc_number(Value::Number(value))
    }
    
    /// Create a user-defined method from source code
    pub fn create_method(&mut self, arg_names: Vec<String>, body_source: &str) -> Option<ObjectRef> {
        // Verify that the source can be parsed
        match iowa_parser::parse(body_source) {
            Ok((_, chains)) => {
                // Decide whether to use source or bytecode based on configuration
                let use_bytecode = true; // Use bytecode execution by default
                
                let method_data = if use_bytecode && !chains.is_empty() {
                    // Create method data with precompiled bytecode
                    let method_data = MethodData {
                        args: arg_names.clone(),
                        body: MethodBody::Source(body_source.to_string()), // Temporary source reference
                    };
                    
                    // Generate bytecode from the parsed AST
                    let bytecode = crate::runtime::bytecode::Bytecode::from_message_chain(&chains[0], &method_data);
                    
                    // Serialize bytecode to bytes
                    let bytecode_data = self.serialize_bytecode(&bytecode);
                    
                    // Create final method data with bytecode
                    MethodData {
                        args: arg_names,
                        body: MethodBody::Bytecode(bytecode_data),
                    }
                } else {
                    // Store the source directly for interpreted execution
                    MethodData {
                        args: arg_names,
                        body: MethodBody::Source(body_source.to_string()),
                    }
                };
                
                // Allocate the method object
                Some(self.memory.alloc_method(MethodType::UserDefined(method_data)))
            }
            _ => None,
        }
    }
    
    /// Create a user-defined method directly from bytecode
    pub fn create_method_with_bytecode(&mut self, arg_names: Vec<String>, bytecode_data: Vec<u8>) -> Option<ObjectRef> {
        // Create method data with bytecode
        let method_data = MethodData {
            args: arg_names,
            body: MethodBody::Bytecode(bytecode_data),
        };
        
        // Allocate the method object
        Some(self.memory.alloc_method(MethodType::UserDefined(method_data)))
    }
    
    /// Serialize a bytecode object to raw bytes
    fn serialize_bytecode(&self, bytecode: &crate::runtime::bytecode::Bytecode) -> Vec<u8> {
        // Use the bytecode's own serialization method
        bytecode.serialize()
    }
    
    /// Run garbage collection
    pub fn collect_garbage(&mut self) {
        // We need to get the roots first to avoid self-borrowing issues
        let lobby = self.lobby;
        let object_proto = self.prototypes.object;
        let number_proto = self.prototypes.number;
        let string_proto = self.prototypes.string;
        let list_proto = self.prototypes.list;
        let map_proto = self.prototypes.map;
        
        let mut call_frame_roots = Vec::new();
        
        // Extract data from current frame if any
        if let Some(frame) = &self.current_frame {
            call_frame_roots.push(frame.method);
            call_frame_roots.push(frame.receiver);
            
            // Add values from arguments and locals
            for arg in &frame.args {
                if let Value::Object(id) = arg {
                    call_frame_roots.push(*id);
                }
            }
            
            for (_, value) in &frame.locals {
                if let Value::Object(id) = value {
                    call_frame_roots.push(*id);
                }
            }
        }
        
        // Now we can call the memory's collect_garbage with all our roots
        self.memory.collect_garbage_with_roots(
            &[lobby, object_proto, number_proto, string_proto, list_proto, map_proto],
            &call_frame_roots
        );
    }
    
    /// Call a method
    pub fn call_method_by_name(&mut self, receiver: u64, method_name: &str, args: Vec<u64>) -> u64 {
        // Convert arguments from tagged values
        let arg_values: Vec<Value> = args.iter()
            .map(|arg| self.from_tagged_value(*arg))
            .collect();
        
        // Dispatch the message
        let receiver_val = self.from_tagged_value(receiver);
        let result = self.dispatch_message(receiver_val, method_name, arg_values);
        
        // Convert result to tagged value
        self.to_tagged_value(&result)
    }
}

/// Core prototype objects in the Io system
#[derive(Default)]
pub struct Prototypes {
    /// Object prototype (the root object)
    pub object: ObjectRef,
    /// Number prototype
    pub number: ObjectRef,
    /// String prototype
    pub string: ObjectRef,
    /// List prototype
    pub list: ObjectRef,
    /// Map prototype
    pub map: ObjectRef,
}

/// Reference to an Io object (just a u64 ID)
pub type ObjectRef = u64;

/// Call frame for method execution
pub struct CallFrame {
    /// The currently executing method
    method: ObjectRef,
    /// The receiver object
    receiver: ObjectRef,
    /// The message being handled
    message: String,
    /// Local variables
    locals: HashMap<String, Value>,
    /// Arguments passed to the method
    args: Vec<Value>,
    /// Return value (if any)
    return_value: Option<Value>,
    /// Parent frame (for call stack)
    parent: Option<Box<CallFrame>>,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(
        method: ObjectRef,
        receiver: ObjectRef,
        message: String,
        args: Vec<Value>,
        parent: Option<Box<CallFrame>>,
    ) -> Self {
        Self {
            method,
            receiver,
            message,
            locals: HashMap::new(),
            args,
            return_value: None,
            parent,
        }
    }
    
    /// Set the return value for this frame
    pub fn set_return_value(&mut self, value: Value) {
        self.return_value = Some(value);
    }
    
    /// Get the return value, if any
    pub fn return_value(&self) -> Option<&Value> {
        self.return_value.as_ref()
    }

    /// Get the method
    pub fn method(&self) -> ObjectRef {
        self.method
    }
    
    /// Get the receiver
    pub fn receiver(&self) -> ObjectRef {
        self.receiver
    }
    
    /// Get the message
    pub fn message(&self) -> &str {
        &self.message
    }
    
    /// Get the arguments
    pub fn args(&self) -> &[Value] {
        &self.args
    }
    
    /// Get the parent frame
    pub fn parent(&self) -> &Option<Box<CallFrame>> {
        &self.parent
    }

    /// Get a local variable
    pub fn get_local(&self, name: &str) -> Option<&Value> {
        self.locals.get(name)
    }

    /// Set a local variable
    pub fn set_local(&mut self, name: String, value: Value) {
        self.locals.insert(name, value);
    }
}

/// The possible types of Io values
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Nil value
    Nil,
    /// Boolean value
    Boolean(bool),
    /// Number value
    Number(f64),
    /// String value
    String(String),
    /// Object reference
    Object(ObjectRef),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Object(id) => write!(f, "Object_{}", id),
        }
    }
}

/// Memory management for the runtime
pub struct Memory {
    /// All allocated objects
    objects: HashMap<ObjectRef, Object>,
    /// Bytes allocated since last GC
    allocated_since_gc: usize,
    /// GC threshold
    gc_threshold: usize,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            allocated_since_gc: 0,
            gc_threshold: 1024 * 1024, // 1MB initial threshold
        }
    }

    /// Allocate a new object with given prototype
    pub fn alloc_object(&mut self, prototype: ObjectRef) -> ObjectRef {
        // Generate a unique ID 
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        
        // Create a generic object
        let obj = Object {
            id,
            prototype,
            slots: HashMap::new(),
            data: ObjectData::Generic,
            mark: false,
        };
        
        // Track allocation
        self.allocated_since_gc += memory::layout::object::HEADER_SIZE;
        
        // Store in objects table
        self.objects.insert(id, obj);
        
        // Check GC threshold
        self.check_gc_threshold();
        
        id
    }
    
    /// Allocate a number object
    pub fn alloc_number(&mut self, value: Value) -> ObjectRef {
        if let Value::Number(num) = value {
            // Create number object
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            
            // Get the Number prototype (will be set by caller)
            let prototype = 0;
            
            let obj = Object {
                id,
                prototype,
                slots: HashMap::new(),
                data: ObjectData::Number(num),
                mark: false,
            };
            
            // Track allocation
            self.allocated_since_gc += memory::layout::number::SIZE;
            
            // Store in objects table
            self.objects.insert(id, obj);
            
            // Check GC threshold
            self.check_gc_threshold();
            
            id
        } else {
            // Not a number, allocation failure
            0
        }
    }
    
    /// Allocate a number object (for const value)
    pub fn alloc_number_const(&self, _value: f64) -> ObjectRef {
        // Create number object in a way that doesn't require mutating memory
        // This is used for const value conversion
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        id
    }
    
    /// Allocate a string object 
    pub fn alloc_string(&mut self, value: Value) -> ObjectRef {
        if let Value::String(s) = value {
            // Create string object
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            
            // Get the String prototype (will be set by caller)
            let prototype = 0;
            
            let obj = Object {
                id,
                prototype,
                slots: HashMap::new(),
                data: ObjectData::String(s.clone()),
                mark: false,
            };
            
            // Track allocation
            self.allocated_since_gc += memory::layout::string::HEADER_SIZE + s.len();
            
            // Store in objects table
            self.objects.insert(id, obj);
            
            // Check GC threshold
            self.check_gc_threshold();
            
            id
        } else {
            // Not a string, allocation failure
            0
        }
    }
    
    /// Allocate a const string object
    pub fn alloc_string_const(&self, _s: &str) -> ObjectRef {
        // Create string object in a way that doesn't require mutating memory
        // This is used for const string conversion
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        id
    }
    
    /// Allocate a list object
    pub fn alloc_list(&mut self, items: Vec<Value>) -> ObjectRef {
        // Create list object
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        
        // Get the List prototype (will be set by caller)
        let prototype = 0;
        
        // Track allocation (rough estimate) before moving items
        let items_size = items.len() * 8;
        
        let obj = Object {
            id,
            prototype,
            slots: HashMap::new(),
            data: ObjectData::List(items),
            mark: false,
        };
        
        // Track allocation
        self.allocated_since_gc += memory::layout::object::HEADER_SIZE + items_size;
        
        // Store in objects table
        self.objects.insert(id, obj);
        
        // Check GC threshold
        self.check_gc_threshold();
        
        id
    }
    
    /// Allocate a map object
    pub fn alloc_map(&mut self, entries: HashMap<String, Value>) -> ObjectRef {
        // Create map object
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        
        // Get the Map prototype (will be set by caller)
        let prototype = 0;
        
        // Track size before moving entries
        let entries_size = entries.len() * 16;
        
        let obj = Object {
            id,
            prototype,
            slots: HashMap::new(),
            data: ObjectData::Map(entries),
            mark: false,
        };
        
        // Track allocation (rough estimate)
        self.allocated_since_gc += memory::layout::object::HEADER_SIZE + entries_size;
        
        // Store in objects table
        self.objects.insert(id, obj);
        
        // Check GC threshold
        self.check_gc_threshold();
        
        id
    }
    
    /// Allocate a method object
    pub fn alloc_method(&mut self, method_type: MethodType) -> ObjectRef {
        // Create method object
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        
        // Get the Object prototype (will be set by caller)
        let prototype = 0;
        
        let obj = Object {
            id,
            prototype,
            slots: HashMap::new(),
            data: ObjectData::Method(method_type),
            mark: false,
        };
        
        // Track allocation
        self.allocated_since_gc += memory::layout::object::HEADER_SIZE;
        
        // Store in objects table
        self.objects.insert(id, obj);
        
        // Check GC threshold
        self.check_gc_threshold();
        
        id
    }
    
    /// Get an object by ID
    pub fn get_object(&self, id: ObjectRef) -> Option<&Object> {
        self.objects.get(&id)
    }
    
    /// Get a mutable object by ID
    pub fn get_object_mut(&mut self, id: ObjectRef) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }
    
    /// Check if GC should run
    fn check_gc_threshold(&mut self) {
        if self.allocated_since_gc > self.gc_threshold {
            // GC needs full runtime access, but we don't have it here
            // This will be handled by the caller
        }
    }
    
    /// Run garbage collection with explicit roots
    pub fn collect_garbage_with_roots(&mut self, object_roots: &[ObjectRef], value_roots: &[ObjectRef]) {
        // Mark phase - mark all reachable objects
        // Reset all mark bits
        for obj in self.objects.values_mut() {
            obj.mark = false;
        }
        
        // Mark from object roots
        for &root_id in object_roots {
            self.mark_object(root_id);
        }
        
        // Mark from value roots
        for &root_id in value_roots {
            self.mark_object(root_id);
        }
        
        // Sweep phase - remove unmarked objects
        let objects_to_remove: Vec<ObjectRef> = self.objects
            .iter()
            .filter(|(_id, obj)| !obj.mark)
            .map(|(id, _obj)| *id)
            .collect();
        
        for id in objects_to_remove {
            self.objects.remove(&id);
        }
        
        // Reset allocation counter
        self.allocated_since_gc = 0;
    }
    
    /// Legacy method for backward compatibility
    #[deprecated]
    pub fn collect_garbage(&mut self, runtime: &Runtime) {
        // Create arrays of roots
        let object_roots = [
            runtime.lobby(),
            runtime.prototypes().object,
            runtime.prototypes().number,
            runtime.prototypes().string,
            runtime.prototypes().list,
            runtime.prototypes().map,
        ];
        
        let mut value_roots = Vec::new();
        
        // Add call frame roots if any
        if let Some(frame) = runtime.current_frame() {
            value_roots.push(frame.method);
            value_roots.push(frame.receiver);
        }
        
        self.collect_garbage_with_roots(&object_roots, &value_roots);
    }
    
    /// Mark an object as reachable
    fn mark_object(&mut self, id: ObjectRef) {
        // First check if already marked to avoid cycles
        if let Some(obj) = self.objects.get(&id) {
            if obj.mark {
                return; // Already marked
            }
        } else {
            return; // Object not found
        }
        
        // Mark this object
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.mark = true;
        }
        
        // Get prototype and other data to mark
        let mut prototype = 0;
        let mut items_to_mark: Vec<ObjectRef> = Vec::new();
        
        // Collect data that needs marking
        if let Some(obj) = self.objects.get(&id) {
            prototype = obj.prototype;
            
            // Collect slot values
            for (_name, value) in &obj.slots {
                if let Value::Object(ref_id) = value {
                    items_to_mark.push(*ref_id);
                }
            }
            
            // Collect items from object data
            match &obj.data {
                ObjectData::List(items) => {
                    for item in items {
                        if let Value::Object(ref_id) = item {
                            items_to_mark.push(*ref_id);
                        }
                    }
                }
                ObjectData::Map(entries) => {
                    for (_key, value) in entries {
                        if let Value::Object(ref_id) = value {
                            items_to_mark.push(*ref_id);
                        }
                    }
                }
                ObjectData::Method(method_type) => {
                    // For method objects, mark their activation if any
                    if let MethodType::UserDefined(_method_data) = method_type {
                        // Mark message chain if it's an activation
                        // In a real implementation, this would mark all captured variables
                    }
                }
                _ => {} // Other data types don't contain references
            }
        }
        
        // Mark prototype first
        self.mark_object(prototype);
        
        // Mark all collected items
        for item_id in items_to_mark {
            self.mark_object(item_id);
        }
        
        // No need to mark entries separately, they're included in items_to_mark
    }
    
    /// Mark a value as reachable
    fn mark_value(&mut self, value: &Value) {
        if let Value::Object(id) = value {
            self.mark_object(*id);
        }
    }
}

/// Method types for Io
#[derive(Clone, Debug)]
pub enum MethodType {
    /// Built-in method (implemented in Rust)
    Primitive(u32), // Index into primitive method table
    /// User-defined method (Io code)
    UserDefined(MethodData),
}

/// Data for user-defined methods
#[derive(Clone, Debug)]
pub struct MethodData {
    /// List of argument names
    pub args: Vec<String>,
    /// Method body as an AST MessageChain
    pub body: MethodBody,
}

/// Method body representation
#[derive(Clone, Debug)]
pub enum MethodBody {
    /// Method body as a source string that can be parsed
    Source(String),
    /// Raw bytecode (for future optimized methods)
    Bytecode(Vec<u8>),
}

/// Io object representation
pub struct Object {
    /// Unique ID for this object
    id: ObjectRef,
    /// Prototype object ID
    prototype: ObjectRef,
    /// Method/attribute slots
    slots: HashMap<String, Value>,
    /// Type-specific data
    data: ObjectData,
    /// Mark bit for garbage collection
    mark: bool,
}

impl Object {
    /// Get the object ID
    pub fn id(&self) -> ObjectRef {
        self.id
    }
    
    /// Get the prototype
    pub fn prototype(&self) -> ObjectRef {
        self.prototype
    }
    
    /// Get the object data
    pub fn data(&self) -> &ObjectData {
        &self.data
    }
    
    /// Set a slot value
    pub fn set_slot(&mut self, name: String, value: Value) {
        self.slots.insert(name, value);
    }
    
    /// Get a slot value
    pub fn get_slot(&self, name: &str) -> Option<&Value> {
        self.slots.get(name)
    }
    
    /// Look up a slot in this object or its prototype chain
    pub fn lookup_slot(&self, name: &str, runtime: &Runtime) -> Value {
        // Check if we have this slot
        if let Some(value) = self.slots.get(name) {
            return value.clone();
        }
        
        // Not found, check prototype (if we're not Object prototype)
        if self.prototype != 0 && self.prototype != self.id {
            if let Some(proto_obj) = runtime.memory().get_object(self.prototype) {
                return proto_obj.lookup_slot(name, runtime);
            }
        }
        
        // Not found
        Value::Nil
    }
    
    /// Get the object's type name
    pub fn type_name(&self) -> String {
        if let Some(Value::String(name)) = self.get_slot("type") {
            name.clone()
        } else {
            // Determine by object data type
            match &self.data {
                ObjectData::Number(_) => "Number".to_string(),
                ObjectData::String(_) => "String".to_string(),
                ObjectData::List(_) => "List".to_string(),
                ObjectData::Map(_) => "Map".to_string(),
                ObjectData::Method(_) => "Method".to_string(),
                ObjectData::Generic => "Object".to_string(),
            }
        }
    }
    
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match &self.data {
            ObjectData::Number(n) => n.to_string(),
            ObjectData::String(s) => s.clone(),
            ObjectData::List(items) => {
                let items_str: Vec<String> = items.iter()
                    .map(|item| item.to_string())
                    .collect();
                format!("list({})", items_str.join(", "))
            }
            ObjectData::Map(entries) => {
                let pairs: Vec<String> = entries.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                format!("map({})", pairs.join(", "))
            }
            ObjectData::Method(_) => {
                format!("method({})", self.id)
            }
            ObjectData::Generic => {
                format!("{}_{}", self.type_name(), self.id)
            }
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Type-specific object data
#[derive(Clone, Debug)]
pub enum ObjectData {
    /// Generic object (no special data)
    Generic,
    /// Number object
    Number(f64),
    /// String object
    String(String),
    /// List object
    List(Vec<Value>),
    /// Map object
    Map(HashMap<String, Value>),
    /// Method object
    Method(MethodType),
}

/// The JIT-compatible runtime function versions
/// These wrap the proper object-oriented Runtime API
pub mod trampoline {
    use super::*;
    
    // When the JIT calls functions, it expects these specific function signatures
    // We'll implement them to forward to the proper Runtime API
    
    /// Allocate an object
    pub fn runtime_alloc_object(runtime: &mut Runtime, _size: u32) -> u64 {
        runtime.alloc_object(runtime.prototypes.object)
    }
    
    /// Allocate a string
    pub fn runtime_alloc_string(runtime: &mut Runtime, length: u32) -> u64 {
        let empty = " ".repeat(length as usize);
        runtime.alloc_string(&empty)
    }
    
    /// Allocate a number
    pub fn runtime_alloc_number(runtime: &mut Runtime, value: f64) -> u64 {
        runtime.alloc_number(value)
    }
    
    /// Allocate a slots table
    pub fn runtime_alloc_slots_table(_runtime: &mut Runtime, _size: u32) -> u64 {
        // Just return a placeholder - slots are integrated into objects
        1
    }
    
    /// Run garbage collection
    pub fn runtime_gc_collect(runtime: &mut Runtime) -> i32 {
        runtime.collect_garbage();
        0 // Success
    }
    
    /// Mark an object
    pub fn runtime_gc_mark(runtime: &mut Runtime, _obj: u64) -> i32 {
        // Just run full GC for simplicity
        runtime.collect_garbage();
        0 // Success
    }
    
    /// Check GC threshold
    pub fn runtime_gc_check_threshold(runtime: &mut Runtime, _allocated_bytes: u64) -> i32 {
        // Just run full GC for simplicity
        runtime.collect_garbage();
        0 // Success
    }
    
    /// Call a method
    pub fn runtime_call_method(runtime: &mut Runtime, method: u64, receiver: u64, _args: u64) -> u64 {
        // Extract method name
        let method_name = match runtime.from_tagged_value(method) {
            Value::Object(id) => {
                if let Some(obj) = runtime.memory().get_object(id) {
                    if let Some(Value::String(name)) = obj.get_slot("name") {
                        name.clone()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            }
            Value::String(s) => s,
            _ => "unknown".to_string(),
        };
        
        // For now, handle empty args 
        // In a real implementation, args would be an array
        let arg_vec = Vec::new();
        
        // Call the method
        runtime.call_method_by_name(receiver, &method_name, arg_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use crate::runtime::value::TAG_OBJECT_REF;
    
    #[test]
    fn test_initialize_runtime() {
        let runtime = Runtime::new();
        
        // Check that prototypes were created
        assert_ne!(runtime.prototypes.object, 0);
        assert_ne!(runtime.prototypes.number, 0);
        assert_ne!(runtime.prototypes.string, 0);
        assert_ne!(runtime.prototypes.list, 0);
        assert_ne!(runtime.prototypes.map, 0);
        
        // Check that Lobby was created
        assert_ne!(runtime.lobby, 0);
    }
    
    #[test]
    fn test_memory_allocation() {
        let mut runtime = Runtime::new();
        
        // Test object allocation
        let obj_id = runtime.memory.alloc_object(runtime.prototypes.object);
        assert!(runtime.memory.get_object(obj_id).is_some());
        
        // Test number allocation
        let num_id = runtime.memory.alloc_number(Value::Number(42.0));
        let num_obj = runtime.memory.get_object(num_id).unwrap();
        match &num_obj.data {
            ObjectData::Number(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected Number object"),
        }
        
        // Test string allocation
        let str_id = runtime.memory.alloc_string(Value::String("hello".to_string()));
        let str_obj = runtime.memory.get_object(str_id).unwrap();
        match &str_obj.data {
            ObjectData::String(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected String object"),
        }
        
        // Test list allocation
        let items = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let list_id = runtime.memory.alloc_list(items.clone());
        let list_obj = runtime.memory.get_object(list_id).unwrap();
        match &list_obj.data {
            ObjectData::List(list_items) => {
                assert_eq!(list_items.len(), 3);
                match &list_items[0] {
                    Value::Number(n) => assert_eq!(*n, 1.0),
                    _ => panic!("Expected Number in list"),
                }
            }
            _ => panic!("Expected List object"),
        }
        
        // Test map allocation
        let mut entries = HashMap::new();
        entries.insert("key1".to_string(), Value::Number(42.0));
        entries.insert("key2".to_string(), Value::String("value".to_string()));
        
        let map_id = runtime.memory.alloc_map(entries);
        let map_obj = runtime.memory.get_object(map_id).unwrap();
        match &map_obj.data {
            ObjectData::Map(map_entries) => {
                assert_eq!(map_entries.len(), 2);
                match map_entries.get("key1") {
                    Some(Value::Number(n)) => assert_eq!(*n, 42.0),
                    _ => panic!("Expected Number in map"),
                }
            }
            _ => panic!("Expected Map object"),
        }
    }
    
    #[test]
    fn test_call_frames() {
        let mut runtime = Runtime::new();
        
        // Create method and receiver objects
        let method_id = runtime.memory.alloc_object(runtime.prototypes.object);
        let receiver_id = runtime.memory.alloc_object(runtime.prototypes.object);
        
        // Create some arguments
        let args = vec![Value::Number(1.0), Value::String("arg".to_string())];
        
        // Push a call frame
        runtime.push_call_frame(method_id, receiver_id, "test".to_string(), args.clone());
        
        // Check current frame
        let frame = runtime.current_frame.as_ref().unwrap();
        assert_eq!(frame.method, method_id);
        assert_eq!(frame.receiver, receiver_id);
        assert_eq!(frame.message, "test");
        assert_eq!(frame.args.len(), 2);
        assert!(frame.parent.is_none());
        
        // Push another frame
        let method2_id = runtime.memory.alloc_object(runtime.prototypes.object);
        let receiver2_id = runtime.memory.alloc_object(runtime.prototypes.object);
        
        runtime.push_call_frame(method2_id, receiver2_id, "test2".to_string(), vec![]);
        
        // Check new current frame
        let frame2 = runtime.current_frame.as_ref().unwrap();
        assert_eq!(frame2.method, method2_id);
        assert_eq!(frame2.message, "test2");
        assert!(frame2.parent.is_some());
        
        // Pop frame and check restored frame
        let popped = runtime.pop_call_frame().unwrap();
        assert_eq!(popped.method, method2_id);
        
        let restored = runtime.current_frame.as_ref().unwrap();
        assert_eq!(restored.method, method_id);
        assert_eq!(restored.message, "test");
        
        // Pop last frame
        let last = runtime.pop_call_frame().unwrap();
        assert_eq!(last.method, method_id);
        assert!(runtime.current_frame.is_none());
    }
    
    #[test]
    fn test_message_dispatch() {
        let mut runtime = Runtime::new();
        
        // Test primitive message handling for numbers
        let result = runtime.dispatch_message(Value::Number(5.0), "+", vec![Value::Number(3.0)]);
        match result {
            Value::Number(n) => assert_eq!(n, 8.0),
            _ => panic!("Expected Number result"),
        }
        
        let result = runtime.dispatch_message(Value::Number(10.0), "/", vec![Value::Number(2.0)]);
        match result {
            Value::Number(n) => assert_eq!(n, 5.0),
            _ => panic!("Expected Number result"),
        }
        
        // Test primitive message handling for strings
        let result = runtime.dispatch_message(Value::String("hello".to_string()), "size", vec![]);
        match result {
            Value::Number(n) => assert_eq!(n, 5.0),
            _ => panic!("Expected Number result"),
        }
        
        let result = runtime.dispatch_message(
            Value::String("hello".to_string()),
            "at",
            vec![Value::Number(1.0)],
        );
        match result {
            Value::String(s) => assert_eq!(s, "e"),
            _ => panic!("Expected String result"),
        }
    }
    
    #[test]
    fn test_object_slots_and_lookup() {
        let mut runtime = Runtime::new();
        
        // Create prototype and object
        let proto_id = runtime.memory.alloc_object(runtime.prototypes.object);
        let obj_id = runtime.memory.alloc_object(proto_id);
        
        // Set slot on prototype
        if let Some(proto) = runtime.memory.get_object_mut(proto_id) {
            proto.set_slot("proto_slot".to_string(), Value::Number(42.0));
        }
        
        // Set slot on object
        if let Some(obj) = runtime.memory.get_object_mut(obj_id) {
            obj.set_slot("obj_slot".to_string(), Value::Number(24.0));
        }
        
        // Test direct slot lookup
        if let Some(obj) = runtime.memory.get_object(obj_id) {
            match obj.get_slot("obj_slot") {
                Some(Value::Number(n)) => assert_eq!(*n, 24.0),
                _ => panic!("Expected Number in obj_slot"),
            }
        }
        
        // Test prototype slot lookup
        if let Some(obj) = runtime.memory.get_object(obj_id) {
            let proto_slot = obj.lookup_slot("proto_slot", &runtime);
            match proto_slot {
                Value::Number(n) => assert_eq!(n, 42.0),
                _ => panic!("Expected Number from proto_slot"),
            }
        }
        
        // Test override
        if let Some(obj) = runtime.memory.get_object_mut(obj_id) {
            obj.set_slot("proto_slot".to_string(), Value::Number(100.0));
        }
        
        if let Some(obj) = runtime.memory.get_object(obj_id) {
            let proto_slot = obj.lookup_slot("proto_slot", &runtime);
            match proto_slot {
                Value::Number(n) => assert_eq!(n, 100.0), // Should get overridden value
                _ => panic!("Expected overridden Number"),
            }
        }
    }
    
    #[test]
    fn test_runtime_reference_based() {
        // Create a runtime instance with Arc<Mutex<>>
        let runtime = Arc::new(Mutex::new(Runtime::new()));
        
        // Allocate a number in the runtime
        let num_id = {
            let mut rt = runtime.lock().unwrap();
            rt.alloc_number(42.0)
        };
        
        // Try to read the number back with runtime_encode_reference first to get a tagged value
        let tagged_value = {
            let rt = runtime.lock().unwrap();
            rt.runtime_encode_reference(num_id, TAG_OBJECT_REF)
        };
        
        // Then convert the tagged value back to a Runtime Value
        let value = {
            let rt = runtime.lock().unwrap();
            rt.from_tagged_value(tagged_value)
        };
        
        // Check that we got the right object value back
        match value {
            Value::Object(id) => {
                // Verify the object exists and is a number object
                let is_valid = {
                    let rt = runtime.lock().unwrap();
                    if let Some(obj) = rt.memory.get_object(id) {
                        match &obj.data {
                            ObjectData::Number(n) => n == &42.0,
                            _ => false,
                        }
                    } else {
                        false
                    }
                };
                assert!(is_valid, "Expected a Number object with value 42.0");
            },
            _ => panic!("Expected Object value, got {:?}", value),
        };
    }
    
    #[test]
    fn test_runtime_multiple_instances() {
        // Create two separate runtime instances
        let runtime1 = Arc::new(Mutex::new(Runtime::new()));
        let runtime2 = Arc::new(Mutex::new(Runtime::new()));
        
        // Get the prototype values we need first
        let proto1 = {
            let rt = runtime1.lock().unwrap();
            rt.prototypes.object
        };
        
        let proto2 = {
            let rt = runtime2.lock().unwrap();
            rt.prototypes.object
        };
        
        // Allocate objects in each runtime
        let obj1_id = {
            let mut rt = runtime1.lock().unwrap();
            rt.alloc_object(proto1)
        };
        
        let obj2_id = {
            let mut rt = runtime2.lock().unwrap();
            rt.alloc_object(proto2)
        };
        
        // Verify the objects exist in their respective runtimes
        {
            let rt1 = runtime1.lock().unwrap();
            assert!(rt1.memory.get_object(obj1_id).is_some());
            assert!(rt1.memory.get_object(obj2_id).is_none()); // Shouldn't exist in runtime1
        }
        
        {
            let rt2 = runtime2.lock().unwrap();
            assert!(rt2.memory.get_object(obj2_id).is_some());
            assert!(rt2.memory.get_object(obj1_id).is_none()); // Shouldn't exist in runtime2
        }
    }
    
    #[test]
    fn test_runtime_garbage_collection() {
        // Create a runtime instance
        let runtime = Arc::new(Mutex::new(Runtime::new()));
        
        // Get the prototype object id first
        let proto_obj = {
            let rt = runtime.lock().unwrap();
            rt.prototypes.object
        };
        
        // Create a reference from Lobby to an object to ensure it's a root
        let root_obj_id = {
            let mut rt = runtime.lock().unwrap();
            let obj_id = rt.alloc_object(proto_obj);
            
            // Get the Lobby ID first
            let lobby_id = rt.lobby;
            
            // Add the object to the Lobby to make it a root
            if let Some(lobby) = rt.memory.get_object_mut(lobby_id) {
                lobby.set_slot("test_object".to_string(), Value::Object(obj_id));
            }
            
            obj_id
        };
        
        // Create a nested object without a reference from any root
        let nested_id = {
            let mut rt = runtime.lock().unwrap();
            let id = rt.alloc_object(proto_obj);
            
            // The object exists but isn't reachable from any root
            assert!(rt.memory.get_object(id).is_some());
            
            id
        };
        
        // Run garbage collection
        {
            let mut rt = runtime.lock().unwrap();
            rt.collect_garbage();
            
            // The unreachable object should be gone
            assert!(rt.memory.get_object(nested_id).is_none(), "Unreachable object should have been collected");
            
            // But the object referenced from Lobby should still exist
            assert!(rt.memory.get_object(root_obj_id).is_some(), "Object referenced from Lobby should still exist");
        }
    }

    #[test]
    fn test_runtime_thread_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        // Create a shared runtime instance
        let runtime = Arc::new(Mutex::new(Runtime::new()));
        
        // Get the Object prototype for later use
        let obj_proto = {
            let rt = runtime.lock().unwrap();
            rt.prototypes.object
        };
        
        // Create an object in the main thread
        let main_obj_id = {
            let mut rt = runtime.lock().unwrap();
            let obj_id = rt.alloc_object(obj_proto);
            // Set a value on it
            if let Some(obj) = rt.memory.get_object_mut(obj_id) {
                obj.set_slot("main_thread".to_string(), Value::String("created".to_string()));
            }
            obj_id
        };
        
        // Spawn multiple threads to work with the same runtime
        let mut handles = vec![];
        
        for i in 0..5 {
            let rt_clone = Arc::clone(&runtime);
            let main_id = main_obj_id;
            
            let handle = thread::spawn(move || {
                // Each thread creates its own object
                let thread_obj_id = {
                    let mut rt = rt_clone.lock().unwrap();
                    rt.alloc_object(obj_proto)
                };
                
                // Set some data on the object
                {
                    let mut rt = rt_clone.lock().unwrap();
                    if let Some(obj) = rt.memory.get_object_mut(thread_obj_id) {
                        obj.set_slot("thread_id".to_string(), Value::Number(i as f64));
                    }
                }
                
                // Also modify the main thread's object to prove thread safety
                {
                    let mut rt = rt_clone.lock().unwrap();
                    if let Some(obj) = rt.memory.get_object_mut(main_id) {
                        let key = format!("thread_{}", i);
                        obj.set_slot(key, Value::Number(i as f64));
                    }
                }
                
                // Return the object ID for verification
                thread_obj_id
            });
            
            handles.push(handle);
        }
        
        // Collect all thread object IDs
        let thread_obj_ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        
        // Verify that all thread objects exist and have correct data
        {
            let rt = runtime.lock().unwrap();
            
            // Verify the main object has all the modifications from the threads
            if let Some(main_obj) = rt.memory.get_object(main_obj_id) {
                assert_eq!(
                    main_obj.lookup_slot("main_thread", &rt),
                    Value::String("created".to_string())
                );
                
                // Check that all thread modifications exist
                for i in 0..5 {
                    let key = format!("thread_{}", i);
                    let value = main_obj.lookup_slot(&key, &rt);
                    assert_eq!(value, Value::Number(i as f64));
                }
            } else {
                panic!("Main object not found");
            }
            
            // Verify all thread-created objects exist
            for (i, &obj_id) in thread_obj_ids.iter().enumerate() {
                if let Some(obj) = rt.memory.get_object(obj_id) {
                    let value = obj.lookup_slot("thread_id", &rt);
                    assert_eq!(value, Value::Number(i as f64));
                } else {
                    panic!("Thread object not found");
                }
            }
        }
    }
}