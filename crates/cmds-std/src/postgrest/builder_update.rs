use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_update";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Query,
    body: serde_json::Map<String, JsonValue>,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Query,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: postgrest::Builder::from_query(input.query, ctx.http().clone())
            .update(serde_json::to_string(&input.body)?)
            .into(),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_update.jsonc"))?
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
