use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_cumsum";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_cumsum.jsonc");

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
    #[serde(default)]
    pub reverse: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let output_col = format!("{}_cumsum", input.column);

    let mut result = df
        .lazy()
        .with_column(col(&input.column).cum_sum(input.reverse).alias(&output_col))
        .collect()
        .map_err(|e| CommandError::msg(format!("Cumulative sum error: {e}")))?;

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
            Series::new("value".into(), &[1i64, 2, 3, 4, 5]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "value".to_string(),
            reverse: false,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 5, "cumsum should preserve row count");
        let cumsum = df.column("value_cumsum").unwrap();
        assert_eq!(cumsum.get(0).unwrap(), AnyValue::Int64(1));
        assert_eq!(cumsum.get(1).unwrap(), AnyValue::Int64(3));
        assert_eq!(cumsum.get(2).unwrap(), AnyValue::Int64(6));
        assert_eq!(cumsum.get(3).unwrap(), AnyValue::Int64(10));
        assert_eq!(cumsum.get(4).unwrap(), AnyValue::Int64(15));
    }
}
