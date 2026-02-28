use crate::polars::types::{series_from_ipc, dual_series_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_series_compare";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_compare.jsonc");

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
    pub operator: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let l = series_from_ipc(&input.left)?;
    let r = series_from_ipc(&input.right)?;

    let result = match input.operator.as_str() {
        "eq" => l.equal(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        "neq" => l.not_equal(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        "gt" => l.gt(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        "gte" => l.gt_eq(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        "lt" => l.lt(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        "lte" => l.lt_eq(&r).map_err(|e| CommandError::msg(format!("Compare error: {e}")))?.into_series(),
        _ => return Err(CommandError::msg(format!("Unknown operator: '{}'. Use: eq, neq, gt, gte, lt, lte", input.operator))),
    };

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
    async fn test_run_compare_gt() {
        let output = run(CommandContext::default(), Input {
            left: test_series_ipc("a", &[1, 2, 3]),
            right: test_series_ipc("b", &[2, 2, 2]),
            operator: "gt".to_string(),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<bool> = result.bool().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![false, false, true]);
    }

    #[tokio::test]
    async fn test_run_compare_eq() {
        let output = run(CommandContext::default(), Input {
            left: test_series_ipc("a", &[1, 2, 3]),
            right: test_series_ipc("b", &[2, 2, 2]),
            operator: "eq".to_string(),
        }).await.unwrap();
        let result = series_from_ipc(&output.series).unwrap();
        let vals: Vec<bool> = result.bool().unwrap().into_no_null_iter().collect();
        assert_eq!(vals, vec![false, true, false]);
    }
}
