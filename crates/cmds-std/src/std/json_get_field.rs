use crate::error::Error::ValueNotFound;
use crate::prelude::*;
use flow_lib::command::prelude::*;
use serde_json::Value as JsonValue;
use value::from_value;

#[derive(Debug, Clone)]
pub struct JsonGetField;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    field: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    result_json: JsonValue,
    result_string: String,
}

// Name
const JSON_GET_FIELD: &str = "json_get_field";

// Inputs
const JSON_OR_STRING: &str = "json_or_string";
const FIELD: &str = "field";

// Outputs
const RESULT_JSON: &str = "result_json";
const RESULT_STRING: &str = "result_string";

#[async_trait]
impl CommandTrait for JsonGetField {
    fn name(&self) -> Name {
        JSON_GET_FIELD.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: JSON_OR_STRING.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: FIELD.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [
            CmdOutput {
                name: RESULT_JSON.into(),
                r#type: ValueType::Json,
                optional: false,
            },
            CmdOutput {
                name: RESULT_STRING.into(),
                r#type: ValueType::String,
                optional: false,
            },
        ]
        .to_vec()
    }

    async fn run(
        &self,
        _ctx: CommandContextX,
        mut inputs: ValueSet,
    ) -> Result<ValueSet, CommandError> {
        let Input { field } = value::from_map(inputs.clone())?;

        let json = inputs
            .swap_remove(JSON_OR_STRING)
            .ok_or_else(|| crate::error::Error::ValueNotFound(JSON_OR_STRING.into()))?;

        match json {
            Value::Map(map) => {
                let value = map.get(&field).ok_or_else(|| ValueNotFound(field))?;

                let result_json: JsonValue = from_value(value.to_owned())?;
                let result_string = result_json.to_string();

                Ok(value::to_map(&Output {
                    result_json,
                    result_string,
                })?)
            }
            Value::String(s) => {
                let json: Result<HashMap<String, JsonValue>, _> = serde_json::from_str(&s);

                let value = json
                    .ok()
                    .and_then(|mut object| object.swap_remove(&field))
                    .unwrap_or_default();

                let result_json: JsonValue = value;
                let result_string = result_json.to_string();

                Ok(value::to_map(&Output {
                    result_json,
                    result_string,
                })?)
            }
            _ => {
                let result_json: JsonValue = JsonValue::Null;
                let result_string = "".into();
                Ok(value::to_map(&Output {
                    result_json,
                    result_string,
                })?)
            }
        }
    }
}

flow_lib::submit!(CommandDescription::new(JSON_GET_FIELD, |_| Ok(Box::new(
    JsonGetField {}
))));

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid() {
        let inputs = value::map! {
            JSON_OR_STRING => value::map! {
                "amount" => 100,
            },
            FIELD => "amount",
        };

        let output = JsonGetField.run(<_>::default(), inputs).await.unwrap();
        let result = value::from_map::<Output>(output).unwrap().result_json;
        assert_eq!(result, 100);
    }
}
