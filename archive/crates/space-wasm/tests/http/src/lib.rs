use serde::{Deserialize, Serialize};
use space_lib::{space, Request, Result};

#[derive(Deserialize)]
struct Input {
    url: String,
}

#[derive(Serialize, Deserialize)]
struct Output {
    id: usize,
    title: String,
    description: String,
}

#[space]
fn main(input: Input) -> Result<Output> {
    let output = Request::get(input.url).call()?.into_json::<Output>()?;
    Ok(output)
}
