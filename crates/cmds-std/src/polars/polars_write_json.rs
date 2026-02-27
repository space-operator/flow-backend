use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_write_json";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_write_json.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    #[serde(default)]
    pub pretty: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub json_string: String,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;
    let mut buf = Vec::new();
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df)
        .map_err(|e| CommandError::msg(format!("JSON write error: {e}")))?;
    let json_string = if input.pretty {
        let value: serde_json::Value = serde_json::from_slice(&buf)
            .map_err(|e| CommandError::msg(format!("JSON parse error: {e}")))?;
        serde_json::to_string_pretty(&value)
            .map_err(|e| CommandError::msg(format!("JSON pretty-print error: {e}")))?
    } else {
        String::from_utf8(buf)
            .map_err(|e| CommandError::msg(format!("UTF-8 error: {e}")))?
    };
    Ok(Output { json_string })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run_write_json() {
        let mut df = DataFrame::new(vec![
            Column::new("name".into(), &["Alice", "Bob"]),
            Column::new("age".into(), &[30i64, 25]),
        ])
        .unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();

        let output = run(
            CommandContext::default(),
            Input {
                dataframe: ipc,
                pretty: false,
            },
        )
        .await
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output.json_string).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alice");
        assert_eq!(arr[1]["age"], 25);
    }

    #[tokio::test]
    async fn test_run_write_json_pretty() {
        let mut df = DataFrame::new(vec![
            Column::new("val".into(), &[1i64, 2]),
        ])
        .unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();

        let output = run(
            CommandContext::default(),
            Input {
                dataframe: ipc,
                pretty: true,
            },
        )
        .await
        .unwrap();

        // Pretty JSON has newlines and indentation
        assert!(output.json_string.contains('\n'));
        let parsed: serde_json::Value = serde_json::from_str(&output.json_string).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }
}
