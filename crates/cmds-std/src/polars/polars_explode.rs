use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_explode";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_explode.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub columns: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let col_names = parse_column_names(&input.columns)?;
    let col_refs: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();

    let mut result = df
        .explode(col_refs)
        .map_err(|e| CommandError::msg(format!("Explode error: {e}")))?;

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

    fn test_df_ipc() -> String {
        // Create a DataFrame with a list column using df! macro and lazy API
        let df = df! {
            "name" => &["Alice", "Bob"],
            "scores" => &[Series::new("".into(), &[90i64, 85]), Series::new("".into(), &[80i64, 95, 70])],
        }.unwrap();
        let mut df = df;
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: serde_json::json!(["scores"]),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        // Alice has 2 scores, Bob has 3 scores => 5 rows total
        assert_eq!(df.height(), 5, "explode should produce 5 rows (2 + 3 scores)");
        assert!(df.column("name").is_ok(), "should have name column");
        assert!(df.column("scores").is_ok(), "should have scores column");
    }
}
