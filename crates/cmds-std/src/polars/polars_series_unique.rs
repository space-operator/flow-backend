use crate::polars::types::{series_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_unique";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_unique.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub series: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
    pub n_unique: u64,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;
    let n_unique = s.n_unique()
        .map_err(|e| CommandError::msg(format!("Unique count error: {e}")))? as u64;
    let result = s.unique()
        .map_err(|e| CommandError::msg(format!("Unique error: {e}")))?;
    let (ipc, json) = dual_series_output(&result)?;
    Ok(Output {
        series: ipc,
        series_json: json,
        n_unique,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{series_to_ipc, series_from_ipc};
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_unique() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[1, 2, 2, 3, 3, 3]),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(output.n_unique, 3);
        let mut vals: Vec<i64> = result.i64().unwrap().into_no_null_iter().collect();
        vals.sort();
        assert_eq!(vals, vec![1, 2, 3]);
    }
}
