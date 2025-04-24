use crate::prelude::*;

// Command Name
const NAME: &str = "to_bytes";

const DEFINITION: &str = flow_lib::node_definition!("std/to_bytes.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
    static CACHE: Lazy<Result<CmdBuilder, BuilderError>> =
        Lazy::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub string: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub bytes: bytes::Bytes,
}

async fn run(mut _ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let string = input.string;
    let bytes = bytes::Bytes::from(string);

    Ok(Output { bytes })
}
