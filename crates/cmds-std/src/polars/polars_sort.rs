use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_sort";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_sort.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub by: JsonValue,
    #[serde(default)]
    pub descending: JsonValue,
    #[serde(default = "default_true")]
    pub nulls_last: bool,
}

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn parse_descending(value: &JsonValue, num_cols: usize) -> Vec<bool> {
    match value {
        JsonValue::Bool(b) => vec![*b; num_cols],
        JsonValue::Array(arr) => arr
            .iter()
            .map(|v| v.as_bool().unwrap_or(false))
            .collect(),
        _ => vec![false; num_cols],
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let by_columns = parse_column_names(&input.by)?;
    let descending_vec = parse_descending(&input.descending, by_columns.len());

    let sort_options = SortMultipleOptions::new()
        .with_order_descending_multi(descending_vec)
        .with_nulls_last(input.nulls_last);

    let mut result = df
        .sort(by_columns, sort_options)
        .map_err(|e| CommandError::msg(format!("Sort error: {e}")))?;

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
            Series::new("age".into(), &[30i64, 25, 35]).into_column(),
            Series::new("score".into(), &[88.5f64, 92.0, 75.3]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_sort_ascending() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            by: serde_json::json!("age"),
            descending: serde_json::json!(false),
            nulls_last: true,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let names = df.column("name").unwrap();
        assert_eq!(names.str().unwrap().get(0).unwrap(), "Bob");
    }

    #[tokio::test]
    async fn test_run_sort_descending() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            by: serde_json::json!("age"),
            descending: serde_json::json!(true),
            nulls_last: true,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let names = df.column("name").unwrap();
        assert_eq!(names.str().unwrap().get(0).unwrap(), "Charlie");
    }
}
