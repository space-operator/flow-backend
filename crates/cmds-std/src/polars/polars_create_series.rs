use crate::polars::types::{dual_series_output, parse_dtype};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_create_series";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_create_series.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub name: String,
    pub values: JsonValue,
    #[serde(default = "default_dtype")]
    pub dtype: String,
}

fn default_dtype() -> String {
    "f64".to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: String,
    pub series_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let arr = input
        .values
        .as_array()
        .ok_or_else(|| CommandError::msg("values must be a JSON array"))?;

    let dtype = parse_dtype(&input.dtype)?;
    let name = PlSmallStr::from(input.name.as_str());

    let s = match dtype {
        DataType::Boolean => {
            let vals: Vec<Option<bool>> = arr
                .iter()
                .map(|v| match v {
                    JsonValue::Bool(b) => Ok(Some(*b)),
                    JsonValue::Null => Ok(None),
                    other => Err(CommandError::msg(format!(
                        "Expected bool, got: {other}"
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Series::new(name, &vals)
        }
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => {
            let vals: Vec<Option<i64>> = arr
                .iter()
                .map(|v| match v {
                    JsonValue::Number(n) => Ok(n.as_i64()),
                    JsonValue::Null => Ok(None),
                    other => Err(CommandError::msg(format!(
                        "Expected integer, got: {other}"
                    ))),
                })
                .collect::<Result<_, _>>()?;
            let s = Series::new(name, &vals);
            s.cast(&dtype)
                .map_err(|e| CommandError::msg(format!("Cast error: {e}")))?
        }
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
            let vals: Vec<Option<u64>> = arr
                .iter()
                .map(|v| match v {
                    JsonValue::Number(n) => Ok(n.as_u64()),
                    JsonValue::Null => Ok(None),
                    other => Err(CommandError::msg(format!(
                        "Expected unsigned integer, got: {other}"
                    ))),
                })
                .collect::<Result<_, _>>()?;
            let s = Series::new(name, &vals);
            s.cast(&dtype)
                .map_err(|e| CommandError::msg(format!("Cast error: {e}")))?
        }
        DataType::Float32 | DataType::Float64 => {
            let vals: Vec<Option<f64>> = arr
                .iter()
                .map(|v| match v {
                    JsonValue::Number(n) => Ok(n.as_f64()),
                    JsonValue::Null => Ok(None),
                    other => Err(CommandError::msg(format!(
                        "Expected number, got: {other}"
                    ))),
                })
                .collect::<Result<_, _>>()?;
            let s = Series::new(name, &vals);
            s.cast(&dtype)
                .map_err(|e| CommandError::msg(format!("Cast error: {e}")))?
        }
        DataType::String => {
            let vals: Vec<Option<String>> = arr
                .iter()
                .map(|v| match v {
                    JsonValue::String(s) => Ok(Some(s.clone())),
                    JsonValue::Null => Ok(None),
                    other => Ok(Some(other.to_string())),
                })
                .collect::<Result<_, CommandError>>()?;
            Series::new(name, &vals)
        }
        _ => {
            return Err(CommandError::msg(format!(
                "Unsupported dtype for series creation: {dtype}"
            )));
        }
    };

    let (ipc, json) = dual_series_output(&s)?;
    Ok(Output {
        series: ipc,
        series_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::series_from_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run_create_series_i64() {
        let output = run(
            CommandContext::default(),
            Input {
                name: "scores".to_string(),
                values: serde_json::json!([10, 20, 30]),
                dtype: "i64".to_string(),
            },
        )
        .await
        .unwrap();

        let s = series_from_ipc(&output.series).unwrap();
        assert_eq!(s.name().as_str(), "scores");
        assert_eq!(s.len(), 3);
        assert_eq!(s.dtype(), &DataType::Int64);
    }

    #[tokio::test]
    async fn test_run_create_series_string() {
        let output = run(
            CommandContext::default(),
            Input {
                name: "names".to_string(),
                values: serde_json::json!(["Alice", "Bob", "Charlie"]),
                dtype: "string".to_string(),
            },
        )
        .await
        .unwrap();

        let s = series_from_ipc(&output.series).unwrap();
        assert_eq!(s.name().as_str(), "names");
        assert_eq!(s.len(), 3);
        assert_eq!(s.dtype(), &DataType::String);
    }

    #[tokio::test]
    async fn test_run_create_series_f64() {
        let output = run(
            CommandContext::default(),
            Input {
                name: "temps".to_string(),
                values: serde_json::json!([1.5, 2.7, 3.15]),
                dtype: "f64".to_string(),
            },
        )
        .await
        .unwrap();

        let s = series_from_ipc(&output.series).unwrap();
        assert_eq!(s.name().as_str(), "temps");
        assert_eq!(s.len(), 3);
        assert_eq!(s.dtype(), &DataType::Float64);
    }

    #[tokio::test]
    async fn test_run_create_series_bool() {
        let output = run(
            CommandContext::default(),
            Input {
                name: "flags".to_string(),
                values: serde_json::json!([true, false, true]),
                dtype: "bool".to_string(),
            },
        )
        .await
        .unwrap();

        let s = series_from_ipc(&output.series).unwrap();
        assert_eq!(s.name().as_str(), "flags");
        assert_eq!(s.len(), 3);
        assert_eq!(s.dtype(), &DataType::Boolean);
    }
}
