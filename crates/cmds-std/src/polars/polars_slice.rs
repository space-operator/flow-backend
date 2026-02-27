use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_slice";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_slice.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    #[serde(default)]
    pub offset: i64,
    pub length: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let mut result = df.slice(input.offset, input.length as usize);
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
    use polars::prelude::*;

    #[test]
    fn test_build() { build().unwrap(); }

    fn test_df_ipc() -> String {
        let mut df = DataFrame::new(vec![
            Series::new("name".into(), &[Some("Alice"), Some("Bob"), Some("Charlie"), Some("Diana")]).into_column(),
            Series::new("age".into(), &[Some(30i64), Some(25), Some(35), Some(28)]).into_column(),
        ]).unwrap();
        df_to_ipc(&mut df).unwrap()
    }

    #[tokio::test]
    async fn test_run_slice() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            offset: 1,
            length: 2,
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);
        // Should contain rows at index 1 and 2: Bob and Charlie
        let names: Vec<Option<&str>> = df.column("name").unwrap()
            .as_materialized_series().str().unwrap().into_iter().collect();
        assert_eq!(names, vec![Some("Bob"), Some("Charlie")]);
    }
}
