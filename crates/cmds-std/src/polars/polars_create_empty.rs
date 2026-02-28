use crate::polars::types::{dual_output, parse_dtype};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_create_empty";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_create_empty.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub schema: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let obj = input
        .schema
        .as_object()
        .ok_or_else(|| CommandError::msg("schema must be a JSON object: {\"col_name\": \"dtype\", ...}"))?;

    let mut fields = Vec::new();
    for (col_name, dtype_val) in obj {
        let dtype_str = dtype_val
            .as_str()
            .ok_or_else(|| CommandError::msg(format!("dtype for column '{col_name}' must be a string")))?;
        let dtype = parse_dtype(dtype_str)?;
        fields.push(Field::new(PlSmallStr::from(col_name.as_str()), dtype));
    }

    let schema = Schema::from_iter(fields);
    let mut df = DataFrame::empty_with_schema(&schema);
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
    async fn test_run_create_empty() {
        let schema = serde_json::json!({
            "a": "i64",
            "b": "string"
        });
        let output = run(
            CommandContext::default(),
            Input { schema },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 0);
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"a"));
        assert!(col_names.contains(&"b"));
    }

    #[tokio::test]
    async fn test_run_create_empty_multiple_types() {
        let schema = serde_json::json!({
            "id": "u32",
            "name": "string",
            "score": "f64",
            "active": "bool"
        });
        let output = run(
            CommandContext::default(),
            Input { schema },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 0);
        assert_eq!(df.width(), 4);
    }
}
