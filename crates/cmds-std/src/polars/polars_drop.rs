use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_drop";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_drop.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub columns: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let cols = parse_column_names(&input.columns)?;
    let mut dropped = df
        .drop_many(cols);
    let (ipc, json) = dual_output(&mut dropped)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::{df_to_ipc, df_from_ipc};
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
    async fn test_run_drop() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            columns: serde_json::json!(["score"]),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.width(), 2);
        assert_eq!(df.height(), 4);
        let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
        assert!(col_names.contains(&"name".to_string()));
        assert!(col_names.contains(&"age".to_string()));
        assert!(!col_names.contains(&"score".to_string()));
    }
}
