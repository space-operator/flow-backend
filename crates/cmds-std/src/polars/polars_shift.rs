use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_shift";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_shift.jsonc");

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
    pub periods: i64,
    #[serde(default)]
    pub fill_value: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let output_col = format!("{}_shifted", input.column);

    let shift_expr = col(&input.column).shift(lit(input.periods));

    let expr = if let Some(ref fv) = input.fill_value {
        match fv {
            JsonValue::Number(n) => {
                if let Some(f) = n.as_f64() {
                    shift_expr.fill_null(lit(f))
                } else {
                    shift_expr
                }
            }
            JsonValue::String(s) => shift_expr.fill_null(lit(s.clone())),
            JsonValue::Bool(b) => shift_expr.fill_null(lit(*b)),
            _ => shift_expr,
        }
    } else {
        shift_expr
    };

    let mut result = df
        .lazy()
        .with_column(expr.alias(&output_col))
        .collect()
        .map_err(|e| CommandError::msg(format!("Shift error: {e}")))?;

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, df_from_ipc};

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("value".into(), &[1i64, 2, 3, 4, 5]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "value".to_string(),
            periods: 1,
            fill_value: None,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 5, "shift should preserve row count");
        let shifted = df.column("value_shifted").unwrap();
        // First value should be null after shifting by 1
        assert!(shifted.get(0).unwrap() == AnyValue::Null, "first value should be null");
        // Second value should be the original first value (1)
        assert_eq!(shifted.get(1).unwrap(), AnyValue::Int64(1));
        assert_eq!(shifted.get(2).unwrap(), AnyValue::Int64(2));
    }

    #[tokio::test]
    async fn test_run_with_fill_value() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            column: "value".to_string(),
            periods: 1,
            fill_value: Some(serde_json::json!(0)),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        let shifted = df.column("value_shifted").unwrap();
        // With fill_value=0, first value should be 0.0 (fill_null casts to f64)
        let val = shifted.get(0).unwrap();
        // The fill_null with a float literal may produce f64
        assert!(val != AnyValue::Null, "first value should not be null when fill_value is provided");
    }
}
