use crate::polars::types::{df_from_json_value, dual_output};
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_create_dataframe";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_create_dataframe.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub data: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub dataframe: String,
    pub dataframe_json: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut df = df_from_json_value(&input.data)?;
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
    async fn test_run_create_dataframe() {
        let data = serde_json::json!([
            {"col_a": 1, "col_b": "x"},
            {"col_a": 2, "col_b": "y"},
            {"col_a": 3, "col_b": "z"}
        ]);
        let output = run(CommandContext::default(), Input { data })
            .await
            .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 2);
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"col_a"));
        assert!(col_names.contains(&"col_b"));
    }

    #[tokio::test]
    async fn test_run_create_dataframe_column_oriented() {
        let data = serde_json::json!({
            "product": ["Laptop", "Phone", "Tablet"],
            "price": [999.99, 699.99, 449.99],
            "quantity": [10, 25, 15]
        });
        let output = run(CommandContext::default(), Input { data })
            .await
            .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(df.width(), 3);
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"product"));
        assert!(col_names.contains(&"price"));
        assert!(col_names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn test_run_create_dataframe_string_input() {
        // Simulates IValue {S: "..."} being deserialized as JsonValue::String
        let data =
            serde_json::json!("{\"product\":[\"Laptop\",\"Phone\"],\"price\":[999.99,699.99]}");
        let output = run(CommandContext::default(), Input { data })
            .await
            .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 2);
    }

    #[tokio::test]
    async fn test_run_create_dataframe_single_column() {
        let data = serde_json::json!([
            {"values": 10},
            {"values": 20},
            {"values": 30},
            {"values": 40}
        ]);
        let output = run(CommandContext::default(), Input { data })
            .await
            .unwrap();

        let df = df_from_ipc(&output.dataframe).unwrap();
        assert_eq!(df.shape(), (4, 1));
    }
}
