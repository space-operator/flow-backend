use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_unique";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_unique.jsonc");

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
    pub columns: JsonValue,
    #[serde(default = "default_keep")]
    pub keep: String,
}

fn default_keep() -> String { "first".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn parse_keep_strategy(s: &str) -> Result<UniqueKeepStrategy, CommandError> {
    match s.to_lowercase().as_str() {
        "first" => Ok(UniqueKeepStrategy::First),
        "last" => Ok(UniqueKeepStrategy::Last),
        "any" => Ok(UniqueKeepStrategy::Any),
        "none" => Ok(UniqueKeepStrategy::None),
        other => Err(CommandError::msg(format!(
            "Unknown keep strategy: {other}. Valid: first, last, any, none"
        ))),
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let keep = parse_keep_strategy(&input.keep)?;

    let subset = if input.columns.is_null() || input.columns == JsonValue::Array(vec![]) {
        None
    } else {
        let names = parse_column_names(&input.columns)?;
        Some(names)
    };

    let mut result = df
        .unique::<String, String>(subset.as_deref(), keep, None)
        .map_err(|e| CommandError::msg(format!("Unique error: {e}")))?;

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, df_from_ipc};

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(30)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), Some(91.0)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_unique_all_columns() {
        // Rows 0 and 3 are (Alice, 30, 88.5) and (Alice, 30, 91.0) -- differ in score
        // So all 4 rows are unique when considering all columns
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: JsonValue::Null,
            keep: "first".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 4);
    }

    #[tokio::test]
    async fn test_run_unique_by_name() {
        // Unique by "name" column: Alice (2x), Bob, Charlie -> 3 unique
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: serde_json::json!(["name"]),
            keep: "first".into(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
    }

    #[tokio::test]
    async fn test_run_unique_with_duplicates() {
        // Create a DataFrame with exact duplicate rows
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Alice"]).into_column(),
            Series::new("age".into(), &[30i64, 25, 30]).into_column(),
        ]).unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();

        let output = run(CommandContext::default(), Input {
            dataframe: ipc,
            columns: JsonValue::Null,
            keep: "first".into(),
        }).await.unwrap();
        let result = df_from_ipc(&output.dataframe).unwrap();
        // Rows 0 and 2 are identical, so unique gives 2 rows
        assert_eq!(result.height(), 2);
    }
}
