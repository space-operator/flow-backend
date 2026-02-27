use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_write_parquet";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_write_parquet.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub file_path: String,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_ipc(&input.dataframe)?;
    let file = std::fs::File::create(&input.file_path)
        .map_err(|e| CommandError::msg(format!("File create error: {e}")))?;
    ParquetWriter::new(file)
        .finish(&mut df)
        .map_err(|e| CommandError::msg(format!("Parquet write error: {e}")))?;
    Ok(Output {
        file_path: input.file_path,
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

    #[tokio::test]
    async fn test_run_write_parquet() {
        let mut df = DataFrame::new(vec![
            Column::new("name".into(), &["Alice", "Bob"]),
            Column::new("age".into(), &[30i64, 25]),
        ])
        .unwrap();
        let ipc = df_to_ipc(&mut df).unwrap();
        let path = "/tmp/test_polars_write.parquet".to_string();

        let output = run(
            CommandContext::default(),
            Input {
                dataframe: ipc,
                file_path: path.clone(),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.file_path, path);
        assert!(std::path::Path::new(&path).exists());
        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
