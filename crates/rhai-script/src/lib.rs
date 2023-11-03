use anyhow::anyhow;
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

pub fn setup_engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_global_module(StandardPackage::new().as_shared_module())
        .register_static_module("rand", RandomPackage::new().as_shared_module())
        .register_fn("utc_now", utc_now)
        .register_fn("Decimal", decimal)
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
}
