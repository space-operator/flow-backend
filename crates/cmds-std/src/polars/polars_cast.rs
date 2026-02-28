use crate::polars::types::{df_from_ipc, dual_output, parse_dtype};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_cast";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_cast.jsonc");

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
    pub dtype: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let target_dtype = parse_dtype(&input.dtype)?;

    let mut result = df
        .lazy()
        .with_column(col(&input.column).cast(target_dtype))
        .collect()
        .map_err(|e| CommandError::msg(format!("Cast error: {e}")))?;

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

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
            Series::new("age".into(), &[30i64, 25, 35]).into_column(),
            Series::new("score".into(), &[88.5f64, 92.0, 75.3]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_cast_i64_to_f64() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "age".to_string(),
            dtype: "f64".to_string(),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.column("age").unwrap().dtype(), &DataType::Float64);
    }

    #[tokio::test]
    async fn test_run_cast_i64_to_string() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "age".to_string(),
            dtype: "str".to_string(),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.column("age").unwrap().dtype(), &DataType::String);
        let ages = df.column("age").unwrap();
        assert_eq!(ages.str().unwrap().get(0).unwrap(), "30");
    }
}
