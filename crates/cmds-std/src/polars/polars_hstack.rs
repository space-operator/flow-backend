use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_hstack";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_hstack.jsonc");

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut left_df = df_from_ipc(&input.left)?;
    let right_df = df_from_ipc(&input.right)?;

    left_df
        .hstack_mut(right_df.get_columns())
        .map_err(|e| CommandError::msg(format!("Hstack error: {e}")))?;

    let (ipc, json) = dual_output(&mut left_df)?;
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

    #[tokio::test]
    async fn test_run() {
        let mut left = DataFrame::new(vec![
            Series::new("name".into(), &["Alice", "Bob", "Charlie"]).into_column(),
        ]).unwrap();
        let mut right = DataFrame::new(vec![
            Series::new("age".into(), &[30i64, 25, 35]).into_column(),
            Series::new("score".into(), &[90i64, 80, 85]).into_column(),
        ]).unwrap();

        let output = run(CommandContext::default(), Input {
            left: df_to_ipc(&mut left).unwrap(),
            right: df_to_ipc(&mut right).unwrap(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3, "hstack should preserve 3 rows");
        assert_eq!(df.width(), 3, "hstack should produce 3 columns (1 + 2)");
        assert!(df.column("name").is_ok(), "should have name column");
        assert!(df.column("age").is_ok(), "should have age column");
        assert!(df.column("score").is_ok(), "should have score column");
    }
}
