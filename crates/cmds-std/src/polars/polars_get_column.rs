use crate::polars::types::{df_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_get_column";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_get_column.jsonc");

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let col = df
        .column(&input.column)
        .map_err(|e| CommandError::msg(format!("Column error: {e}")))?;
    let series = col.as_materialized_series();
    let (ipc, json) = dual_series_output(series)?;
    Ok(Output {
        series: ipc,
        series_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, series_from_ipc};
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(30)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), Some(91.0)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_get_column() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "name".into(),
        }).await.unwrap();
        let series = series_from_ipc(&output.series).unwrap();
        assert_eq!(series.len(), 4);
        assert_eq!(series.name().as_str(), "name");
        let values: Vec<Option<&str>> = series.str().unwrap().into_iter().collect();
        assert_eq!(values, vec![Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]);
    }
}
