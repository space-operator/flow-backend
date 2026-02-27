use crate::polars::types::{df_from_ipc, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_sample";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_sample.jsonc");

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
    pub n: Option<u64>,
    #[serde(default)]
    pub fraction: Option<f64>,
    #[serde(default)]
    pub with_replacement: bool,
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let shuffle = true;

    let mut result = if let Some(n) = input.n {
        df.sample_n_literal(n as usize, input.with_replacement, shuffle, input.seed)
            .map_err(|e| CommandError::msg(format!("Sample error: {e}")))?
    } else if let Some(frac) = input.fraction {
        if frac < 0.0 || frac > 1.0 {
            return Err(CommandError::msg(
                "fraction must be between 0.0 and 1.0",
            ));
        }
        let n = (df.height() as f64 * frac) as usize;
        df.sample_n_literal(n, input.with_replacement, shuffle, input.seed)
            .map_err(|e| CommandError::msg(format!("Sample error: {e}")))?
    } else {
        return Err(CommandError::msg(
            "Either 'n' or 'fraction' must be provided for sampling",
        ));
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
    async fn test_run_sample_n() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: Some(2),
            fraction: None,
            with_replacement: false,
            seed: Some(42),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_run_sample_fraction() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: None,
            fraction: Some(0.5),
            with_replacement: false,
            seed: Some(42),
        }).await.unwrap();
        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);
    }

    #[tokio::test]
    async fn test_run_sample_deterministic() {
        // Same seed should produce the same result
        let output1 = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: Some(2),
            fraction: None,
            with_replacement: false,
            seed: Some(123),
        }).await.unwrap();
        let output2 = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: Some(2),
            fraction: None,
            with_replacement: false,
            seed: Some(123),
        }).await.unwrap();
        let df1 = df_from_ipc(&output1.dataframe).unwrap();
        let df2 = df_from_ipc(&output2.dataframe).unwrap();
        assert!(df1.equals(&df2));
    }

    #[tokio::test]
    async fn test_run_sample_fraction_over_one_errors() {
        let result = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: None,
            fraction: Some(1.5),
            with_replacement: false,
            seed: None,
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_sample_no_n_or_fraction_errors() {
        let result = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
            n: None,
            fraction: None,
            with_replacement: false,
            seed: None,
        }).await;
        assert!(result.is_err());
    }
}
