use flow_lib::command::prelude::*;

const NAME: &str = "to_vec";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("std/to_vec.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    v0: Value,
    v1: Option<Value>,
    v2: Option<Value>,
    v3: Option<Value>,
    v4: Option<Value>,
    v5: Option<Value>,
    v6: Option<Value>,
    v7: Option<Value>,
}

#[derive(Serialize, Debug)]
struct Output {
    result: Vec<Value>,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut result = vec![input.v0];
    for v in [
        input.v1, input.v2, input.v3, input.v4, input.v5, input.v6, input.v7,
    ] {
        if let Some(v) = v {
            result.push(v);
        }
    }

    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_value() {
        let output = run(
            <_>::default(),
            Input {
                v0: Value::String("only".to_string()),
                v1: None,
                v2: None,
                v3: None,
                v4: None,
                v5: None,
                v6: None,
                v7: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, vec![Value::String("only".to_string())]);
    }

    #[tokio::test]
    async fn test_multiple_values() {
        let output = run(
            <_>::default(),
            Input {
                v0: Value::I64(1),
                v1: Some(Value::I64(2)),
                v2: Some(Value::I64(3)),
                v3: None,
                v4: None,
                v5: None,
                v6: None,
                v7: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            output.result,
            vec![Value::I64(1), Value::I64(2), Value::I64(3)]
        );
    }

    #[tokio::test]
    async fn test_all_slots() {
        let output = run(
            <_>::default(),
            Input {
                v0: Value::I64(0),
                v1: Some(Value::I64(1)),
                v2: Some(Value::I64(2)),
                v3: Some(Value::I64(3)),
                v4: Some(Value::I64(4)),
                v5: Some(Value::I64(5)),
                v6: Some(Value::I64(6)),
                v7: Some(Value::I64(7)),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result.len(), 8);
        for (i, v) in output.result.iter().enumerate() {
            assert_eq!(v, &Value::I64(i as i64));
        }
    }

    #[tokio::test]
    async fn test_mixed_types() {
        let output = run(
            <_>::default(),
            Input {
                v0: Value::I64(42),
                v1: Some(Value::String("hello".to_string())),
                v2: Some(Value::Bool(true)),
                v3: None,
                v4: None,
                v5: None,
                v6: None,
                v7: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            output.result,
            vec![
                Value::I64(42),
                Value::String("hello".to_string()),
                Value::Bool(true),
            ]
        );
    }
}
