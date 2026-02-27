use crate::polars::types::dual_output;
use flow_lib::command::prelude::*;
use polars::prelude::*;

pub const NAME: &str = "polars_read_parquet";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_read_parquet.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let file = std::fs::File::open(&input.file_path)
        .map_err(|e| CommandError::msg(format!("File open error: {e}")))?;
    let mut df = ParquetReader::new(file)
        .finish()
        .map_err(|e| CommandError::msg(format!("Parquet read error: {e}")))?;
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
    async fn test_run_read_parquet_roundtrip() {
        // First write a parquet file using polars directly
        let path = "/tmp/test_polars_read_roundtrip.parquet";
        let mut df = DataFrame::new(vec![
            Column::new("city".into(), &["NYC", "LA", "SF"]),
            Column::new("pop".into(), &[8_000_000i64, 4_000_000, 800_000]),
        ])
        .unwrap();
        let file = std::fs::File::create(path).unwrap();
        ParquetWriter::new(file).finish(&mut df).unwrap();

        // Now read it back using the node
        let output = run(
            CommandContext::default(),
            Input {
                file_path: path.to_string(),
            },
        )
        .await
        .unwrap();

        let result_df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(result_df.shape(), (3, 2));
        let col_names: Vec<&str> = result_df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"city"));
        assert!(col_names.contains(&"pop"));

        // Clean up
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn test_run_parquet_write_then_read() {
        // Full round-trip: write parquet directly, then read via the node
        let path = "/tmp/test_polars_rw.parquet";
        let mut df = DataFrame::new(vec![
            Column::new("a".into(), &[1i64, 2, 3]),
            Column::new("b".into(), &["x", "y", "z"]),
        ])
        .unwrap();

        // Write parquet file directly
        let file = std::fs::File::create(path).unwrap();
        ParquetWriter::new(file).finish(&mut df).unwrap();

        // Read via read_parquet node
        let read_output = run(
            CommandContext::default(),
            Input {
                file_path: path.to_string(),
            },
        )
        .await
        .unwrap();

        let result_df = df_from_ipc(&read_output.dataframe).unwrap();
        assert_eq!(result_df.shape(), (3, 2));
        let col_names: Vec<&str> = result_df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"a"));
        assert!(col_names.contains(&"b"));

        // Clean up
        let _ = std::fs::remove_file(path);
    }
}
