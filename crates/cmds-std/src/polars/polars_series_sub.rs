use crate::polars::types::{series_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_sub";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_sub.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub left: String,
    pub right: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let l = series_from_ipc(&input.left)?;
    let r = series_from_ipc(&input.right)?;
    let result = (&l - &r)
        .map_err(|e| CommandError::msg(format!("Subtract error: {e}")))?;
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
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_sub() {
        let output = run(CommandContext::default(), Input {
            left: test_series_ipc("a", &[10, 20, 30]),
            right: test_series_ipc("b", &[1, 2, 3]),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<i64> = result.i64().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![9, 18, 27]);
    }
}
