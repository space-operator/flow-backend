use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_rolling_sum";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_rolling_sum.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub column: String,
    pub window_size: u32,
    #[serde(default = "default_min_periods")]
    pub min_periods: u32,
}

fn default_min_periods() -> u32 { 1 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let output_col = format!("{}_rolling_sum", input.column);

    let opts = RollingOptionsFixedWindow {
        window_size: input.window_size as usize,
        min_periods: input.min_periods as usize,
        ..Default::default()
    };

    let mut result = df
        .lazy()
        .with_column(col(&input.column).rolling_sum(opts).alias(&output_col))
        .collect()
        .map_err(|e| CommandError::msg(format!("Rolling sum error: {e}")))?;

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
            Series::new("value".into(), &[1.0f64, 2.0, 3.0, 4.0, 5.0]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "value".to_string(),
            window_size: 3,
            min_periods: 3,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 5, "rolling_sum should preserve row count");
        let rolling = df.column("value_rolling_sum").unwrap();
        // First 2 values are null (min_periods=3)
        assert!(rolling.get(0).unwrap() == AnyValue::Null, "index 0 should be null");
        assert!(rolling.get(1).unwrap() == AnyValue::Null, "index 1 should be null");
        // sum(1,2,3) = 6.0
        let v2: f64 = rolling.f64().unwrap().get(2).unwrap();
        assert!((v2 - 6.0).abs() < 1e-10);
        // sum(2,3,4) = 9.0
        let v3: f64 = rolling.f64().unwrap().get(3).unwrap();
        assert!((v3 - 9.0).abs() < 1e-10);
        // sum(3,4,5) = 12.0
        let v4: f64 = rolling.f64().unwrap().get(4).unwrap();
        assert!((v4 - 12.0).abs() < 1e-10);
    }
}
