use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_left_join";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_left_join.jsonc");

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
    #[serde(default)]
    pub on: Option<JsonValue>,
    #[serde(default)]
    pub left_on: Option<JsonValue>,
    #[serde(default)]
    pub right_on: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let left = df_from_ipc(&input.left)?;
    let right = df_from_ipc(&input.right)?;

    let mut result = if let Some(ref on) = input.on {
        let on_cols = parse_column_names(on)?;
        left.join(&right, on_cols.clone(), on_cols, JoinArgs::new(JoinType::Left), None)
            .map_err(|e| CommandError::msg(format!("Left join error: {e}")))?
    } else {
        let left_on = parse_column_names(
            input.left_on.as_ref().ok_or_else(|| {
                CommandError::msg("Either 'on' or both 'left_on' and 'right_on' must be provided")
            })?,
        )?;
        let right_on = parse_column_names(
            input.right_on.as_ref().ok_or_else(|| {
                CommandError::msg("Either 'on' or both 'left_on' and 'right_on' must be provided")
            })?,
        )?;
        left.join(
            &right,
            left_on,
            right_on,
            JoinArgs::new(JoinType::Left),
            None,
        )
        .map_err(|e| CommandError::msg(format!("Left join error: {e}")))?
    };

    let (ipc, json) = dual_output(&mut result)?;
    Ok(Output {
        dataframe: ipc,
        dataframe_json: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    fn left_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("id".into(), &[1i64, 2, 3]).into_column(),
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    fn right_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("id".into(), &[2i64, 3, 4]).into_column(),
            Series::new("city".into(), &["NYC", "LA", "SF"]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_left_join() {
        let output = run(CommandContext::default(), Input {
            left: left_df_ipc(),
            right: right_df_ipc(),
            on: Some(serde_json::json!("id")),
            left_on: None,
            right_on: None,
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        // id=1 (Alice) should have null city
        let cities = df.column("city").unwrap();
        assert_eq!(cities.null_count(), 1);
    }
}
