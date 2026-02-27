use crate::polars::types::{series_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_div";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_div.jsonc");

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
    let result = (&l / &r)
        .map_err(|e| CommandError::msg(format!("Divide error: {e}")))?;
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

    fn test_series_f64_ipc(name: &str, values: &[f64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_div() {
        let output = run(CommandContext::default(), Input {
            left: test_series_f64_ipc("a", &[10.0, 20.0, 30.0]),
            right: test_series_f64_ipc("b", &[2.0, 5.0, 10.0]),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<f64> = result.f64().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![5.0, 4.0, 3.0]);
    }
}
