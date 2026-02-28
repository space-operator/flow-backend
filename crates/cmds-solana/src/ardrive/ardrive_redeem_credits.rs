use crate::prelude::*;
use super::helper::{ardrive_get, check_response};

pub const NAME: &str = "ardrive_redeem_credits";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_redeem_credits.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub destination_address: String,
    pub id: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let result = check_response(
        ardrive_get(&ctx, "/redeem")
            .query(&[
                ("destinationAddress", &input.destination_address),
                ("id", &input.id),
                ("email", &input.email),
            ])
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
