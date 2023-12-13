use anyhow::anyhow;
use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_match";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Query,
    body: serde_json::Map<String, JsonValue>,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Query,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut query = postgrest::Builder::from_query(input.query, ctx.http);
    for (k, v) in input.body {
        let v = match v {
            JsonValue::Null => "null".to_owned(),
            JsonValue::Bool(x) => x.to_string(),
            JsonValue::Number(x) => x.to_string(),
            JsonValue::String(x) => serde_json::to_string(&x).unwrap(),
            JsonValue::Array(_) => return Err(anyhow!("array type is not supported")),
            JsonValue::Object(_) => return Err(anyhow!("object type is not supported")),
        };
        query = query.eq(k, v.to_string());
    }

    Ok(Output {
        query: query.into(),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_match.json"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
