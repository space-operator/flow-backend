use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_sum";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_sum.jsonc");

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
    pub columns: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let exprs: Vec<Expr> = if let Some(ref cols) = input.columns {
        let col_names = parse_column_names(cols)?;
        col_names.iter().map(|c| col(c.as_str()).sum()).collect()
    } else {
        df.get_column_names()
            .iter()
            .map(|name| col(name.as_str()).sum())
            .collect()
    };

    let mut result = df
        .lazy()
        .select(exprs)
        .collect()
        .map_err(|e| CommandError::msg(format!("Sum error: {e}")))?;

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
            Series::new("category".into(), &["A", "B", "A", "B", "A"]).into_column(),
            Series::new("value".into(), &[10i64, 20, 30, 40, 50]).into_column(),
            Series::new("score".into(), &[1.0f64, 2.0, 3.0, 4.0, 5.0]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_single_column() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: Some(serde_json::json!(["value"])),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1, "sum should produce a 1-row DataFrame");
        let val = df.column("value").unwrap().get(0).unwrap();
        assert_eq!(val, AnyValue::Int64(150));
    }

    #[tokio::test]
    async fn test_run_multiple_columns() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: Some(serde_json::json!(["value", "score"])),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1, "sum should produce a 1-row DataFrame");
        let value_sum = df.column("value").unwrap().get(0).unwrap();
        assert_eq!(value_sum, AnyValue::Int64(150));
        let score_sum: f64 = df.column("score").unwrap().f64().unwrap().get(0).unwrap();
        assert!((score_sum - 15.0).abs() < 1e-10);
    }
}
