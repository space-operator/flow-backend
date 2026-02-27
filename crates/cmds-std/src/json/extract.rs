use flow_lib::command::prelude::*;

const JSON_EXTRACT: &str = "json_extract";

#[derive(Deserialize, Debug)]
struct Input {
    json_input: Value,
    field_path: String,
}

#[derive(Serialize, Debug)]
struct Output {
    value: Value,
    trimmed_json: Value,
}

async fn run(_: CommandContext, mut input: Input) -> Result<Output, CommandError> {
    // If json_input is a string, try to parse it as JSON.
    if let Value::String(s) = &input.json_input {
        let json: serde_json::Value = serde_json::from_str(s)
            .map_err(|_| CommandError::msg("json_input is a string but not valid JSON"))?;
        input.json_input = value::to_value(&json)?;
    }

    // Scalars (numbers, booleans, null) can't be path-extracted — return directly.
    match &input.json_input {
        Value::Map(_) | Value::Array(_) => {}
        Value::Null => {
            return Ok(Output {
                value: Value::Null,
                trimmed_json: Value::Null,
            });
        }
        scalar => {
            return Ok(Output {
                value: scalar.clone(),
                trimmed_json: Value::Null,
            });
        }
    }

    let path = value::crud::path::Path::parse(&input.field_path)?;
    let extracted =
        value::crud::remove(&mut input.json_input, &path.segments).unwrap_or(Value::Null);

    Ok(Output {
        value: extracted,
        trimmed_json: input.json_input,
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("json/extract.jsonc"))?
            .check_name(JSON_EXTRACT)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(JSON_EXTRACT, |_| build()));

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flat_field() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! {
                    "a" => 1i64,
                    "b" => 2i64,
                    "c" => 3i64,
                }),
                field_path: "c".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::I64(3));
        assert_eq!(
            output.trimmed_json,
            Value::Map(value::map! { "a" => 1i64, "b" => 2i64 })
        );
    }

    #[tokio::test]
    async fn test_nested_path() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! {
                    "data" => value::map! {
                        "user" => value::map! {
                            "name" => "Alice",
                        },
                    },
                }),
                field_path: "/data/user/name".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::String("Alice".to_string()));
    }

    #[tokio::test]
    async fn test_array_index() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! {
                    "items" => value::array![10i64, 20i64, 30i64],
                }),
                field_path: "/items/1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::I64(20));
    }

    #[tokio::test]
    async fn test_missing_field_returns_null() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! { "a" => 1i64 }),
                field_path: "missing".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::Null);
        assert_eq!(
            output.trimmed_json,
            Value::Map(value::map! { "a" => 1i64 })
        );
    }

    #[tokio::test]
    async fn test_extract_removes_from_source() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! {
                    "keep" => "yes",
                    "remove" => "gone",
                }),
                field_path: "remove".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::String("gone".to_string()));
        assert_eq!(
            output.trimmed_json,
            Value::Map(value::map! { "keep" => "yes" })
        );
    }

    #[tokio::test]
    async fn test_string_input_parsed_as_json() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::String(r#"{"name": "Bob", "age": 30}"#.to_string()),
                field_path: "name".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::String("Bob".to_string()));
    }

    #[tokio::test]
    async fn test_non_json_string_error() {
        let err = run(
            <_>::default(),
            Input {
                json_input: Value::String("not json".to_string()),
                field_path: "anything".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("not valid JSON"));
    }

    #[tokio::test]
    async fn test_number_passthrough() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::U64(42),
                field_path: "field".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::U64(42));
        assert_eq!(output.trimmed_json, Value::Null);
    }

    #[tokio::test]
    async fn test_string_number_parsed() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::String("3.14".to_string()),
                field_path: "anything".to_string(),
            },
        )
        .await
        .unwrap();
        // "3.14" is valid JSON, parsed as a number
        assert!(matches!(output.value, Value::F64(_)));
        assert_eq!(output.trimmed_json, Value::Null);
    }

    #[tokio::test]
    async fn test_null_passthrough() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Null,
                field_path: "field".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.value, Value::Null);
        assert_eq!(output.trimmed_json, Value::Null);
    }

    #[tokio::test]
    async fn test_extract_nested_object() {
        let output = run(
            <_>::default(),
            Input {
                json_input: Value::Map(value::map! {
                    "config" => value::map! {
                        "debug" => true,
                        "verbose" => false,
                    },
                    "name" => "app",
                }),
                field_path: "config".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            output.value,
            Value::Map(value::map! { "debug" => true, "verbose" => false })
        );
        assert_eq!(
            output.trimmed_json,
            Value::Map(value::map! { "name" => "app" })
        );
    }
}
