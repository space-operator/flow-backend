use flow_lib::command::prelude::*;

const NAME: &str = "to_string";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("std/to_string.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(default)]
    stringify: Value,
}

#[derive(Serialize, Debug)]
struct Output {
    result: String,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let result = match input.stringify {
        Value::Null => String::new(),
        Value::Bool(v) => v.to_string(),
        Value::String(s) => s,
        Value::Decimal(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::U128(v) => v.to_string(),
        Value::I128(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::B32(v) => bs58::encode(&v).into_string(),
        Value::B64(v) => bs58::encode(&v).into_string(),
        other => serde_json::to_string_pretty(&other)?,
    };

    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_string_passthrough() {
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::String("hello".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, "hello");
    }

    #[tokio::test]
    async fn test_integer() {
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::I64(-42),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, "-42");
    }

    #[tokio::test]
    async fn test_bool() {
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::Bool(true),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, "true");
    }

    #[tokio::test]
    async fn test_null_returns_empty() {
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::Null,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, "");
    }

    #[tokio::test]
    async fn test_b32_base58() {
        let bytes = [1u8; 32];
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::B32(bytes),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.result, bs58::encode(&bytes).into_string());
    }

    #[tokio::test]
    async fn test_map_to_json() {
        let map = value::map! { "key" => "val" };
        let output = run(
            <_>::default(),
            Input {
                stringify: Value::Map(map.clone()),
            },
        )
        .await
        .unwrap();
        // Falls through to serde_json::to_string_pretty with Value's tagged format
        let roundtrip: Value = serde_json::from_str(&output.result).unwrap();
        assert_eq!(roundtrip, Value::Map(map));
    }
}
