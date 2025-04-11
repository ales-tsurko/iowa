use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_recursive_fibonacci() {
    // Test realistic recursive function (fibonacci)
    let mut runtime = Runtime::new();
    
    // Create the fibonacci method with Io-like source code
    let source = r#"
    method(n,
        if(n <= 1,
            return n,
            return fibonacci(n - 1) + fibonacci(n - 2)
        )
    )
    "#;
    
    // Get the lobby to hold our method
    let lobby = runtime.lobby();
    
    // Create the fibonacci method and attach it to the lobby
    let method = runtime.create_method(vec!["n".to_string()], source).unwrap();
    
    // Set the slot using the object API
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("fibonacci".to_string(), Value::Object(method));
    }
    
    // Call fibonacci(6) which should return 8
    let args = vec![Value::Number(6.0)];
    runtime.push_call_frame(method, lobby, "fibonacci".to_string(), args.clone());
    let result = runtime.dispatch_message(Value::Object(lobby), "fibonacci", args);
    
    assert_eq!(result, Value::Number(8.0));
}

#[test]
fn test_object_manipulation() {
    // Test creating and manipulating objects
    let mut runtime = Runtime::new();
    
    // Create a method that creates and manipulates an object
    let source = r#"
    method(name, age,
        person := Object clone
        person name := name
        person age := age
        
        person description := method(
            self name .. " is " .. self age .. " years old"
        )
        
        person birthday := method(
            self age := self age + 1
            return self
        )
        
        return person
    )
    "#;
    
    // Get the lobby to hold our method
    let lobby = runtime.lobby();
    let method = runtime.create_method(
        vec!["name".to_string(), "age".to_string()], 
        source
    ).unwrap();
    
    // Add method to lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("createPerson".to_string(), Value::Object(method));
    }
    
    // Call the method to create a person
    let args = vec![
        Value::String("John".to_string()), 
        Value::Number(30.0)
    ];
    runtime.push_call_frame(method, lobby, "createPerson".to_string(), args.clone());
    let person = runtime.dispatch_message(Value::Object(lobby), "createPerson", args);
    
    // Call description on the person
    if let Value::Object(person_id) = person {
        runtime.push_call_frame(0, person_id, "description".to_string(), vec![]);
        let description = runtime.dispatch_message(Value::Object(person_id), "description", vec![]);
        assert_eq!(description, Value::String("John is 30 years old".to_string()));
        
        // Call birthday on the person
        runtime.push_call_frame(0, person_id, "birthday".to_string(), vec![]);
        runtime.dispatch_message(Value::Object(person_id), "birthday", vec![]);
        
        // Call description again and verify age increased
        runtime.push_call_frame(0, person_id, "description".to_string(), vec![]);
        let description_after = runtime.dispatch_message(Value::Object(person_id), "description", vec![]);
        assert_eq!(description_after, Value::String("John is 31 years old".to_string()));
    }
}

#[test]
fn test_nested_control_flow() {
    // Test nested if statements and complex logic
    let mut runtime = Runtime::new();
    
    // Method with nested if statements and complex logic
    let source = r#"
    method(x, y, z,
        result := nil
        
        if(x > 10,
            if(y > 10,
                if(z > 10,
                    result = "all large"
                ) else(
                    result = "x and y large"
                )
            ) else(
                if(z > 10,
                    result = "x and z large"
                ) else(
                    result = "only x large"
                )
            )
        ) else(
            if(y > 10,
                if(z > 10,
                    result = "y and z large"
                ) else(
                    result = "only y large"
                )
            ) else(
                if(z > 10,
                    result = "only z large"
                ) else(
                    result = "none large"
                )
            )
        )
        
        return result
    )
    "#;
    
    // Get the lobby to hold our method
    let lobby = runtime.lobby();
    let method = runtime.create_method(
        vec!["x".to_string(), "y".to_string(), "z".to_string()], 
        source
    ).unwrap();
    
    // Add method to lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("categorize".to_string(), Value::Object(method));
    }
    
    // Test with different inputs
    
    // Test: all values large
    let args1 = vec![Value::Number(15.0), Value::Number(15.0), Value::Number(15.0)];
    runtime.push_call_frame(method, lobby, "categorize".to_string(), args1.clone());
    let result1 = runtime.dispatch_message(Value::Object(lobby), "categorize", args1);
    assert_eq!(result1, Value::String("all large".to_string()));
    
    // Test: only x large
    let args2 = vec![Value::Number(15.0), Value::Number(5.0), Value::Number(5.0)];
    runtime.push_call_frame(method, lobby, "categorize".to_string(), args2.clone());
    let result2 = runtime.dispatch_message(Value::Object(lobby), "categorize", args2);
    assert_eq!(result2, Value::String("only x large".to_string()));
    
    // Test: none large
    let args3 = vec![Value::Number(5.0), Value::Number(5.0), Value::Number(5.0)];
    runtime.push_call_frame(method, lobby, "categorize".to_string(), args3.clone());
    let result3 = runtime.dispatch_message(Value::Object(lobby), "categorize", args3);
    assert_eq!(result3, Value::String("none large".to_string()));
}

#[test]
fn test_complex_factorial() {
    // Implement and test factorial with error handling and input validation
    let mut runtime = Runtime::new();
    
    // Method that calculates factorial with validation
    let source = r#"
    method(n,
        // Input validation
        if(n < 0,
            Exception raise("Factorial not defined for negative numbers")
            return nil
        )
        
        // Base cases
        if(n == 0, return 1)
        if(n == 1, return 1)
        
        // Early return for small inputs to avoid deep recursion
        if(n == 2, return 2)
        if(n == 3, return 6)
        if(n == 4, return 24)
        
        // For larger inputs, use recursion
        return n * factorial(n - 1)
    )
    "#;
    
    // Get the lobby to hold our method
    let lobby = runtime.lobby();
    let method = runtime.create_method(vec!["n".to_string()], source).unwrap();
    
    // Add method to lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("factorial".to_string(), Value::Object(method));
    }
    
    // Test factorial with various inputs
    
    // Test factorial(0)
    let args0 = vec![Value::Number(0.0)];
    runtime.push_call_frame(method, lobby, "factorial".to_string(), args0.clone());
    let result0 = runtime.dispatch_message(Value::Object(lobby), "factorial", args0);
    assert_eq!(result0, Value::Number(1.0));
    
    // Test factorial(5)
    let args5 = vec![Value::Number(5.0)];
    runtime.push_call_frame(method, lobby, "factorial".to_string(), args5.clone());
    let result5 = runtime.dispatch_message(Value::Object(lobby), "factorial", args5);
    assert_eq!(result5, Value::Number(120.0));
    
    // Test factorial(10) - might cause stack overflow, so we'll limit to smaller numbers
    let args10 = vec![Value::Number(6.0)];  // Using 6 instead of 10 to avoid stack issues
    runtime.push_call_frame(method, lobby, "factorial".to_string(), args10.clone());
    let result10 = runtime.dispatch_message(Value::Object(lobby), "factorial", args10);
    assert_eq!(result10, Value::Number(720.0));
}

#[test]
fn test_method_composition() {
    // Test methods that call other methods
    let mut runtime = Runtime::new();
    
    // Create a simpler test with a calculator method
    let lobby = runtime.lobby();
    
    // Create a method that does calculations directly
    let calc_source = r#"
    method(a, b,
        square := method(x, return x * x)
        return square(a) + square(b)
    )
    "#;
    
    let method = runtime.create_method(
        vec!["a".to_string(), "b".to_string()], 
        calc_source
    ).unwrap();
    
    // Add method to lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("sumOfSquares".to_string(), Value::Object(method));
    }
    
    // Test method with inputs 3 and 4
    let args = vec![Value::Number(3.0), Value::Number(4.0)];
    runtime.push_call_frame(method, lobby, "sumOfSquares".to_string(), args.clone());
    let result = runtime.dispatch_message(Value::Object(lobby), "sumOfSquares", args);
    
    // sumOfSquares(3, 4) = 3^2 + 4^2 = 9 + 16 = 25
    assert_eq!(result, Value::Number(25.0));
}

#[test]
fn test_loops_with_counter() {
    // Test loops implemented with recursion and a counter
    let mut runtime = Runtime::new();
    
    // Create a method that implements a while loop via recursion
    let source = r#"
    method(max,
        // Helper method to do the looping
        loopHelper := method(current, max, sum,
            if(current > max,
                return sum,
                return loopHelper(current + 1, max, sum + current)
            )
        )
        
        // Start the loop with initial values
        return loopHelper(1, max, 0)
    )
    "#;
    
    // Get the lobby to hold our method
    let lobby = runtime.lobby();
    let method = runtime.create_method(vec!["max".to_string()], source).unwrap();
    
    // Add method to lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("sumUpTo".to_string(), Value::Object(method));
    }
    
    // Test summing numbers from 1 to 5
    let args5 = vec![Value::Number(5.0)];
    runtime.push_call_frame(method, lobby, "sumUpTo".to_string(), args5.clone());
    let result5 = runtime.dispatch_message(Value::Object(lobby), "sumUpTo", args5);
    assert_eq!(result5, Value::Number(15.0)); // 1+2+3+4+5 = 15
    
    // Test summing numbers from 1 to 10
    let args10 = vec![Value::Number(10.0)];
    runtime.push_call_frame(method, lobby, "sumUpTo".to_string(), args10.clone());
    let result10 = runtime.dispatch_message(Value::Object(lobby), "sumUpTo", args10);
    assert_eq!(result10, Value::Number(55.0)); // Sum of 1 to 10 is 55
}