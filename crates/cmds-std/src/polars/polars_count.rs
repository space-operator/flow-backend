use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_count";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_count.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub count: u64,
    pub null_counts: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let count = df.height() as u64;

    let null_df = df.null_count();
    let mut null_map = serde_json::Map::new();
    for col in null_df.get_columns() {
        let name = col.name().to_string();
        let null_count = col.get(0)
            .map(|v| {
                if let polars::prelude::AnyValue::UInt32(n) = v {
                    n as u64
                } else {
                    0
                }
            })
            .unwrap_or(0);
        null_map.insert(name, JsonValue::Number(serde_json::Number::from(null_count)));
    }

    Ok(Output {
        count,
        null_counts: JsonValue::Object(null_map),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("category".into(), &["A", "B", "A", "B", "A"]).into_column(),
            Series::new("value".into(), &[10i64, 20, 30, 40, 50]).into_column(),
            Series::new("score".into(), &[1.0f64, 2.0, 3.0, 4.0, 5.0]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_count() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
        }).await.unwrap();
        assert_eq!(output.count, 5);
        let null_counts = output.null_counts.as_object().unwrap();
        for (_col_name, count) in null_counts {
            assert_eq!(count.as_u64().unwrap(), 0, "no nulls expected in test data");
        }
        // Verify all 3 columns are present in null_counts
        assert!(null_counts.contains_key("category"));
        assert!(null_counts.contains_key("value"));
        assert!(null_counts.contains_key("score"));
    }
}
