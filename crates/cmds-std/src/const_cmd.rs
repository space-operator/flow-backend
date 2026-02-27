use flow_lib::command::prelude::*;

#[derive(Debug)]
pub enum ConfigType {
    Json,
    File,
    // Doesn't need special handling,
    // we don't care about them
    Other(String),
}

impl From<String> for ConfigType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "JSON" => ConfigType::Json,
            "File" => ConfigType::File,
            _ => ConfigType::Other(value),
        }
    }
}

impl From<&str> for ConfigType {
    fn from(value: &str) -> Self {
        match value {
            "JSON" => ConfigType::Json,
            "File" => ConfigType::File,
            _ => ConfigType::Other(value.to_owned()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ConfigType {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(d)?;
        Ok(s.into())
    }
}

#[derive(Debug)]
pub struct ConstCommand {
    inner: Inner,
}

pub const CONST_CMD: &str = "const";

const SOURCE: &str = "output";

#[derive(Debug)]
enum ConfigValue {
    Value(Value),
    Urls(Vec<String>),
}

#[derive(Debug)]
struct Inner {
    value: ConfigValue,
    r#type: ValueType,
}

#[derive(ThisError, Debug, Clone)]
pub enum Error {
    #[error("{0}")]
    Deserialize(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Deserialize(e.to_string())
    }
}

#[derive(Deserialize)]
struct ConstConfig {
    r#type: ConfigType,
    value: JsonValue,
}

fn read_config(config: ConstConfig) -> Result<Inner, Error> {
    match config.r#type {
        ConfigType::Json => {
            let value = match config.value {
                JsonValue::String(s) => {
                    let value: JsonValue = serde_json::from_str(&s)?;
                    Value::from(value)
                }
                other => flow_lib::command::parse_value_tagged_or_json(other),
            };
            Ok(Inner {
                value: ConfigValue::Value(value),
                r#type: ValueType::Free,
            })
        }
        ConfigType::File => {
            let urls: Vec<String> = serde_json::from_value(config.value.clone()).or_else(|_| {
                match flow_lib::command::parse_value_tagged_or_json(config.value) {
                    Value::Array(values) => values
                        .into_iter()
                        .map(|value| match value {
                            Value::String(s) => Ok(s),
                            _ => Err(serde_json::Error::io(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "const.file.value must be array<string>",
                            ))),
                        })
                        .collect(),
                    _ => Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "const.file.value must be array<string>",
                    ))),
                }
            })?;
            Ok(Inner {
                value: ConfigValue::Urls(urls),
                r#type: ValueType::Free,
            })
        }
        ConfigType::Other(_) => {
            let value = flow_lib::command::parse_value_tagged_or_json(config.value);
            Ok(Inner {
                value: ConfigValue::Value(value),
                r#type: ValueType::Free,
            })
        }
    }
}

fn parse_config(value: JsonValue) -> Result<ConstConfig, Error> {
    if let Ok(config) = serde_json::from_value::<ConstConfig>(value.clone()) {
        return Ok(config);
    }

    let r#type = value
        .get("type")
        .map(
            |json| match flow_lib::command::parse_value_tagged_or_json(json.clone()) {
                Value::String(s) => s.into(),
                _ => ConfigType::Other("".to_owned()),
            },
        )
        .unwrap_or(ConfigType::Other("".to_owned()));
    let value = value.get("value").cloned().unwrap_or(JsonValue::Null);
    Ok(ConstConfig { r#type, value })
}

impl ConstCommand {
    fn new(data: &NodeData) -> Result<Self, CommandError> {
        let config = parse_config(data.config.clone())?;
        let inner = read_config(config)?;
        Ok(Self { inner })
    }
}

#[async_trait(?Send)]
impl CommandTrait for ConstCommand {
    fn name(&self) -> Name {
        CONST_CMD.into()
    }

    fn inputs(&self) -> Vec<Input> {
        [].to_vec()
    }

    fn outputs(&self) -> Vec<Output> {
        [Output {
            name: SOURCE.into(),
            r#type: self.inner.r#type.clone(),
            optional: false,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: CommandContext, _inputs: ValueSet) -> Result<ValueSet, CommandError> {
        match &self.inner.value {
            ConfigValue::Value(value) => Ok(value::map! {
                SOURCE => value.clone(),
            }),
            ConfigValue::Urls(urls) => {
                // TODO: download the file
                let urls: Vec<Value> = urls.iter().map(|url| Value::String(url.clone())).collect();
                Ok(value::map! {
                    SOURCE => urls,
                })
            }
        }
    }
}

flow_lib::submit!(CommandDescription::new(CONST_CMD, |data: &NodeData| {
    Ok(Box::new(ConstCommand::new(data)?))
}));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdg_attr() {
        const JSON: &str = r#"
        {
          "Smoke_amount": {
            "concat": false,
            "flag": 0,
            "own": false,
            "type": 1,
            "value": [
              10
            ]
          }
        }"#;

        let res = read_config(ConstConfig {
            r#type: ConfigType::Json,
            value: JsonValue::String(JSON.to_owned()),
        })
        .unwrap();
        let val = match res.value {
            ConfigValue::Value(val) => val,
            _ => panic!("wrong type"),
        };
        assert_eq!(
            val,
            Value::Map(value::map! {
                "Smoke_amount" => value::map! {
                    "concat" => false,
                    "flag" => 0u64,
                    "own" => false,
                    "type" => 1u64,
                    "value" => vec![Value::U64(10)],
                }
            })
        );
    }
}
