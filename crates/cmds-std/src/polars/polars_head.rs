use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_head";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_head.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    #[serde(default = "default_n")]
    pub n: u64,
}

fn default_n() -> u64 { 5 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let mut head_df = df.head(Some(input.n as usize));
    let (ipc, json) = dual_output(&mut head_df)?;
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
    async fn test_run_head_n2() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: 2,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 3);
    }

    #[tokio::test]
    async fn test_run_head_default() {
        // Default n=5, but df only has 4 rows, so should return all 4
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: default_n(),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 4);
    }
}
