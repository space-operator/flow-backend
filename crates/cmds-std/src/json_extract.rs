use flow_lib::command::prelude::*;

const JSON_EXTRACT: &str = "json_extract";

#[derive(Deserialize, Debug)]
struct Input {
    json_input: Value,
    field_path: String,
}

#[derive(Serialize, Debug)]
struct Output {
    value: Value,
    trimmed_json: Value,
}

async fn run(_: Context, mut input: Input) -> Result<Output, CommandError> {
    let path = value::crud::path::Path::parse(&input.field_path)?;
    let extracted =
        value::crud::remove(&mut input.json_input, &path.segments).unwrap_or(Value::Null);

    Ok(Output {
        value: extracted,
        trimmed_json: input.json_input,
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(crate::node_definition!("json_extract.json"))?
            .check_name(JSON_EXTRACT)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(JSON_EXTRACT, |_| build()));

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_extract() {
        let inputs = value::map! {
            "json_input" => value::map! {
                "a" => 1i64,
                "b" => 2i64,
                "c" => 3i64,
            },
            "field_path" => "c",
        };

        let mut outputs = build().unwrap().run(<_>::default(), inputs).await.unwrap();
        let value = outputs.remove("value").unwrap();
        let trimmed = outputs.remove("trimmed_json").unwrap();
        assert_eq!(value, Value::I64(3));
        assert_eq!(
            trimmed,
            Value::Map(value::map! {
                "a" => 1i64,
                "b" => 2i64,
            })
        );
    }
}
