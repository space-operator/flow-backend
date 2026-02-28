use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_rank";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_rank.jsonc");

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
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub descending: bool,
}

fn default_method() -> String { "average".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let output_col = format!("{}_rank", input.column);

    let method = match input.method.as_str() {
        "average" => RankMethod::Average,
        "min" => RankMethod::Min,
        "max" => RankMethod::Max,
        "dense" => RankMethod::Dense,
        "ordinal" => RankMethod::Ordinal,
        "random" => RankMethod::Random,
        _ => RankMethod::Average,
    };

    let opts = RankOptions {
        method,
        descending: input.descending,
    };

    let mut result = df
        .lazy()
        .with_column(col(&input.column).rank(opts, None).alias(&output_col))
        .collect()
        .map_err(|e| CommandError::msg(format!("Rank error: {e}")))?;

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
            Series::new("value".into(), &[40i64, 10, 30, 20, 50]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "value".to_string(),
            method: "ordinal".to_string(),
            descending: false,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 5, "rank should preserve row count");
        let rank = df.column("value_rank").unwrap();
        // ordinal rank of [40, 10, 30, 20, 50] => [4, 1, 3, 2, 5]
        // Polars ordinal rank returns u32
        assert_eq!(rank.get(0).unwrap(), AnyValue::UInt32(4));
        assert_eq!(rank.get(1).unwrap(), AnyValue::UInt32(1));
        assert_eq!(rank.get(2).unwrap(), AnyValue::UInt32(3));
        assert_eq!(rank.get(3).unwrap(), AnyValue::UInt32(2));
        assert_eq!(rank.get(4).unwrap(), AnyValue::UInt32(5));
    }
}
