use anyhow::anyhow;
use convert::{dynamic_to_value, value_to_dynamic};
use flow_lib::command::prelude::*;
use rhai::{
    packages::{Package, StandardPackage},
    Dynamic, Engine,
};
use rhai_rand::RandomPackage;

pub mod convert;

pub fn setup_engine() -> Engine {
    let mut engine = Engine::new();
    engine.register_global_module(StandardPackage::new().as_shared_module());
    engine.register_static_module("rand", RandomPackage::new().as_shared_module());
    engine
}

pub struct Command {
    source_code_name: Name,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
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
                .remove(&self.source_code_name)
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
            match input.remove(&i.name) {
                Some(value) => {
                    scope.push_dynamic(&i.name, value_to_dynamic(value));
                }
                None => {
                    if i.required {
                        tracing::warn!("missing input: {}", i.name);
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
            outputs.insert(o.name.clone(), value);
        }
        if outputs.is_empty() && self.outputs.len() == 1 {
            let name = self.outputs[0].name.clone();
            let value =
                dynamic_to_value(eval_result).map_err(|error| anyhow!("{:?}: {}", name, error))?;
            outputs.insert(name, value);
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
