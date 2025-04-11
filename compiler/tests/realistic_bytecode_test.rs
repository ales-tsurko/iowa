use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_iterative_fibonacci() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a fibonacci method with an iterative implementation to avoid stack overflow
    let method_source = r#"
        // Handle base cases
        if(n <= 0, return 0)
        if(n == 1, return 1)
        
        // Iterative solution using local variables
        a := 0
        b := 1
        i := 2
        
        // Implement while loop using recursion, since we don't have actual while loops
        whileLoop := method(
            if(i <= n,
                temp := a + b
                a := b
                b := temp
                i := i + 1
                whileLoop()
            )
        )
        whileLoop()
        
        return b
    "#;

    let method_id = runtime
        .create_method(vec!["n".to_string()], method_source)
        .unwrap();

    // Create a receiver object and assign the method
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("fibonacci".to_string(), Value::Object(method_id));
    }

    // Calculate fibonacci(6) which should be 8
    let args = vec![Value::Number(6.0)];
    runtime.push_call_frame(method_id, lobby, "fibonacci".to_string(), args.clone());

    // Call the method and check the result
    let result = runtime.dispatch_message(Value::Object(lobby), "fibonacci", args);
    assert_eq!(result, Value::Number(8.0));
}

#[test]
fn test_object_creation_and_manipulation() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a method that creates and manipulates an object
    let method_source = r#"
        // Create a new object
        person := Object clone
        
        // Set properties
        person setSlot("name", name)
        person setSlot("age", age)
        
        // Add a method to get full description
        person setSlot("description", method(
            return self getSlot("name") .. " is " .. self getSlot("age") .. " years old"
        ))
        
        // Add a method to have a birthday
        person setSlot("birthday", method(
            self setSlot("age", self getSlot("age") + 1)
            return self
        ))
        
        return person
    "#;

    let create_person_id = runtime
        .create_method(vec!["name".to_string(), "age".to_string()], method_source)
        .unwrap();

    // Assign the method to the Lobby
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("createPerson".to_string(), Value::Object(create_person_id));
    }

    // Call the method to create a person object
    let args = vec![Value::String("John".to_string()), Value::Number(30.0)];
    runtime.push_call_frame(
        create_person_id,
        lobby,
        "createPerson".to_string(),
        args.clone(),
    );
    let person = runtime.dispatch_message(Value::Object(lobby), "createPerson", args);

    // The person should be an object reference
    assert!(matches!(person, Value::Object(_)));

    // Get the description
    if let Value::Object(person_id) = person {
        // Call the description method
        runtime.push_call_frame(0, person_id, "description".to_string(), vec![]);
        let description = runtime.dispatch_message(Value::Object(person_id), "description", vec![]);
        assert_eq!(
            description,
            Value::String("John is 30 years old".to_string())
        );

        // Call the birthday method
        runtime.push_call_frame(0, person_id, "birthday".to_string(), vec![]);
        runtime.dispatch_message(Value::Object(person_id), "birthday", vec![]);

        // Get the description again to verify age increased
        runtime.push_call_frame(0, person_id, "description".to_string(), vec![]);
        let description_after =
            runtime.dispatch_message(Value::Object(person_id), "description", vec![]);
        assert_eq!(
            description_after,
            Value::String("John is 31 years old".to_string())
        );
    } else {
        panic!("Expected an object but got: {:?}", person);
    }
}

#[test]
fn test_complex_control_flow() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a method with complex nested if statements
    let method_source = r#"
        result := nil
        
        if(x > 10,
            if(y > 10,
                if(z > 10,
                    result = "all large",
                    result = "x and y large"
                ),
                if(z > 10,
                    result = "x and z large",
                    result = "only x large"
                )
            ),
            if(y > 10,
                if(z > 10,
                    result = "y and z large",
                    result = "only y large"
                ),
                if(z > 10,
                    result = "only z large",
                    result = "none large"
                )
            )
        )
        
        return result
    "#;

    let method_id = runtime
        .create_method(
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
            method_source,
        )
        .unwrap();

    // Assign the method to the Lobby
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("categorize".to_string(), Value::Object(method_id));
    }

    // Test case 1: all large
    let args1 = vec![
        Value::Number(15.0),
        Value::Number(15.0),
        Value::Number(15.0),
    ];
    runtime.push_call_frame(method_id, lobby, "categorize".to_string(), args1.clone());
    let result1 = runtime.dispatch_message(Value::Object(lobby), "categorize", args1);
    assert_eq!(result1, Value::String("all large".to_string()));

    // Test case 2: only x large
    let args2 = vec![Value::Number(15.0), Value::Number(5.0), Value::Number(5.0)];
    runtime.push_call_frame(method_id, lobby, "categorize".to_string(), args2.clone());
    let result2 = runtime.dispatch_message(Value::Object(lobby), "categorize", args2);
    assert_eq!(result2, Value::String("only x large".to_string()));

    // Test case 3: none large
    let args3 = vec![Value::Number(5.0), Value::Number(5.0), Value::Number(5.0)];
    runtime.push_call_frame(method_id, lobby, "categorize".to_string(), args3.clone());
    let result3 = runtime.dispatch_message(Value::Object(lobby), "categorize", args3);
    assert_eq!(result3, Value::String("none large".to_string()));
}

#[test]
fn test_complex_factorial() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a factorial method with error handling and optimization
    let method_source = r#"
        // Handle negative input
        if(n < 0,
            return nil // In real Io, we'd raise an exception
        )
        
        // Base cases
        if(n == 0, return 1)
        if(n == 1, return 1)
        
        // Optimization for small values to avoid deep recursion
        if(n == 2, return 2)
        if(n == 3, return 6)
        if(n == 4, return 24)
        if(n == 5, return 120)
        
        // Implement factorial iteratively to avoid stack overflow
        result := 1
        i := 1
        
        iterFactorial := method(
            while(i <= n,
                result := result * i
                i := i + 1
            )
        )
        
        // Helper method for the while loop
        while := method(condition, body,
            if(condition,
                body()
                while(condition, body)
            )
        )
        
        // Run the computation
        iterFactorial()
        return result
    "#;

    let method_id = runtime
        .create_method(vec!["n".to_string()], method_source)
        .unwrap();

    // Assign the method to the Lobby
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("factorial".to_string(), Value::Object(method_id));
    }

    // Test factorial(0)
    let args0 = vec![Value::Number(0.0)];
    runtime.push_call_frame(method_id, lobby, "factorial".to_string(), args0.clone());
    let result0 = runtime.dispatch_message(Value::Object(lobby), "factorial", args0);
    assert_eq!(result0, Value::Number(1.0));

    // Test factorial(5) - should use the optimized path
    let args5 = vec![Value::Number(5.0)];
    runtime.push_call_frame(method_id, lobby, "factorial".to_string(), args5.clone());
    let result5 = runtime.dispatch_message(Value::Object(lobby), "factorial", args5);
    assert_eq!(result5, Value::Number(120.0));

    // Test factorial(6) - should use recursion
    let args6 = vec![Value::Number(6.0)];
    runtime.push_call_frame(method_id, lobby, "factorial".to_string(), args6.clone());
    let result6 = runtime.dispatch_message(Value::Object(lobby), "factorial", args6);
    assert_eq!(result6, Value::Number(720.0));

    // Test factorial(-1) - should return nil
    let args_neg = vec![Value::Number(-1.0)];
    runtime.push_call_frame(method_id, lobby, "factorial".to_string(), args_neg.clone());
    let result_neg = runtime.dispatch_message(Value::Object(lobby), "factorial", args_neg);
    assert_eq!(result_neg, Value::Nil);
}

#[test]
fn test_method_composition() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a calculator object with multiple methods
    let calculator_id = runtime.alloc_object(runtime.prototypes().object);

    // Create a simple calculator object with methods
    let calc_source = r#"
        // Create a new calculator object
        calc := Object clone
        
        // Add basic methods
        calc add := method(a, b, return a + b)
        calc multiply := method(a, b, return a * b)
        calc square := method(x, return x * x)
        calc sumOfSquares := method(a, b, 
            aSquared := self square(a)
            bSquared := self square(b)
            return self add(aSquared, bSquared)
        )
        
        return calc
    "#;

    let create_calc_id = runtime.create_method(vec![], calc_source).unwrap();

    // Add method to Lobby
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot(
            "createCalculator".to_string(),
            Value::Object(create_calc_id),
        );
    }

    // Call method to create calculator
    runtime.push_call_frame(
        create_calc_id,
        lobby,
        "createCalculator".to_string(),
        vec![],
    );
    let calculator = runtime.dispatch_message(Value::Object(lobby), "createCalculator", vec![]);

    // The calculator should be an object
    assert!(matches!(calculator, Value::Object(_)));

    // Test the sumOfSquares method (3^2 + 4^2 = 9 + 16 = 25)
    let args = vec![Value::Number(3.0), Value::Number(4.0)];
    if let Value::Object(calc_id) = calculator {
        runtime.push_call_frame(0, calc_id, "sumOfSquares".to_string(), args.clone());
        let result = runtime.dispatch_message(Value::Object(calc_id), "sumOfSquares", args);
        assert_eq!(result, Value::Number(25.0));
    } else {
        panic!("Calculator is not an object");
    }
}

#[test]
fn test_loop_with_recursive_helper() {
    // Create a new runtime
    let mut runtime = Runtime::new();

    // Create a method that implements a loop using recursion
    let method_source = r#"
        // Helper function to implement the loop
        loopHelper := method(current, max, sum,
            if(current > max,
                return sum,
                return loopHelper(current + 1, max, sum + current)
            )
        )
        
        // Start the loop with initial state
        return loopHelper(1, max, 0)
    "#;

    let method_id = runtime
        .create_method(vec!["max".to_string()], method_source)
        .unwrap();

    // Assign the method to the Lobby
    let lobby = runtime.lobby();

    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("sumUpTo".to_string(), Value::Object(method_id));
    }

    // Test summing numbers from 1 to 5 = 15
    let args5 = vec![Value::Number(5.0)];
    runtime.push_call_frame(method_id, lobby, "sumUpTo".to_string(), args5.clone());
    let result5 = runtime.dispatch_message(Value::Object(lobby), "sumUpTo", args5);
    assert_eq!(result5, Value::Number(15.0)); // 1+2+3+4+5 = 15

    // Test summing numbers from 1 to 10 = 55
    let args10 = vec![Value::Number(10.0)];
    runtime.push_call_frame(method_id, lobby, "sumUpTo".to_string(), args10.clone());
    let result10 = runtime.dispatch_message(Value::Object(lobby), "sumUpTo", args10);
    assert_eq!(result10, Value::Number(55.0)); // Sum of 1 to 10 is 55
}
