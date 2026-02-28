use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_pivot";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_pivot.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub on: String,
    pub index: JsonValue,
    pub values: String,
    #[serde(default = "default_agg")]
    pub agg: String,
}

fn default_agg() -> String { "first".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

fn build_agg_expr(col_name: &str, agg: &str) -> Expr {
    match agg {
        "sum" => col(col_name).sum(),
        "mean" => col(col_name).mean(),
        "min" => col(col_name).min(),
        "max" => col(col_name).max(),
        "count" => col(col_name).count(),
        "last" => col(col_name).last(),
        "median" => col(col_name).median(),
        _ => col(col_name).first(), // default: first
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let index_cols = parse_column_names(&input.index)?;

    // Build pivot using group_by + agg, then reshape
    // First group by index + on column, aggregate the values
    let mut by_cols: Vec<Expr> = index_cols.iter().map(|c| col(c.as_str())).collect();
    by_cols.push(col(input.on.as_str()));

    let agg_expr = build_agg_expr(input.values.as_str(), &input.agg);

    let grouped = df
        .clone()
        .lazy()
        .group_by(&by_cols)
        .agg([agg_expr])
        .collect()
        .map_err(|e| CommandError::msg(format!("Pivot group_by error: {e}")))?;

    // Now use pivot to reshape
    let index_strs: Vec<&str> = index_cols.iter().map(|s| s.as_str()).collect();
    let mut result = pivot::pivot(
        &grouped,
        [input.on.as_str()],
        Some(index_strs),
        Some([input.values.as_str()]),
        false,
        None,
        None,
    )
    .map_err(|e| CommandError::msg(format!("Pivot error: {e}")))?;

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
            Series::new("product".into(), &["A", "A", "B", "B"]).into_column(),
            Series::new("quarter".into(), &["Q1", "Q2", "Q1", "Q2"]).into_column(),
            Series::new("sales".into(), &[10i64, 20, 30, 40]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            on: "quarter".to_string(),
            index: serde_json::json!(["product"]),
            values: "sales".to_string(),
            agg: "first".to_string(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2, "pivot should produce 2 rows (one per product)");
        assert!(df.column("Q1").is_ok(), "should have Q1 column");
        assert!(df.column("Q2").is_ok(), "should have Q2 column");
    }
}
