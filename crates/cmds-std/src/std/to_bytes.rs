use flow_lib::command::prelude::*;

const NAME: &str = "to_bytes";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("std/to_bytes.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    input: Value,
}

#[derive(Serialize, Debug)]
struct Output {
    bytes: bytes::Bytes,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let bytes = match input.input {
        Value::Null => bytes::Bytes::new(),
        Value::Bool(v) => bytes::Bytes::from(vec![v as u8]),
        Value::String(s) => bytes::Bytes::from(s),
        Value::U64(v) => bytes::Bytes::from(v.to_le_bytes().to_vec()),
        Value::I64(v) => bytes::Bytes::from(v.to_le_bytes().to_vec()),
        Value::U128(v) => bytes::Bytes::from(v.to_le_bytes().to_vec()),
        Value::I128(v) => bytes::Bytes::from(v.to_le_bytes().to_vec()),
        Value::F64(v) => bytes::Bytes::from(v.to_le_bytes().to_vec()),
        Value::Decimal(v) => bytes::Bytes::from(v.serialize().to_vec()),
        Value::B32(v) => bytes::Bytes::from(v.to_vec()),
        Value::B64(v) => bytes::Bytes::from(v.to_vec()),
        Value::Bytes(b) => b,
        other => bytes::Bytes::from(serde_json::to_vec(&other)?),
    };

    Ok(Output { bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_string_to_utf8() {
        let output = run(
            <_>::default(),
            Input {
                input: Value::String("hello".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.bytes.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn test_u64_le() {
        let output = run(
            <_>::default(),
            Input {
                input: Value::U64(256),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.bytes.as_ref(), &256u64.to_le_bytes());
    }

    #[tokio::test]
    async fn test_bool() {
        let output = run(
            <_>::default(),
            Input {
                input: Value::Bool(true),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.bytes.as_ref(), &[1u8]);
    }

    #[tokio::test]
    async fn test_null_empty() {
        let output = run(
            <_>::default(),
            Input {
                input: Value::Null,
            },
        )
        .await
        .unwrap();
        assert!(output.bytes.is_empty());
    }

    #[tokio::test]
    async fn test_b32_raw() {
        let key = [42u8; 32];
        let output = run(
            <_>::default(),
            Input {
                input: Value::B32(key),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.bytes.as_ref(), &key);
    }

    #[tokio::test]
    async fn test_bytes_passthrough() {
        let data = bytes::Bytes::from_static(b"\x00\x01\x02\x03");
        let output = run(
            <_>::default(),
            Input {
                input: Value::Bytes(data.clone()),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.bytes, data);
    }
}
