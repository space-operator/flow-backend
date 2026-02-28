use crate::prelude::*;
use super::helper::{ardrive_get, check_response};

pub const NAME: &str = "ardrive_x402_topup";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_x402_topup.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payment_type: String,
    pub qty: String,
    #[serde(default)]
    pub x_payment: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut req = ardrive_get(&ctx, "/x402/top-up")
        .query(&[("type", &input.payment_type), ("qty", &input.qty)]);
    if let Some(ref payment) = input.x_payment {
        req = req.header("x-payment", payment);
    }
    let result = check_response(req.send().await?).await?;
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
