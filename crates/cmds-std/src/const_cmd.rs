use flow_lib::command::prelude::*;

#[derive(Debug)]
pub enum FormType {
    Json,
    File,
    // Doesn't need special handling,
    // we don't care about them
    Other(String),
}

impl From<String> for FormType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "JSON" => FormType::Json,
            "File" => FormType::File,
            _ => FormType::Other(value),
        }
    }
}

impl From<&str> for FormType {
    fn from(value: &str) -> Self {
        match value {
            "JSON" => FormType::Json,
            "File" => FormType::File,
            _ => FormType::Other(value.to_owned()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for FormType {
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

const SOURCE: &str = "Source";

#[derive(Debug)]
enum FormValue {
    Value(Value),
    Urls(Vec<String>),
}

#[derive(Debug)]
struct Inner {
    value: FormValue,
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
struct FormData {
    r#type: FormType,
    value: JsonValue,
}

fn read_form_data(form: FormData) -> Result<Inner, Error> {
    match form.r#type {
        FormType::Json => {
            let s: String = serde_json::from_value(form.value)?;
            let value: JsonValue = serde_json::from_str(&s)?;
            Ok(Inner {
                value: FormValue::Value(value.into()),
                r#type: ValueType::Free,
            })
        }
        FormType::File => {
            let urls: Vec<String> = serde_json::from_value(form.value)?;
            Ok(Inner {
                value: FormValue::Urls(urls),
                r#type: ValueType::Free,
            })
        }
        FormType::Other(_) => {
            let value = serde_json::from_value::<Value>(form.value)?;
            Ok(Inner {
                value: FormValue::Value(value),
                r#type: ValueType::Free,
            })
        }
    }
}

impl ConstCommand {
    fn new(data: &NodeData) -> Result<Self, CommandError> {
        let form: FormData = serde_json::from_value(data.targets_form.form_data.clone())?;
        let inner = read_form_data(form)?;
        Ok(Self { inner })
    }
}

#[async_trait]
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
            FormValue::Value(value) => Ok(value::map! {
                SOURCE => value.clone(),
            }),
            FormValue::Urls(urls) => {
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

        let res = read_form_data(FormData {
            r#type: FormType::Json,
            value: JsonValue::String(JSON.to_owned()),
        })
        .unwrap();
        let val = match res.value {
            FormValue::Value(val) => val,
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
