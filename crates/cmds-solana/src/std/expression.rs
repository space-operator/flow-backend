use crate::prelude::*;
#[derive(Debug)]
pub struct ScriptCommand;

impl ScriptCommand {
    // Counts the number of slots until the first unused.
    fn count_unique_slots(expression: &str) -> usize {
        for i in 0..usize::MAX {
            if !expression.contains(&format!("${{{}}}", i)) {
                return i;
            }
        }
        0
    }
}

pub const SCRIPT_CMD: &str = "expression";

// Inputs
const SCRIPT: &str = "script";
const VALUES: &str = "values";

// Outputs
const OUTPUT: &str = "output";

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub output: Value,
}
#[async_trait]
impl CommandTrait for ScriptCommand {
    fn name(&self) -> Name {
        SCRIPT_CMD.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: SCRIPT.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: VALUES.into(),
                type_bounds: [ValueType::Json].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: OUTPUT.into(),
            r#type: ValueType::Free,
        }]
        .to_vec()
    }

    async fn run(&self, _ctx: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let mut expression = if let Value::String(expression) = inputs
            .remove(SCRIPT)
            .ok_or_else(|| crate::Error::ValueNotFound(SCRIPT.into()))?
        {
            expression
        } else {
            return Err(crate::Error::RhaiExecutionError(
                "Cannot execute this expression".to_string(),
            )
            .into());
        };
        let values = match inputs
            .remove(VALUES)
            .ok_or_else(|| crate::Error::ValueNotFound(VALUES.into()))?
        {
            Value::Array(values) => values,
            _ => {
                return Err(crate::Error::RhaiExecutionError(
                    "Values passed aren't JSON array".to_string(),
                )
                .into())
            }
        };

        let slots = Self::count_unique_slots(&expression);

        if values.len() != slots {
            return Err(crate::Error::RhaiExecutionError(
                "Input values count not matching Script count".to_string(),
            )
            .into());
        }

        for (index, val) in values.iter().enumerate() {
            match val {
                Value::String(s) => {
                    expression = expression
                        .as_str()
                        .replace(&format!("${{{}}}", index), &format!("\"{}\"", s));
                }
                Value::Bool(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::Array(n) => {
                    let n = n
                        .iter()
                        .map(|v| {
                            let value = match v {
                                Value::Null => "null".into(),
                                Value::String(v) => v.to_string(),
                                Value::Decimal(v) => v.to_string(),
                                Value::U64(v) => v.to_string(),
                                Value::I64(v) => v.to_string(),
                                Value::U128(v) => v.to_string(),
                                Value::I128(v) => v.to_string(),
                                Value::F64(v) => v.to_string(),
                                Value::Bytes(v) => String::from_utf8_lossy(v).to_string(),
                                Value::Array(_v) => "only flat arrays supported".into(),
                                Value::Map(_v) => "maps_not_supported".into(),
                                Value::B32(v) => bs58::encode(&v).into_string(),
                                Value::B64(v) => bs58::encode(&v).into_string(),
                                other => serde_json::to_string_pretty(&other).unwrap(),
                            };
                            value
                        })
                        .collect::<Vec<String>>();

                    expression =
                        expression.replace(&format!("${{{}}}", index), &format!("{:#?}", n));
                }
                Value::Decimal(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::F64(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::I128(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::I64(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::U128(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                Value::U64(n) => {
                    expression = expression.replace(&format!("${{{}}}", index), &format!("{}", n));
                }
                _ => {
                    return Err(crate::Error::ValueNotFound(
                        "Value currently not supported!".into(),
                    )
                    .into());
                }
            }
        }

        let engine = rhai::Engine::new();

        let exp = engine
            .eval::<rhai::Dynamic>(&expression)
            .map_err(|e| crate::Error::RhaiExecutionError(e.to_string()))?;

        let output = match exp.type_name() {
            "i64" => {
                let v: Option<i64> = exp.try_cast();
                if let Some(v) = v {
                    Value::from(v)
                } else {
                    Value::Null
                }
            }
            "f64" => {
                let v: Option<f64> = exp.try_cast();
                if let Some(v) = v {
                    Value::from(v)
                } else {
                    Value::Null
                }
            }
            "string" => {
                let v: Option<String> = exp.try_cast();
                if let Some(v) = v {
                    Value::from(v)
                } else {
                    Value::Null
                }
            }
            "bool" => {
                let v: Option<bool> = exp.try_cast();
                if let Some(v) = v {
                    Value::from(v)
                } else {
                    Value::Null
                }
            }
            _ => {
                return Err(crate::Error::RhaiExecutionError(
                    "Currently not supported".to_string(),
                )
                .into());
            }
        };

        Ok(value::to_map(&Output { output })?)
    }
}

inventory::submit!(CommandDescription::new(SCRIPT_CMD, |_| {
    Ok(Box::new(ScriptCommand {}))
}));

#[cfg(test)]
mod test {
    use super::*;
    use value::{array, map, Value};

    #[tokio::test]
    async fn test_simple_command() {
        let cmd = ScriptCommand {};
        let ctx = Context::default();

        // Compare integers
        let inputs = map! {
            SCRIPT => "${0} + ${1}",
            VALUES => array![1, 2],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_ok());
    }

    #[tokio::test]
    async fn test_complex_command() {
        let cmd = ScriptCommand {};
        let ctx = Context::default();

        let expression = r#"
            let comparison = (${0} * ${1} / ${0} * ${2}) - ${3};
            if comparison > 0 {
                "The comparison is positive"
            }else{
                "The comparison is negative"
            }
        "#;

        // Compare integers
        let inputs = map! {
            SCRIPT => expression,
            VALUES => array![1, 2, 3, 5],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        dbg!(&outputs);
        assert!(outputs.is_ok());
        let outputs = outputs.unwrap();
        let o = outputs.get(OUTPUT);
        assert!(o.is_some());
        let o = o.unwrap();
        assert_eq!(o, &Value::String("The comparison is positive".into()));
    }

    #[tokio::test]
    async fn test_simple_comparison() {
        let cmd = ScriptCommand {};
        let ctx = Context::default();

        // Compare integers
        let inputs = map! {
            SCRIPT => "${0} * ${1}",
            VALUES => array![1, 2],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_ok());

        // Compare mixed types
        let inputs = map! {
            SCRIPT => "${0} - ${1}",
            VALUES => array!["1", 2],
        };
        dbg!(&inputs);

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_err());

        // Compare strings
        let inputs = map! {
            SCRIPT => r#"if ${0} == ${1} {"They match"}else{"They don't match"}"#,
            VALUES => array!["1", "2"],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_ok());
    }

    #[tokio::test]
    async fn text_missing_arguments() {
        let cmd = ScriptCommand {};
        let ctx = Context::default();

        // More values than expression slots
        let inputs = map! {
            SCRIPT => "${0} > ${1}",
            VALUES => array![1, 2, 3],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        dbg!(&outputs);

        // More expression slots than values
        let inputs = map! {
            SCRIPT => r#"${0} > ${1} && ${1} > ${2}"#,
            VALUES => array![1, 2],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_err());

        // No slots
        let inputs = map! {
            SCRIPT => "1 > 2",
            VALUES => array![1, 2],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_err());

        // No values
        let inputs = map! {
            SCRIPT => "${0} == ${1}",
            VALUES => array![],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_err());

        // No inputs
        let inputs = map! {
            SCRIPT => "1 > 2",
            VALUES => array![],
        };

        let outputs = cmd.run(ctx.clone(), inputs).await;
        assert!(outputs.is_ok());
    }
}
