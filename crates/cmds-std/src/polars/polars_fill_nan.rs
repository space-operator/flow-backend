use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_fill_nan";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_fill_nan.jsonc");

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
    pub value: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    let mut result = df
        .lazy()
        .with_column(col(&input.column).fill_nan(lit(input.value)))
        .collect()
        .map_err(|e| CommandError::msg(format!("Fill NaN error: {e}")))?;

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

    fn df_with_nan_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
            Series::new("value".into(), &[1.0f64, f64::NAN, 3.0]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_fill_nan() {
        let output = run(CommandContext::default(), Input {
            dataframe: df_with_nan_ipc(),
            column: "value".to_string(),
            value: 0.0,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        let values = df.column("value").unwrap();
        let val = values.f64().unwrap().get(1).unwrap();
        assert_eq!(val, 0.0);
        // Verify no NaN remains
        assert!(!val.is_nan());
    }
}
