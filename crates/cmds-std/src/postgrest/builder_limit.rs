use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_limit";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Query,
    count: u64,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Query,
}

async fn run(ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: postgrest::Builder::from_query(input.query, ctx.http().clone())
            .limit(input.count as usize)
            .into(),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_limit.json"))?
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
