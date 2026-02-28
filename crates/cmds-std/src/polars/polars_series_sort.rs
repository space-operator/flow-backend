use crate::polars::types::{series_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_series_sort";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_sort.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub series: String,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;
    let result = s.sort(SortOptions::new().with_order_descending(input.descending))
        .map_err(|e| CommandError::msg(format!("Sort error: {e}")))?;
    let (ipc, json) = dual_series_output(&result)?;
    Ok(Output {
        series: ipc,
        series_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{series_to_ipc, series_from_ipc};

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_sort_ascending() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[3, 1, 2]),
            descending: false,
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<i64> = result.i64().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_run_sort_descending() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[3, 1, 2]),
            descending: true,
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<i64> = result.i64().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![3, 2, 1]);
    }
}
