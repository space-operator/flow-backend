use flow_lib::command::prelude::*;

const NAME: &str = "postgrest_builder_select";

#[derive(Deserialize, Debug)]
pub struct Input {
    pub query: postgrest::Query,
    pub columns: String,
}

#[derive(Serialize, Debug)]
pub struct Output {
    pub query: postgrest::Query,
}

async fn run(ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        query: postgrest::Builder::from_query(input.query, ctx.http().clone())
            .select(input.columns)
            .into(),
    })
}

pub fn build() -> BuildResult {
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
