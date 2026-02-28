use crate::polars::types::{df_from_ipc, dual_output, parse_column_names};
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_semi_join";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_semi_join.jsonc");

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
    pub on: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let left = df_from_ipc(&input.left)?;
    let right = df_from_ipc(&input.right)?;

    let on_cols = parse_column_names(&input.on)?;

    let mut result = left
        .join(
            &right,
            on_cols.clone(),
            on_cols,
            JoinArgs::new(JoinType::Semi),
            None,
        )
        .map_err(|e| CommandError::msg(format!("Semi join error: {e}")))?;

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
    async fn test_run_semi_join() {
        let output = run(CommandContext::default(), Input {
            left: left_df_ipc(),
            right: right_df_ipc(),
            on: serde_json::json!("id"),
        }).await.unwrap();
        let df = crate::polars::types::df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2); // only left rows with matching id (2 and 3)
        assert_eq!(df.width(), 2); // only left columns (id, name), no right columns
        let ids: Vec<i64> = df.column("id").unwrap().i64().unwrap().into_no_null_iter().collect();
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
    }
}
