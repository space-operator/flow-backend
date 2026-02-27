use crate::polars::types::df_from_ipc;
use flow_lib::command::prelude::*;

pub const NAME: &str = "polars_schema";
const DEFINITION: &str = flow_lib::node_definition!("polars/polars_schema.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub dataframe: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub schema: JsonValue,
    pub column_names: JsonValue,
    pub dtypes: JsonValue,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let df = df_from_ipc(&input.dataframe)?;
    let schema = df.schema();

    let mut schema_map = serde_json::Map::new();
    let mut col_names = Vec::new();
    let mut dtype_strs = Vec::new();

    for (name, dtype) in schema.iter() {
        let dtype_str = format!("{}", dtype);
        schema_map.insert(name.to_string(), JsonValue::String(dtype_str.clone()));
        col_names.push(JsonValue::String(name.to_string()));
        dtype_strs.push(JsonValue::String(dtype_str));
    }

    Ok(Output {
        schema: JsonValue::Object(schema_map),
        column_names: JsonValue::Array(col_names),
        dtypes: JsonValue::Array(dtype_strs),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polars::types::df_to_ipc;
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
    async fn test_run_schema() {
        let output = run(CommandContext::default(), Input {
            dataframe: test_df_ipc(),
        }).await.unwrap();

        // Verify schema contains correct column names and types
        let schema = output.schema.as_object().unwrap();
        assert!(schema.contains_key("name"));
        assert!(schema.contains_key("age"));
        assert!(schema.contains_key("score"));

        // Verify column_names is a JSON array of the column name strings
        let col_names = output.column_names.as_array().unwrap();
        assert_eq!(col_names.len(), 3);
        assert_eq!(col_names[0].as_str().unwrap(), "name");
        assert_eq!(col_names[1].as_str().unwrap(), "age");
        assert_eq!(col_names[2].as_str().unwrap(), "score");

        // Verify dtypes array has 3 entries
        let dtypes = output.dtypes.as_array().unwrap();
        assert_eq!(dtypes.len(), 3);
    }
}
