use crate::polars::types::{df_from_json_value, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_from_rows";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_from_rows.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub rows: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_json_value(&input.rows)?;
    let (ipc, json) = dual_output(&mut df)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_from_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run_from_rows() {
        let rows = serde_json::json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);
        let output = run(
            CommandContext::default(),
            Input { rows },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (2, 2));
        let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
        assert!(col_names.contains(&"name".to_string()));
        assert!(col_names.contains(&"age".to_string()));
    }

    #[tokio::test]
    async fn test_run_from_rows_three_columns() {
        let rows = serde_json::json!([
            {"x": 1, "y": 2, "z": 3},
            {"x": 4, "y": 5, "z": 6},
            {"x": 7, "y": 8, "z": 9}
        ]);
        let output = run(
            CommandContext::default(),
            Input { rows },
        )
        .await
        .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (3, 3));
    }
}
