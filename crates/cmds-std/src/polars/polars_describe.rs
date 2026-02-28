use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_describe";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_describe.jsonc");

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
    pub description: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;

    // Build summary statistics manually since DataFrame::describe() is not
    // available as a method in Polars 0.46. We compute per-column stats
    // using the lazy API aggregations.
    let mut stats = serde_json::Map::new();

    for col_ref in df.get_columns() {
        let col_name = col_ref.name().to_string();
        let series = col_ref.as_materialized_series();
        let mut col_stats = serde_json::Map::new();

        col_stats.insert("count".into(), JsonValue::Number(series.len().into()));
        col_stats.insert(
            "null_count".into(),
            JsonValue::Number(series.null_count().into()),
        );

        // Numeric stats (for numeric columns)
        if series.dtype().is_primitive_numeric() {
            let mean = series.mean_reduce();
            let val = mean.value().to_string();
            col_stats.insert(
                "mean".into(),
                serde_json::from_str(&val).unwrap_or(JsonValue::String(val)),
            );
            if let Ok(min) = series.min_reduce() {
                let val = min.value().to_string();
                col_stats.insert(
                    "min".into(),
                    serde_json::from_str(&val).unwrap_or(JsonValue::String(val)),
                );
            }
            if let Ok(max) = series.max_reduce() {
                let val = max.value().to_string();
                col_stats.insert(
                    "max".into(),
                    serde_json::from_str(&val).unwrap_or(JsonValue::String(val)),
                );
            }
        }

        col_stats.insert("dtype".into(), JsonValue::String(format!("{}", series.dtype())));
        stats.insert(col_name, JsonValue::Object(col_stats));
    }

    Ok(Output {
        description: JsonValue::Object(stats),
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
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Alice")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(30)]).into_column(),
            Series::new("score".into(), &[Some(88.5f64), Some(92.0), Some(75.3), Some(91.0)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_describe() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
        }).await.unwrap();

        let desc = output.description.as_object().unwrap();

        // All columns should be present
        assert!(desc.contains_key("name"));
        assert!(desc.contains_key("age"));
        assert!(desc.contains_key("score"));

        // Numeric columns should have mean, min, max stats
        let age_stats = desc["age"].as_object().unwrap();
        assert!(age_stats.contains_key("count"));
        assert!(age_stats.contains_key("mean"));
        assert!(age_stats.contains_key("min"));
        assert!(age_stats.contains_key("max"));
        assert_eq!(age_stats["count"], 4);

        let score_stats = desc["score"].as_object().unwrap();
        assert!(score_stats.contains_key("mean"));
        assert!(score_stats.contains_key("min"));
        assert!(score_stats.contains_key("max"));

        // String column should have count but no mean
        let name_stats = desc["name"].as_object().unwrap();
        assert!(name_stats.contains_key("count"));
        assert!(!name_stats.contains_key("mean"));
    }
}
