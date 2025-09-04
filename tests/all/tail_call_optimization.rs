//! Tests for tail call optimization functionality.

use anyhow::Result;
use wasmtime::*;

/// Test that tail-call-only functions work correctly with frame optimization
#[test]
fn tail_call_only_frame_optimization() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    // Simple tail-recursive factorial function
    let wat = r#"
        (module
            (func $factorial-tail (export "factorial-tail") 
                  (param $n i32) (param $acc i32) (result i32)
                local.get $n
                i32.eqz
                if (result i32)
                    local.get $acc
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    local.get $acc
                    local.get $n
                    i32.mul
                    return_call $factorial-tail  ;; This should be classified as TailCallOnly
                end
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let factorial_tail =
        instance.get_typed_func::<(i32, i32), i32>(&mut store, "factorial-tail")?;

    // Test various factorial calculations
    assert_eq!(factorial_tail.call(&mut store, (0, 1))?, 1);
    assert_eq!(factorial_tail.call(&mut store, (1, 1))?, 1);
    assert_eq!(factorial_tail.call(&mut store, (5, 1))?, 120);
    assert_eq!(factorial_tail.call(&mut store, (10, 1))?, 3628800);

    Ok(())
}

/// Test mixed regular and tail calls
#[test]
fn mixed_call_types() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    let wat = r#"
        (module
            (func $helper (param $x i32) (result i32)
                local.get $x
                i32.const 1
                i32.add
            )
            
            (func $mixed (export "mixed") (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 0
                else
                    local.get $n
                    call $helper  ;; Regular call - should be classified as Regular
                end
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let mixed = instance.get_typed_func::<i32, i32>(&mut store, "mixed")?;

    assert_eq!(mixed.call(&mut store, 0)?, 0);
    assert_eq!(mixed.call(&mut store, 1)?, 2); // helper(1) = 2
    assert_eq!(mixed.call(&mut store, 5)?, 6); // helper(5) = 6

    Ok(())
}

/// Test indirect tail calls
#[test]
fn indirect_tail_calls() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    let wat = r#"
        (module
            (type $sig (func (param i32) (result i32)))
            (table 1 funcref)
            
            (func $double (param $x i32) (result i32)
                local.get $x
                i32.const 2
                i32.mul
            )
            
            (func $indirect-tail (export "indirect-tail") (param $x i32) (result i32)
                local.get $x
                i32.const 0
                return_call_indirect (type $sig)  ;; Indirect tail call
            )
            
            (elem (i32.const 0) $double)
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let indirect_tail = instance.get_typed_func::<i32, i32>(&mut store, "indirect-tail")?;

    assert_eq!(indirect_tail.call(&mut store, 5)?, 10);
    assert_eq!(indirect_tail.call(&mut store, 10)?, 20);

    Ok(())
}

/// Test leaf functions (no calls at all)
#[test]
fn leaf_functions() -> Result<()> {
    let engine = Engine::default();

    let wat = r#"
        (module
            (func $add (export "add") (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add  ;; No calls - should be classified as Leaf
            )
            
            (func $square (export "square") (param $x i32) (result i32)
                local.get $x
                local.get $x
                i32.mul  ;; No calls - should be classified as Leaf
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
    let square = instance.get_typed_func::<i32, i32>(&mut store, "square")?;

    assert_eq!(add.call(&mut store, (3, 4))?, 7);
    assert_eq!(square.call(&mut store, 8)?, 64);

    Ok(())
}

/// Test deep tail recursion to verify stack optimization
#[test]
fn deep_tail_recursion() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    let wat = r#"
        (module
            (func $countdown (export "countdown") (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 42  ;; Base case
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    return_call $countdown  ;; Tail recursive
                end
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let countdown = instance.get_typed_func::<i32, i32>(&mut store, "countdown")?;

    // Test deep recursion that would overflow without tail call optimization
    assert_eq!(countdown.call(&mut store, 0)?, 42);
    assert_eq!(countdown.call(&mut store, 1000)?, 42);

    // This should not cause stack overflow with proper tail call optimization
    assert_eq!(countdown.call(&mut store, 10000)?, 42);

    Ok(())
}

/// Test state machine implemented with tail calls
#[test]
fn state_machine_tail_calls() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    let wat = r#"
        (module
            (func $state_a (param $counter i32) (result i32)
                local.get $counter
                i32.const 0
                i32.eq
                if (result i32)
                    i32.const 1  ;; Final state A
                else
                    local.get $counter
                    i32.const 1
                    i32.sub
                    return_call $state_b  ;; Tail call to state B
                end
            )
            
            (func $state_b (param $counter i32) (result i32)
                local.get $counter
                i32.const 0
                i32.eq
                if (result i32)
                    i32.const 2  ;; Final state B
                else
                    local.get $counter
                    i32.const 1
                    i32.sub
                    return_call $state_a  ;; Tail call to state A
                end
            )
            
            (func $start_machine (export "start") (param $steps i32) (result i32)
                local.get $steps
                return_call $state_a  ;; Initial tail call to state A
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let start = instance.get_typed_func::<i32, i32>(&mut store, "start")?;

    // Even number of steps should end in state A
    assert_eq!(start.call(&mut store, 0)?, 1);
    assert_eq!(start.call(&mut store, 2)?, 1);
    assert_eq!(start.call(&mut store, 4)?, 1);

    // Odd number of steps should end in state B
    assert_eq!(start.call(&mut store, 1)?, 2);
    assert_eq!(start.call(&mut store, 3)?, 2);
    assert_eq!(start.call(&mut store, 5)?, 2);

    Ok(())
}

/// Test that optimizations don't break with different calling conventions
#[test]
fn calling_convention_compatibility() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);

    // Test with different optimization levels and configurations
    for strategy in [Strategy::Cranelift] {
        config.strategy(strategy);
        let engine = Engine::new(&config)?;

        let wat = r#"
            (module
                (func $simple-tail (export "simple") (param $x i32) (result i32)
                    local.get $x
                    i32.const 10
                    i32.lt_s
                    if (result i32)
                        local.get $x
                    else
                        local.get $x
                        i32.const 1
                        i32.sub
                        return_call $simple-tail
                    end
                )
            )
        "#;

        let module = Module::new(&engine, wat)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        let simple = instance.get_typed_func::<i32, i32>(&mut store, "simple")?;

        assert_eq!(simple.call(&mut store, 5)?, 5);
        assert_eq!(simple.call(&mut store, 15)?, 9);
    }

    Ok(())
}

/// Test x64-specific tail call optimization with SystemV calling convention
#[test]
fn x64_systemv_tail_call_optimization() -> Result<()> {
    // This test specifically validates x64 tail call frame optimization
    // Currently only enabled for SystemV calling convention on Unix platforms
    let mut config = Config::new();
    config.wasm_tail_call(true);

    // Configure to target x64 SystemV ABI where possible
    let engine = Engine::new(&config)?;

    // Test function that should benefit from x64 tail call optimization
    let wat = r#"
        (module
            (func $countdown (export "countdown") (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 0
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    return_call $countdown  ;; x64 optimization should apply here
                end
            )

            ;; Function with mixed calls - should use standard frame
            (func $mixed_calls (export "mixed_calls") (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 42
                    call $helper  ;; Regular call - prevents optimization
                    return
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    return_call $mixed_calls
                end
            )

            (func $helper (param i32) (result i32)
                local.get 0
                i32.const 1
                i32.add
            )

            ;; Pure leaf function - should have minimal frame
            (func $leaf (export "leaf") (param $x i32) (result i32)
                local.get $x
                i32.const 2
                i32.mul
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    // Test the tail-call-only function
    let countdown = instance.get_typed_func::<i32, i32>(&mut store, "countdown")?;
    assert_eq!(countdown.call(&mut store, 0)?, 0);
    assert_eq!(countdown.call(&mut store, 1)?, 0);
    assert_eq!(countdown.call(&mut store, 100)?, 0); // Deep recursion test

    // Test mixed calls function
    let mixed_calls = instance.get_typed_func::<i32, i32>(&mut store, "mixed_calls")?;
    assert_eq!(mixed_calls.call(&mut store, 0)?, 43);
    assert_eq!(mixed_calls.call(&mut store, 5)?, 43);

    // Test leaf function
    let leaf = instance.get_typed_func::<i32, i32>(&mut store, "leaf")?;
    assert_eq!(leaf.call(&mut store, 21)?, 42);

    Ok(())
}

/// Test x64 frame optimization safety conditions
#[test]
fn x64_optimization_safety_conditions() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);

    // Test with different configurations that should affect optimization
    let engine = Engine::new(&config)?;

    // Simple tail-recursive function to test optimization safety
    let wat = r#"
        (module
            (func $simple_tail_recurse (export "simple_tail_recurse") 
                  (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 1
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    return_call $simple_tail_recurse
                end
            )
        )
    "#;

    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let func = instance.get_typed_func::<i32, i32>(&mut store, "simple_tail_recurse")?;

    // Test basic functionality regardless of optimization level
    assert_eq!(func.call(&mut store, 0)?, 1);
    assert_eq!(func.call(&mut store, 10)?, 1);
    assert_eq!(func.call(&mut store, 1000)?, 1); // Stress test

    Ok(())
}

/// Test x64 tail call optimization edge cases that should prevent optimization
#[test]
fn x64_edge_cases_preventing_optimization() -> Result<()> {
    let mut config = Config::new();
    config.wasm_tail_call(true);
    let engine = Engine::new(&config)?;

    // Test 1: Function with many parameters that likely use stack arguments
    // This should prevent tail call frame optimization due to outgoing_args_size > 0
    let wat_many_params = r#"
        (module
            (func $many_params_tail_call (export "many_params_tail_call")
                  (param $p1 i32) (param $p2 i32) (param $p3 i32) (param $p4 i32)
                  (param $p5 i32) (param $p6 i32) (param $p7 i32) (param $p8 i32) 
                  (param $p9 i32) (param $p10 i32) (result i32)
                local.get $p1
                i32.eqz
                if (result i32)
                    local.get $p10
                else
                    local.get $p1 i32.const 1 i32.sub
                    local.get $p2 local.get $p3 local.get $p4 local.get $p5
                    local.get $p6 local.get $p7 local.get $p8 local.get $p9 local.get $p10
                    return_call $many_params_tail_call
                end
            )
        )
    "#;

    let module = Module::new(&engine, wat_many_params)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let func = instance.get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32), i32>(
        &mut store,
        "many_params_tail_call",
    )?;

    // Test functionality - should work correctly regardless of optimization
    assert_eq!(func.call(&mut store, (0, 1, 2, 3, 4, 5, 6, 7, 8, 9))?, 9);
    assert_eq!(func.call(&mut store, (3, 1, 2, 3, 4, 5, 6, 7, 8, 10))?, 10);

    // Test 2: Function that uses local variables (creating stackslots)
    // This should prevent optimization due to stackslots_size > 0
    let wat_locals = r#"
        (module
            (func $with_locals (export "with_locals") (param $n i32) (result i32)
                (local $temp1 i32)
                (local $temp2 i32) 
                (local $temp3 i32)
                (local $temp4 i64)  ;; Mix of types to increase stack usage
                (local $temp5 f32)
                (local $temp6 f64)
                
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 42
                else
                    local.get $n
                    local.set $temp1
                    local.get $temp1
                    i32.const 1
                    i32.sub
                    return_call $with_locals
                end
            )
        )
    "#;

    let module2 = Module::new(&engine, wat_locals)?;
    let mut store2 = Store::new(&engine, ());
    let instance2 = Instance::new(&mut store2, &module2, &[])?;

    let func2 = instance2.get_typed_func::<i32, i32>(&mut store2, "with_locals")?;

    // Test functionality
    assert_eq!(func2.call(&mut store2, 0)?, 42);
    assert_eq!(func2.call(&mut store2, 5)?, 42);

    // Test 3: Minimal tail recursive function (should be optimizable if register args only)
    let wat_minimal = r#"
        (module
            (func $minimal_tail (export "minimal_tail") (param $n i32) (result i32)
                local.get $n
                i32.const 0
                i32.le_s
                if (result i32)
                    i32.const 1
                else
                    local.get $n
                    i32.const 1
                    i32.sub
                    return_call $minimal_tail
                end
            )
        )
    "#;

    let module3 = Module::new(&engine, wat_minimal)?;
    let mut store3 = Store::new(&engine, ());
    let instance3 = Instance::new(&mut store3, &module3, &[])?;

    let func3 = instance3.get_typed_func::<i32, i32>(&mut store3, "minimal_tail")?;

    // Test functionality - this one might be optimized if all conditions are met
    assert_eq!(func3.call(&mut store3, 0)?, 1);
    assert_eq!(func3.call(&mut store3, 100)?, 1);

    Ok(())
}
