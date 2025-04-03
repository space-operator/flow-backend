use anyhow::anyhow;
use std::io::{Error, ErrorKind};

use chrono::Utc;
use convert::{dynamic_to_value, value_to_dynamic};
use flow_lib::command::prelude::*;
use rhai::{
    packages::{Package, StandardPackage},
    Dynamic, EvalAltResult,
};
use rhai_rand::RandomPackage;

pub mod convert;

pub use rhai::Engine;
use tracing::info;

fn utc_now() -> String {
    Utc::now().to_string()
}

fn decimal(x: Dynamic) -> Result<Decimal, Box<EvalAltResult>> {
    let value = dynamic_to_value(x)
        .map_err(|error| EvalAltResult::ErrorSystem("convert error".to_owned(), Box::new(error)))?;
    let decimal = value::decimal::deserialize(value)
        .map_err(|error| EvalAltResult::ErrorSystem("convert error".to_owned(), Box::new(error)))?;
    Ok(decimal)
}

fn map_to_blob(map_obj: Dynamic) -> Result<Dynamic, Box<EvalAltResult>> {
    let value = dynamic_to_value(map_obj).map_err(|error| {
        Box::new(EvalAltResult::ErrorSystem(
            "convert error".to_owned(),
            Box::new(error),
        ))
    })?;

    if let Value::Map(map) = value {
        let mut bytes = vec![0u8; 32]; // Initialize with zeroes

        for (key, value) in map.iter() {
            if let Ok(index) = key.to_string().parse::<usize>() {
                if index < bytes.len() {
                    // Just try to extract a number from any value type
                    let byte_value = match value {
                        Value::U64(n) => *n as u8,
                        Value::I64(n) => *n as u8,
                        Value::Decimal(n) => {
                            if let Ok(num) = n.to_string().parse::<u8>() {
                                num
                            } else {
                                info!("Failed to parse decimal value at index {}: {}", index, n);
                                0
                            }
                        }
                        _ => {
                            // Log the unknown type and its debug representation
                            info!(
                                "Unknown value type at index {}: {:?} (type: {})",
                                index,
                                value,
                                std::any::type_name_of_val(value)
                            );

                            // Try to parse from debug representation
                            let debug_str = format!("{:?}", value);

                            // Extract numbers from debug string
                            if let Some(num_str) = debug_str
                                .trim_start_matches(|c: char| !c.is_digit(10))
                                .split(|c: char| !c.is_digit(10))
                                .next()
                            {
                                let num = num_str.parse::<u8>().unwrap_or(0);
                                num
                            } else {
                                info!("Failed to extract number from {:?}", debug_str);
                                0
                            }
                        }
                    };
                    bytes[index] = byte_value;
                }
            }
        }
        return Ok(Dynamic::from(rhai::Blob::from(bytes)));
    }

    Err(Box::new(EvalAltResult::ErrorSystem(
        "Expected a map with numeric keys".to_owned(),
        Box::new(Error::new(ErrorKind::InvalidData, "Invalid map structure")),
    )))
}

fn json_str_to_blob(json_str: Dynamic) -> Result<Dynamic, Box<EvalAltResult>> {
    // Get the JSON string
    let json_string = json_str.into_string().map_err(|_| {
        Box::new(EvalAltResult::ErrorSystem(
            "Expected a JSON string".to_owned(),
            Box::new(Error::new(ErrorKind::InvalidData, "Invalid JSON string")),
        ))
    })?;

    // Use the engine to parse the JSON string into a map
    let engine = Engine::new();
    let map = engine.parse_json(&json_string, true).map_err(|e| {
        Box::new(EvalAltResult::ErrorSystem(
            "Failed to parse JSON".to_owned(),
            Box::new(e),
        ))
    })?;

    // Now process the map using map_to_blob
    map_to_blob(Dynamic::from(map))
}

fn blob_to_string(bytes: Dynamic) -> Result<String, Box<EvalAltResult>> {
    // First, check if it's a blob
    if bytes.is::<rhai::Blob>() {
        let blob = bytes.cast::<rhai::Blob>();
        return String::from_utf8(blob.to_vec()).map_err(|e| {
            Box::new(EvalAltResult::ErrorSystem(
                "Invalid UTF-8".to_owned(),
                Box::new(e),
            ))
        });
    }

    // If not a blob, return error
    Err(Box::new(EvalAltResult::ErrorSystem(
        "Expected a blob".to_owned(),
        Box::new(Error::new(ErrorKind::InvalidData, "Invalid blob")),
    )))
}

fn trim_null_bytes(s: Dynamic) -> Result<String, Box<EvalAltResult>> {
    let string = s.into_string().map_err(|_| {
        Box::new(EvalAltResult::ErrorSystem(
            "Expected a string".to_owned(),
            Box::new(Error::new(ErrorKind::InvalidData, "Invalid string")),
        ))
    })?;

    Ok(string.trim_end_matches(char::from(0)).to_string())
}

// convenience all in one, RPC response in bytes to a string
fn bytes_map_to_string(map_obj: Dynamic) -> Result<String, Box<EvalAltResult>> {
    // convert map to blob
    let blob = map_to_blob(map_obj)?;
    // convert blob to string
    let string = blob_to_string(blob)?;

    // trim null bytes
    Ok(string.trim_end_matches(char::from(0)).to_string())
}

pub fn setup_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_global_module(StandardPackage::new().as_shared_module())
        .register_static_module("rand", RandomPackage::new().as_shared_module())
        .register_fn("utc_now", utc_now)
        .register_fn("Decimal", decimal)
        .register_fn("blob_to_string", blob_to_string)
        .register_fn("json_str_to_blob", json_str_to_blob)
        .register_fn("map_to_blob", map_to_blob)
        .register_fn("trim_null_bytes", trim_null_bytes)
        .register_fn("bytes_map_to_string", bytes_map_to_string)
        .set_max_expr_depths(32, 32)
        .set_max_call_levels(256)
        .set_max_operations(10_000_000)
        .set_max_string_size(50_000)
        .set_max_array_size(10_000)
        .set_max_map_size(10_000)
        .set_max_variables(50);
    engine
}

pub const COMMAND_ID_PREFIX: &str = "rhai_script_";

pub fn is_rhai_script(s: &str) -> bool {
    s.starts_with(COMMAND_ID_PREFIX)
}

pub struct Command {
    pub source_code_name: Name,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

impl Command {
    pub fn run(
        &self,
        engine: &mut Engine,
        ctx: Context,
        mut input: ValueSet,
    ) -> Result<ValueSet, CommandError> {
        let code = String::deserialize(
            input
                .swap_remove(&self.source_code_name)
                .ok_or_else(|| anyhow!("missing input: {}", self.source_code_name))?,
        )?;

        let mut scope = rhai::Scope::new();

        let rhai_env = ctx
            .environment
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect::<rhai::Map>();
        scope.push_constant_dynamic("ENV", rhai_env.into());

        for i in &self.inputs {
            if i.name == self.source_code_name {
                continue;
            }
            match input.swap_remove(&i.name) {
                Some(value) => {
                    scope.push_dynamic(&i.name, value_to_dynamic(value));
                }
                None => {
                    if i.required {
                        tracing::warn!("missing input: {}", i.name);
                    } else {
                        scope.push_dynamic(&i.name, Dynamic::UNIT);
                    }
                }
            }
        }
        let eval_result = engine
            .eval_with_scope::<Dynamic>(&mut scope, &code)
            .map_err(|error| anyhow!(error.to_string()))?;
        let mut outputs = ValueSet::new();
        for o in &self.outputs {
            let dy = match scope.remove(&o.name) {
                Some(x) => x,
                None => {
                    if o.optional && self.outputs.len() > 1 {
                        tracing::debug!("missing output: {}", o.name);
                    }
                    continue;
                }
            };
            let value = dynamic_to_value(dy).map_err(|error| anyhow!("{:?}: {}", o.name, error))?;
            if !matches!(value, Value::Null) {
                outputs.insert(o.name.clone(), value);
            }
        }
        if outputs.is_empty() && self.outputs.len() == 1 {
            let name = self.outputs[0].name.clone();
            let value = dynamic_to_value(eval_result).map_err(|e| anyhow!("{:?}: {}", name, e))?;
            if !matches!(value, Value::Null) {
                outputs.insert(name, value);
            }
        }
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Dynamic;

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

    #[test]
    fn test_blob_to_string() {
        let engine = setup_engine();
        let mut scope = rhai::Scope::new();

        // Create a blob with "hello" in UTF-8
        let hello_bytes = rhai::Blob::from(vec![104, 101, 108, 108, 111]);
        scope.push("bytes", Dynamic::from(hello_bytes));

        let script = "blob_to_string(bytes)";
        let res = engine
            .eval_with_scope::<String>(&mut scope, script)
            .unwrap();
        assert_eq!(res, "hello");
    }

    #[test]
    fn test_parse_json_name() {
        let engine = setup_engine();
        let mut scope = rhai::Scope::new();

        // Example "Test vault"
        let json = r#"{
                "0": 84,
                "1": 101,
                "2": 115,
                "3": 116,
                "4": 32,
                "5": 118,
                "6": 97,
                "7": 117,
                "8": 108,
                "9": 116,
                "10": 0,
                "11": 0,
                "12": 0,
                "13": 0,
                "14": 0,
                "15": 0,
                "16": 0,
                "17": 0,
                "18": 0,
                "19": 0,
                "20": 0,
                "21": 0,
                "22": 0,
                "23": 0,
                "24": 0,
                "25": 0,
                "26": 0,
                "27": 0,
                "28": 0,
                "29": 0,
                "30": 0,
                "31": 0
        }"#;

        scope.push("json_str", json);

        // Parse JSON and convert to blob, then to string
        let script = "blob_to_string(json_str_to_blob(json_str))";
        let res = engine
            .eval_with_scope::<String>(&mut scope, script)
            .unwrap();

        // Verify we get "Test vault"
        assert_eq!(res.trim_end_matches(char::from(0)), "Test vault");
    }
}
