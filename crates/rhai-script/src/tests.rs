use super::*;
use rhai::Dynamic;

/// Helper: evaluate a script with setup_engine() and return the Dynamic result.
fn eval(script: &str, scope: &mut rhai::Scope) -> Dynamic {
    let engine = setup_engine();
    engine
        .eval_with_scope::<Dynamic>(scope, script)
        .unwrap()
}

#[test]
fn test_engine() {
    let script = r#"
    let y = x * 2;
    x
    "#;
    let mut e = rhai::Engine::new();
    e.register_fn("*", |a: i128, b: i64| a * b as i128);
    let mut scope = rhai::Scope::new();
    scope.push("x", 10i128);
    let res = e.eval_with_scope::<Dynamic>(&mut scope, script).unwrap();
    let y = scope.get("y").unwrap();
    let _ = dbg!(res);
    dbg!(y);
}

#[test]
fn test_string_format() {
    let script = r#"`http://${x}.com`"#;
    let e = rhai::Engine::new();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("x", value_to_dynamic(Value::from("google")));
    let res = e.eval_with_scope::<Dynamic>(&mut scope, script).unwrap();
    let value = dynamic_to_value(res).unwrap();
    dbg!(value);
}

#[test]
fn test_map() {
    let script = r#"#{name: x, a: "12"}"#;
    let e = rhai::Engine::new();
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("x", value_to_dynamic(Value::from("google")));
    let res = e.eval_with_scope::<Dynamic>(&mut scope, script).unwrap();
    let value = dynamic_to_value(res).unwrap();
    dbg!(value);
}

// ── Template verification tests ──────────────────────────────────

#[test]
fn test_math_templates() {
    // Basic arithmetic
    let mut scope = rhai::Scope::new();
    scope.push("input", 10_i64);
    scope.push("input1", 5_i64);
    let res = eval("input + input1", &mut scope);
    assert_eq!(res.as_int().unwrap(), 15);

    // Multiply
    let mut scope = rhai::Scope::new();
    scope.push("input", 4_i64);
    scope.push("input1", 3_i64);
    let res = eval("input * input1", &mut scope);
    assert_eq!(res.as_int().unwrap(), 12);

    // Percentage
    let mut scope = rhai::Scope::new();
    scope.push("input", 200.0_f64);
    scope.push("input1", 15.0_f64);
    let res = eval("input * input1 / 100.0", &mut scope);
    assert_eq!(res.as_float().unwrap(), 30.0);

    // Decimal arithmetic
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"let a = Decimal("100.50"); let b = Decimal("0.25"); a * b"#,
        &mut scope,
    );
    assert!(res.as_decimal().is_ok());

    // Clamp (min/max pattern)
    let mut scope = rhai::Scope::new();
    scope.push("input", 150_i64);
    let res = eval(
        r#"
        let min_val = 0;
        let max_val = 100;
        if input < min_val { min_val } else if input > max_val { max_val } else { input }
        "#,
        &mut scope,
    );
    assert_eq!(res.as_int().unwrap(), 100);

    // Random — API is rand::rand(min, max), not rand::int
    let mut scope = rhai::Scope::new();
    let res = eval("rand::rand(1, 100)", &mut scope);
    let n = res.as_int().unwrap();
    assert!((1..=100).contains(&n));
}

#[test]
fn test_string_templates() {
    // Concatenation
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("hello")));
    scope.push_dynamic("input1", value_to_dynamic(Value::from(" world")));
    let res = eval("input + input1", &mut scope);
    assert_eq!(res.into_string().unwrap(), "hello world");

    // String interpolation
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("Alice")));
    let res = eval(r#"`Hello, ${input}!`"#, &mut scope);
    assert_eq!(res.into_string().unwrap(), "Hello, Alice!");

    // Split
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("a,b,c")));
    let res = eval(r#"input.split(",")"#, &mut scope);
    let arr = res.into_array().unwrap();
    assert_eq!(arr.len(), 3);

    // Trim — Rhai's trim() is in-place (returns unit), so must read from scope
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("  hello  ")));
    let _ = eval("input.trim()", &mut scope);
    let trimmed = scope.get_value::<rhai::ImmutableString>("input").unwrap();
    assert_eq!(trimmed.as_str(), "hello");

    // Replace — also in-place
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("hello world")));
    let _ = eval(r#"input.replace("world", "rhai")"#, &mut scope);
    let replaced = scope.get_value::<rhai::ImmutableString>("input").unwrap();
    assert_eq!(replaced.as_str(), "hello rhai");

    // Case conversion — to_upper/to_lower return new strings (not in-place)
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("Hello")));
    let res = eval("input.to_upper()", &mut scope);
    assert_eq!(res.into_string().unwrap(), "HELLO");

    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("Hello")));
    let res = eval("input.to_lower()", &mut scope);
    assert_eq!(res.into_string().unwrap(), "hello");
}

#[test]
fn test_array_templates() {
    // Map
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "input",
        value_to_dynamic(Value::Array(vec![
            Value::I64(1),
            Value::I64(2),
            Value::I64(3),
        ])),
    );
    let res = eval("input.map(|x| x * 2)", &mut scope);
    let arr = res.into_array().unwrap();
    assert_eq!(arr[0].as_int().unwrap(), 2);
    assert_eq!(arr[2].as_int().unwrap(), 6);

    // Filter
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "input",
        value_to_dynamic(Value::Array(vec![
            Value::I64(1),
            Value::I64(2),
            Value::I64(3),
            Value::I64(4),
        ])),
    );
    let res = eval("input.filter(|x| x > 2)", &mut scope);
    let arr = res.into_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Reduce
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "input",
        value_to_dynamic(Value::Array(vec![
            Value::I64(1),
            Value::I64(2),
            Value::I64(3),
        ])),
    );
    let res = eval("input.reduce(|sum, x| sum + x, 0)", &mut scope);
    assert_eq!(res.as_int().unwrap(), 6);

    // Find
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "input",
        value_to_dynamic(Value::Array(vec![
            Value::I64(10),
            Value::I64(20),
            Value::I64(30),
        ])),
    );
    let res = eval("input.find(|x| x > 15)", &mut scope);
    assert_eq!(res.as_int().unwrap(), 20);

    // Sort — Rhai's sort() is in-place (returns unit), so must read from scope
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "input",
        value_to_dynamic(Value::Array(vec![
            Value::I64(3),
            Value::I64(1),
            Value::I64(2),
        ])),
    );
    let _ = eval(
        "input.sort(|a, b| if a < b { -1 } else if a > b { 1 } else { 0 })",
        &mut scope,
    );
    let sorted = scope.get_value::<rhai::Array>("input").unwrap();
    assert_eq!(sorted[0].as_int().unwrap(), 1);
    assert_eq!(sorted[2].as_int().unwrap(), 3);
}

#[test]
fn test_json_templates() {
    // Map literal and field access
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("test")));
    let res = eval(r#"#{name: input, value: 42}"#, &mut scope);
    let map = res.cast::<rhai::Map>();
    assert_eq!(
        map.get("name").unwrap().clone().into_string().unwrap(),
        "test"
    );
    assert_eq!(map.get("value").unwrap().as_int().unwrap(), 42);

    // Keys
    let mut scope = rhai::Scope::new();
    let res = eval(r#"let m = #{a: 1, b: 2}; m.keys()"#, &mut scope);
    let arr = res.into_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Values
    let mut scope = rhai::Scope::new();
    let res = eval(r#"let m = #{a: 1, b: 2}; m.values()"#, &mut scope);
    let arr = res.into_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Contains key
    let mut scope = rhai::Scope::new();
    let res = eval(r#"let m = #{a: 1}; m.contains("a")"#, &mut scope);
    assert!(res.as_bool().unwrap());

    // Merge pattern (iterate + set)
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"
        let a = #{x: 1};
        let b = #{y: 2};
        for key in b.keys() { a[key] = b[key]; }
        a
        "#,
        &mut scope,
    );
    let map = res.cast::<rhai::Map>();
    assert_eq!(map.get("x").unwrap().as_int().unwrap(), 1);
    assert_eq!(map.get("y").unwrap().as_int().unwrap(), 2);
}

#[test]
fn test_conditional_templates() {
    // If/else
    let mut scope = rhai::Scope::new();
    scope.push("input", 10_i64);
    let res = eval(r#"if input > 5 { "big" } else { "small" }"#, &mut scope);
    assert_eq!(res.into_string().unwrap(), "big");

    // Switch
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("B")));
    let res = eval(
        r#"
        switch input {
            "A" => 1,
            "B" => 2,
            "C" => 3,
            _ => 0,
        }
        "#,
        &mut scope,
    );
    assert_eq!(res.as_int().unwrap(), 2);

    // Coalesce pattern (check for unit)
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", Dynamic::UNIT);
    scope.push("input1", 42_i64);
    let res = eval(
        r#"if input == () { input1 } else { input }"#,
        &mut scope,
    );
    assert_eq!(res.as_int().unwrap(), 42);
}

#[test]
fn test_type_templates() {
    // type_of
    let mut scope = rhai::Scope::new();
    let res = eval(r#"type_of(42)"#, &mut scope);
    assert_eq!(res.into_string().unwrap(), "i64");

    // parse_int
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("123")));
    let res = eval("parse_int(input)", &mut scope);
    assert_eq!(res.as_int().unwrap(), 123);

    // parse_float
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", value_to_dynamic(Value::from("3.19")));
    let res = eval("parse_float(input)", &mut scope);
    let f = res.as_float().unwrap();
    assert!((f - 3.19).abs() < f64::EPSILON);

    // to_string
    let mut scope = rhai::Scope::new();
    scope.push("input", 42_i64);
    let res = eval(r#"`${input}`"#, &mut scope);
    assert_eq!(res.into_string().unwrap(), "42");

    // is_null check via unit comparison
    let mut scope = rhai::Scope::new();
    scope.push_dynamic("input", Dynamic::UNIT);
    let res = eval("input == ()", &mut scope);
    assert!(res.as_bool().unwrap());
}

#[test]
fn test_datetime_templates() {
    // utc_now returns a non-empty string
    let mut scope = rhai::Scope::new();
    let res = eval("utc_now()", &mut scope);
    let s = res.into_string().unwrap();
    assert!(!s.is_empty());
    assert!(s.contains("UTC"));

    // Add timestamp to data
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"
        let data = #{name: "test"};
        data.timestamp = utc_now();
        data
        "#,
        &mut scope,
    );
    let map = res.cast::<rhai::Map>();
    assert!(map.contains_key("timestamp"));
}

#[test]
fn test_crypto_templates() {
    // lamports_to_sol: input * Decimal("0.000000001")
    let mut scope = rhai::Scope::new();
    scope.push("input", 1_000_000_000_i64);
    let res = eval(
        r#"Decimal(input) * Decimal("0.000000001")"#,
        &mut scope,
    );
    let dec = res.as_decimal().unwrap();
    assert_eq!(dec, rust_decimal::Decimal::new(1, 0)); // 1.0 SOL

    // sol_to_lamports: input * Decimal("1000000000")
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"Decimal("2.5") * Decimal("1000000000")"#,
        &mut scope,
    );
    let dec = res.as_decimal().unwrap();
    assert_eq!(dec, rust_decimal::Decimal::new(2_500_000_000, 0));

    // token_amount with decimals
    let mut scope = rhai::Scope::new();
    scope.push("input", 1000000_i64);
    scope.push("input1", 6_i64);
    let res = eval(
        r#"
        let amount = Decimal(input);
        let decimals = input1;
        let divisor = Decimal("1");
        for i in 0..decimals { divisor = divisor * Decimal("10"); }
        amount / divisor
        "#,
        &mut scope,
    );
    let dec = res.as_decimal().unwrap();
    assert_eq!(dec, rust_decimal::Decimal::new(1, 0)); // 1.0 token
}

// ── Structured error tests ───────────────────────────────────────

#[test]
fn test_script_error_captures_position() {
    let engine = setup_engine();
    let mut scope = rhai::Scope::new();
    // Line 2 has an undefined variable
    let script = "let x = 1;\nlet y = undefined_var + x;";
    let err = engine
        .eval_with_scope::<Dynamic>(&mut scope, script)
        .unwrap_err();
    let script_err = ScriptError::from(err);
    assert_eq!(script_err.error_type, "VariableNotFound");
    assert_eq!(script_err.line, Some(2));
    assert!(script_err.column.is_some());
    // Display includes line info
    let display = format!("{script_err}");
    assert!(display.contains("line 2"));
}

#[test]
fn test_script_error_parse_error() {
    let engine = setup_engine();
    let mut scope = rhai::Scope::new();
    let script = "let x = {;"; // invalid syntax
    let err = engine
        .eval_with_scope::<Dynamic>(&mut scope, script)
        .unwrap_err();
    let script_err = ScriptError::from(err);
    assert_eq!(script_err.error_type, "ParseError");
    assert!(script_err.line.is_some());
}

// ── Standard library tests ───────────────────────────────────────

#[test]
fn test_base58_roundtrip() {
    let mut scope = rhai::Scope::new();
    // Encode bytes to base58, then decode back
    let res = eval(
        r#"
        let bytes = blob(32, 0xff);
        let encoded = base58_encode(bytes);
        let decoded = base58_decode(encoded);
        decoded.len() == 32
        "#,
        &mut scope,
    );
    assert!(res.as_bool().unwrap());
}

#[test]
fn test_hex_roundtrip() {
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"
        let bytes = blob(4, 0xab);
        let encoded = hex_encode(bytes);
        let decoded = hex_decode(encoded);
        decoded == bytes
        "#,
        &mut scope,
    );
    assert!(res.as_bool().unwrap());

    // Verify the hex string format
    let mut scope = rhai::Scope::new();
    let res = eval(r#"hex_encode(blob(2, 0xff))"#, &mut scope);
    assert_eq!(res.into_string().unwrap(), "ffff");
}

#[test]
fn test_json_roundtrip() {
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"
        let data = #{name: "test", value: 42, active: true};
        let json_str = json_encode(data);
        let parsed = json_decode(json_str);
        parsed.name
        "#,
        &mut scope,
    );
    assert_eq!(res.into_string().unwrap(), "test");
}

#[test]
fn test_json_decode_array() {
    let mut scope = rhai::Scope::new();
    let res = eval(
        r#"
        let arr = json_decode("[1, 2, 3]");
        arr.len()
        "#,
        &mut scope,
    );
    assert_eq!(res.as_int().unwrap(), 3);
}

#[test]
fn test_json_decode_integers_are_i64() {
    // Verify decoded JSON integers work with arithmetic (not Decimal)
    let mut scope = rhai::Scope::new();
    scope.push_dynamic(
        "json_str",
        value_to_dynamic(Value::from(r#"{"x": 10, "y": 20}"#)),
    );
    let res = eval(
        "let data = json_decode(json_str); data.x + data.y + 5",
        &mut scope,
    );
    assert_eq!(res.as_int().unwrap(), 35);
}

#[test]
fn test_base58_decode_error() {
    let engine = setup_engine();
    let mut scope = rhai::Scope::new();
    let result = engine.eval_with_scope::<Dynamic>(
        &mut scope,
        r#"base58_decode("invalid!@#$")"#,
    );
    assert!(result.is_err());
}
