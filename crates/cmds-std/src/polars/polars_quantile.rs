use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_quantile";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_quantile.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub quantile: f64,
    #[serde(default)]
    pub columns: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.quantile < 0.0 || input.quantile > 1.0 {
        return Err(CommandError::msg("quantile must be between 0.0 and 1.0"));
    }

    let df = df_from_ipc(&input.dataframe)?;

    let col_list: Vec<String> = if let Some(ref cols) = input.columns {
        parse_column_names(cols)?
    } else {
        df.get_column_names()
            .iter()
            .map(|name| name.to_string())
            .collect()
    };

    let exprs: Vec<Expr> = col_list
        .iter()
        .map(|name| {
            col(name.as_str()).quantile(
                lit(input.quantile),
                QuantileMethod::Linear,
            )
        })
        .collect();

    let mut result = df
        .lazy()
        .select(exprs)
        .collect()
        .map_err(|e| CommandError::msg(format!("Quantile error: {e}")))?;

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
    async fn test_run_quantile_median() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            quantile: 0.5,
            columns: Some(serde_json::json!(["value"])),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1);
        let val: f64 = df.column("value").unwrap().f64().unwrap().get(0).unwrap();
        assert!((val - 30.0).abs() < 1e-10, "quantile 0.5 should be 30.0, got {val}");
    }

    #[tokio::test]
    async fn test_run_quantile_min() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            quantile: 0.0,
            columns: Some(serde_json::json!(["value"])),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let val: f64 = df.column("value").unwrap().f64().unwrap().get(0).unwrap();
        assert!((val - 10.0).abs() < 1e-10, "quantile 0.0 should be 10.0, got {val}");
    }

    #[tokio::test]
    async fn test_run_quantile_max() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            quantile: 1.0,
            columns: Some(serde_json::json!(["value"])),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let val: f64 = df.column("value").unwrap().f64().unwrap().get(0).unwrap();
        assert!((val - 50.0).abs() < 1e-10, "quantile 1.0 should be 50.0, got {val}");
    }
}
