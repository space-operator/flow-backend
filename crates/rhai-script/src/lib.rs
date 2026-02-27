use anyhow::anyhow;
use chrono::Utc;
use convert::{dynamic_to_value, value_to_dynamic};
use flow_lib::command::prelude::*;
use rhai::{
    Dynamic, EvalAltResult,
    packages::{Package, StandardPackage},
};
use rhai_rand::RandomPackage;

pub mod convert;
pub mod error;

pub use error::ScriptError;
pub use rhai::Engine;

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

fn rhai_err(msg: String) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(msg.into(), rhai::Position::NONE).into()
}

fn base58_encode(input: rhai::Blob) -> String {
    bs58::encode(&input).into_string()
}

fn base58_decode(input: &str) -> Result<rhai::Blob, Box<EvalAltResult>> {
    bs58::decode(input)
        .into_vec()
        .map_err(|e| rhai_err(format!("base58 decode error: {e}")))
}

fn hex_encode(input: rhai::Blob) -> String {
    hex::encode(&input)
}

fn hex_decode(input: &str) -> Result<rhai::Blob, Box<EvalAltResult>> {
    hex::decode(input).map_err(|e| rhai_err(format!("hex decode error: {e}")))
}

fn json_encode(input: Dynamic) -> Result<String, Box<EvalAltResult>> {
    let value = dynamic_to_value(input)
        .map_err(|e| rhai_err(format!("json encode error: {e}")))?;
    let json = serde_json::Value::from(value);
    serde_json::to_string(&json)
        .map_err(|e| rhai_err(format!("json encode error: {e}")))
}

fn json_decode(input: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let json: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| rhai_err(format!("json decode error: {e}")))?;
    Ok(json_value_to_dynamic(json))
}

/// Convert serde_json::Value to Dynamic directly, keeping JSON integers as i64
/// rather than going through flow_lib::Value which converts u64 to Decimal.
fn json_value_to_dynamic(v: serde_json::Value) -> Dynamic {
    match v {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => b.into(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into()
            } else if let Some(f) = n.as_f64() {
                f.into()
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => s.into(),
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(json_value_to_dynamic)
            .collect::<rhai::Array>()
            .into(),
        serde_json::Value::Object(map) => map
            .into_iter()
            .map(|(k, v)| (k.into(), json_value_to_dynamic(v)))
            .collect::<rhai::Map>()
            .into(),
    }
}

pub fn setup_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_global_module(StandardPackage::new().as_shared_module())
        .register_static_module("rand", RandomPackage::new().as_shared_module())
        .register_fn("utc_now", utc_now)
        .register_fn("Decimal", decimal)
        .register_fn("base58_encode", base58_encode)
        .register_fn("base58_decode", base58_decode)
        .register_fn("hex_encode", hex_encode)
        .register_fn("hex_decode", hex_decode)
        .register_fn("json_encode", json_encode)
        .register_fn("json_decode", json_decode)
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
        ctx: CommandContext,
        mut input: ValueSet,
    ) -> Result<ValueSet, CommandError> {
        let code = String::deserialize(
            input
                .swap_remove(&self.source_code_name)
                .ok_or_else(|| anyhow!("missing input: {}", self.source_code_name))?,
        )?;

        let mut scope = rhai::Scope::new();

        let rhai_env = ctx
            .environment()
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
            .map_err(|error| anyhow!(ScriptError::from(error)))?;
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
mod tests;
