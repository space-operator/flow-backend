use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_skew";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_skew.jsonc");

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
    #[serde(default = "default_bias")]
    pub bias: bool,
}

fn default_bias() -> bool { true }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let exprs: Vec<Expr> = if let Some(ref cols) = input.columns {
        let col_names = parse_column_names(cols)?;
        col_names
            .iter()
            .map(|c| col(c.as_str()).skew(input.bias))
            .collect()
    } else {
        df.get_column_names()
            .iter()
            .map(|name| col(name.as_str()).skew(input.bias))
            .collect()
    };

    let mut result = df
        .lazy()
        .select(exprs)
        .collect()
        .map_err(|e| CommandError::msg(format!("Skew error: {e}")))?;

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
            Series::new("value".into(), &[1.0f64, 2.0, 3.0, 4.0, 100.0]).into_column(),
            Series::new("symmetric".into(), &[1.0f64, 2.0, 3.0, 4.0, 5.0]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_skew() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: Some(serde_json::json!(["value"])),
            bias: true,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 1, "skew should produce a 1-row DataFrame");
        let val: f64 = df.column("value").unwrap().f64().unwrap().get(0).unwrap();
        // [1,2,3,4,100] is heavily right-skewed
        assert!(val > 1.0, "expected positive skew for right-tailed data, got {val}");
    }

    #[tokio::test]
    async fn test_run_skew_symmetric() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: Some(serde_json::json!(["symmetric"])),
            bias: true,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let val: f64 = df.column("symmetric").unwrap().f64().unwrap().get(0).unwrap();
        // [1,2,3,4,5] is perfectly symmetric => skew ~ 0
        assert!(val.abs() < 0.01, "expected near-zero skew for symmetric data, got {val}");
    }
}
