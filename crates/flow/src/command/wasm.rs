use crate::command::prelude::*;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use space_wasm::Wasm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    pub name: Name,
    pub r#type: ValueType,
}

#[derive(Debug, Clone)]
pub struct WasmCommand {
    pub bytes: Bytes,
    pub function: String,
    pub inputs: Vec<Description>,
    pub outputs: Vec<Description>,
}

#[async_trait]
impl CommandTrait for WasmCommand {
    fn name(&self) -> Name {
        "Wasm".into()
    }

    fn inputs(&self) -> Vec<Input> {
        self.inputs
            .iter()
            .map(|it| Input {
                name: it.name.clone(),
                type_bounds: [it.r#type.clone()].to_vec(),
                required: true,
                passthrough: false,
            })
            .collect()
    }

    fn outputs(&self) -> Vec<Output> {
        self.outputs
            .iter()
            .map(|it| Output {
                name: it.name.clone(),
                r#type: it.r#type.clone(),
                optional: false,
            })
            .collect()
    }

    async fn run(&self, ctx: Context, values: ValueSet) -> Result<ValueSet, CommandError> {
        let command = self.clone();
        let output_name = self
            .outputs
            .first()
            .ok_or(CommandError::msg("Expected 1 output, got 0"))?
            .name
            .clone();
        let env = ctx.environment;
        tokio::task::spawn_blocking(move || {
            let input: Json = match values.first() {
                _ if values.len() > 1 => {
                    let map = values
                        .into_iter()
                        .map(|(key, value)| (key, value.into()))
                        .collect();
                    Json::Object(map)
                }
                Some((_, input)) => input.clone().into(),
                _ => Err(CommandError::msg("Expected some input, got none"))?,
            };
            let wasm = Wasm::new(&command.bytes, env)?;
            let output = wasm.run::<Json, Json>(&command.function, &input)?;
            match output.as_object() {
                Some(object) if object.contains_key("Err") => {
                    let message = format!("{}", object["Err"]["description"]);
                    Err(CommandError::msg(message))
                }
                Some(mut object) => {
                    if object.contains_key("Ok") {
                        match object["Ok"].as_object() {
                            Some(ok_object) => object = ok_object,
                            _ => {
                                return Ok(ValueSet::from([(
                                    output_name,
                                    object["Ok"].clone().into(),
                                )]))
                            }
                        }
                    };
                    Ok(object
                        .into_iter()
                        .map(|(key, value)| (key.to_owned(), value.to_owned().into()))
                        .collect::<ValueSet>())
                }
                _ => Ok(ValueSet::from([(output_name, output.into())])),
            }
        })
        .await?
    }
}
