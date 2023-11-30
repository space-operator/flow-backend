use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_order";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Query,
    columns: String,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Query,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: postgrest::Builder::from_query(input.query, ctx.http)
            .order(input.columns)
            .into(),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_order.json"))?
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
