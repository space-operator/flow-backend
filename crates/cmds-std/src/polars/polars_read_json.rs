use crate::polars::types::dual_output;
use flow_lib::command::prelude::*;
use polars::prelude::*;
use std::io::Cursor;

pub const NAME: &str = "polars_read_json";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_read_json.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub json_string: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let cursor = Cursor::new(input.json_string.as_bytes());
    let mut df = JsonReader::new(cursor)
        .finish()
        .map_err(|e| CommandError::msg(format!("JSON parse error: {e}")))?;
    let (ipc, json) = dual_output(&mut df)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_from_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run_json_array() {
        let json = r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#.to_string();
        let output = run(
            CommandContext::default(),
            Input {
                json_string: json,
            },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (2, 2));
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"age"));
    }

    #[tokio::test]
    async fn test_run_json_single_row() {
        let json = r#"[{"x":1,"y":2,"z":3}]"#.to_string();
        let output = run(
            CommandContext::default(),
            Input {
                json_string: json,
            },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (1, 3));
    }
}
