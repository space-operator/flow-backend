use crate::polars::types::{series_from_ipc, dual_series_output, parse_dtype};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_cast";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_cast.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub series: String,
    pub dtype: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;
    let dtype = parse_dtype(&input.dtype)?;
    let result = s.cast(&dtype)
        .map_err(|e| CommandError::msg(format!("Cast error: {e}")))?;
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
    async fn test_run_cast_to_f64() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[1, 2, 3]),
            dtype: "f64".to_string(),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        assert_eq!(result.dtype(), &DataType::Float64);
        let vals: Vec<f64> = result.f64().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn test_run_cast_to_string() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[1, 2, 3]),
            dtype: "string".to_string(),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        assert_eq!(result.dtype(), &DataType::String);
        let vals: Vec<&str> = result.str().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec!["1", "2", "3"]);
    }
}
