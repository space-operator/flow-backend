use crate::prelude::*;
use super::helper::{bridge_put, check_response};

pub const NAME: &str = "bridge_update_associated_person";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_update_associated_person.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub customer_id: String,
    pub associated_person_id: String,
    pub body: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/v0/customers/{}/associated_persons/{}", input.customer_id, input.associated_person_id);
    let result = check_response(
        bridge_put(&ctx, &path, &input.api_key)
            .json(&input.body)
            .send()
            .await?,
    )
    .await?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
