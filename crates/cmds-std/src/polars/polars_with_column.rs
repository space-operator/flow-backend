use crate::polars::types::{df_from_ipc, dual_output, series_from_ipc};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_with_column";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_with_column.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub series: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;
    let series = series_from_ipc(&input.series)?;

    df.with_column(Column::from(series))
        .map_err(|e| CommandError::msg(format!("With column error: {e}")))?;

    let (ipc, json) = dual_output(&mut df)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, series_to_ipc};

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
    async fn test_run_with_column() {
        let new_series = Series::new("city".into(), &["NYC", "LA", "SF"]);
        let series_ipc = series_to_ipc(&new_series).unwrap();

        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            series: series_ipc,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 4);
        let cities = df.column("city").unwrap();
        assert_eq!(cities.str().unwrap().get(0).unwrap(), "NYC");
    }
}
