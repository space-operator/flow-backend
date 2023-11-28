use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_is";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Builder,
    column: String,
    filter: String,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Builder,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: input.query.is(input.column, input.filter),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_is.json"))?
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
