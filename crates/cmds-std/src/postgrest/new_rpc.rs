use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_new_rpc";

#[derive(Deserialize, Debug)]
struct Input {
    url: Option<String>,
    schema: Option<String>,
    function: String,
    params: JsonValue,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Query,
}

async fn run(ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let url = input
        .url
        .unwrap_or_else(|| format!("{}/rest/v1", ctx.endpoints().supabase));
    let url = format!("{}/rpc/{}", url, input.function);
    let query = postgrest::Builder::new(url, input.schema, <_>::default(), ctx.http().clone())
        .rpc(serde_json::to_string(&input.params)?)
        .into();

    Ok(Output { query })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/new_rpc.json"))?
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
