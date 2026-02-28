use crate::polars::types::series_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_series_sum";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_series_sum.jsonc");

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
    pub value: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let s = series_from_ipc(&input.series)?;
    let sum_val: JsonValue = match s.sum_reduce()
        .ok()
        .map(|scalar| {
            let av = scalar.value().clone();
            match av {
                polars::prelude::AnyValue::Float64(f) => serde_json::json!(f),
                polars::prelude::AnyValue::Float32(f) => serde_json::json!(f),
                polars::prelude::AnyValue::Int64(i) => serde_json::json!(i),
                polars::prelude::AnyValue::Int32(i) => serde_json::json!(i),
                polars::prelude::AnyValue::UInt64(u) => serde_json::json!(u),
                polars::prelude::AnyValue::UInt32(u) => serde_json::json!(u),
                polars::prelude::AnyValue::Null => JsonValue::Null,
                other => serde_json::json!(format!("{other}")),
            }
        }) {
        Some(v) => v,
        None => JsonValue::Null,
    };
    Ok(Output {
        value: sum_val,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::series_to_ipc;
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_series_ipc(name: &str, values: &[i64]) -> String {
        let s = Series::new(name.into(), values);
        series_to_ipc(&s).unwrap()
    }

    #[tokio::test]
    async fn test_run_sum() {
        let output = run(CommandContext::default(), Input {
            series: test_series_ipc("a", &[10, 20, 30]),
        }).await.unwrap();
        assert_eq!(output.value, serde_json::json!(60));
    }
}
