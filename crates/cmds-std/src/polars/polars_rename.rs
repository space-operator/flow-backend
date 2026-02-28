use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_rename";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_rename.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub mapping: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;
    let mapping = input
        .mapping
        .as_object()
        .ok_or_else(|| CommandError::msg("mapping must be a JSON object {\"old_name\": \"new_name\", ...}"))?;

    for (old_name, new_value) in mapping {
        let new_name = new_value
            .as_str()
            .ok_or_else(|| CommandError::msg(format!("Rename value for '{old_name}' must be a string")))?;
        df.rename(old_name, new_name.into())
            .map_err(|e| CommandError::msg(format!("Rename error: {e}")))?;
    }

    let (ipc, json) = dual_output(&mut df)?;
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
    async fn test_run_rename() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            mapping: serde_json::json!({"name": "full_name", "age": "years"}),
        }).await.unwrap();

        // Verify via JSON output (renames are reflected here)
        let rows = output.dataframe_json.as_array().unwrap();
        assert_eq!(rows.len(), 4);
        let first_row = rows[0].as_object().unwrap();
        assert!(first_row.contains_key("full_name"));
        assert!(first_row.contains_key("years"));
        assert!(first_row.contains_key("score"));
        assert!(!first_row.contains_key("name"));
        assert!(!first_row.contains_key("age"));

        // Verify the IPC round-trip also works
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.width(), 3);
        assert_eq!(df.height(), 4);
    }
}
