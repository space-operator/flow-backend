use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_group_by";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_group_by.jsonc");

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
    pub agg: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn parse_agg_expr(col_name: &str, func: &str) -> Result<Expr, CommandError> {
    match func.to_lowercase().as_str() {
        "sum" => Ok(col(col_name).sum()),
        "mean" => Ok(col(col_name).mean()),
        "min" => Ok(col(col_name).min()),
        "max" => Ok(col(col_name).max()),
        "count" => Ok(col(col_name).count()),
        "first" => Ok(col(col_name).first()),
        "last" => Ok(col(col_name).last()),
        "median" => Ok(col(col_name).median()),
        "std" => Ok(col(col_name).std(1)),
        "var" => Ok(col(col_name).var(1)),
        other => Err(CommandError::msg(format!(
            "Unknown aggregation function: {other}. Valid: sum, mean, min, max, count, first, last, median, std, var"
        ))),
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let by_cols = parse_column_names(&input.by)?;

    let agg_map = input
        .agg
        .as_object()
        .ok_or_else(|| CommandError::msg("agg must be a JSON object mapping column names to aggregation functions"))?;

    let agg_exprs: Vec<Expr> = agg_map
        .iter()
        .map(|(col_name, func_val)| {
            let func = func_val.as_str().unwrap_or("sum");
            parse_agg_expr(col_name, func)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let by_exprs: Vec<Expr> = by_cols.iter().map(|c| col(c.as_str())).collect();

    let mut result = df
        .lazy()
        .group_by(by_exprs)
        .agg(agg_exprs)
        .collect()
        .map_err(|e| CommandError::msg(format!("Group by error: {e}")))?;

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
    async fn test_run_group_by_sum() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            by: serde_json::json!(["category"]),
            agg: serde_json::json!({"value": "sum"}),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2, "group by category should produce 2 rows");
        assert!(df.column("category").is_ok(), "output should have category column");

        // Sort by category to ensure deterministic order
        let df = df.sort(["category"], Default::default()).unwrap();
        let categories: Vec<&str> = df.column("category").unwrap().str().unwrap()
            .into_no_null_iter().collect();
        assert_eq!(categories, vec!["A", "B"]);

        let values: Vec<i64> = df.column("value").unwrap().i64().unwrap()
            .into_no_null_iter().collect();
        assert_eq!(values, vec![90, 60], "A sum=10+30+50=90, B sum=20+40=60");
    }

    #[tokio::test]
    async fn test_run_group_by_multi_agg() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            by: serde_json::json!(["category"]),
            agg: serde_json::json!({"value": "mean", "score": "sum"}),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);

        let df = df.sort(["category"], Default::default()).unwrap();

        let value_means: Vec<f64> = df.column("value").unwrap().f64().unwrap()
            .into_no_null_iter().collect();
        assert!((value_means[0] - 30.0).abs() < 1e-10, "A mean = (10+30+50)/3 = 30.0");
        assert!((value_means[1] - 30.0).abs() < 1e-10, "B mean = (20+40)/2 = 30.0");

        let score_sums: Vec<f64> = df.column("score").unwrap().f64().unwrap()
            .into_no_null_iter().collect();
        assert!((score_sums[0] - 9.0).abs() < 1e-10, "A score sum = 1+3+5 = 9.0");
        assert!((score_sums[1] - 6.0).abs() < 1e-10, "B score sum = 2+4 = 6.0");
    }
}
