use crate::polars::types::series_from_ipc;
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_series_min_max";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_min_max.jsonc");

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
    pub min: JsonValue,
    pub max: JsonValue,
}

fn any_value_to_json(val: AnyValue) -> JsonValue {
    match val {
        AnyValue::Null => JsonValue::Null,
        AnyValue::Boolean(b) => serde_json::json!(b),
        AnyValue::Int8(v) => serde_json::json!(v),
        AnyValue::Int16(v) => serde_json::json!(v),
        AnyValue::Int32(v) => serde_json::json!(v),
        AnyValue::Int64(v) => serde_json::json!(v),
        AnyValue::UInt8(v) => serde_json::json!(v),
        AnyValue::UInt16(v) => serde_json::json!(v),
        AnyValue::UInt32(v) => serde_json::json!(v),
        AnyValue::UInt64(v) => serde_json::json!(v),
        AnyValue::Float32(v) => serde_json::json!(v),
        AnyValue::Float64(v) => serde_json::json!(v),
        AnyValue::String(s) => serde_json::json!(s),
        other => serde_json::json!(format!("{other}")),
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;

    let min_val = s.min_reduce()
        .map_err(|e| CommandError::msg(format!("Min error: {e}")))?;
    let max_val = s.max_reduce()
        .map_err(|e| CommandError::msg(format!("Max error: {e}")))?;

    Ok(Output {
        min: any_value_to_json(min_val.value().clone()),
        max: any_value_to_json(max_val.value().clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::series_to_ipc;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_min_max() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[5, 1, 8, 3]),
        }).await.unwrap();
        assert_eq!(output.min, serde_json::json!(1));
        assert_eq!(output.max, serde_json::json!(8));
    }
}
