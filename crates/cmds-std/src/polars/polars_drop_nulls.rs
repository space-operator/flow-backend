use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_drop_nulls";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_drop_nulls.jsonc");

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let subset = if input.columns.is_null() || input.columns == JsonValue::Array(vec![]) {
        None
    } else {
        let names = parse_column_names(&input.columns)?;
        Some(names)
    };

    let mut result = df
        .drop_nulls(subset.as_deref())
        .map_err(|e| CommandError::msg(format!("Drop nulls error: {e}")))?;

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
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_with_nulls_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), None, Some("Diana")]).into_column(),
            Series::new("age".into(), &[Some(30i64), None, Some(35), Some(28)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), None]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_drop_nulls_all() {
        // Row 0: all present, Row 1: age null, Row 2: name null, Row 3: score null
        // Only row 0 has no nulls across any column
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_with_nulls_ipc(),
            columns: JsonValue::Null,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1);
    }

    #[tokio::test]
    async fn test_run_drop_nulls_subset() {
        // Drop rows where "name" is null -- only row 2 has null name
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_with_nulls_ipc(),
            columns: serde_json::json!(["name"]),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
    }
}
