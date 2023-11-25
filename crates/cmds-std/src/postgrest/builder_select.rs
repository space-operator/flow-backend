use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_select";

#[derive(Deserialize, Debug)]
struct Input {
    query: postgrest::Builder,
    columns: String,
}

#[derive(Serialize, Debug)]
struct Output {
    query: postgrest::Builder,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: input.query.select(input.columns),
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("postgrest/builder_select.json"))?
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
