use crate::prelude::*;
use super::helper::{ardrive_get, apply_auth, check_response, ArDriveAuth};

pub const NAME: &str = "ardrive_get_price_quote";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_get_price_quote.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payment_type: String,
    pub amount: String,
    #[serde(default)]
    pub promo_code: Option<String>,
    #[serde(default)]
    pub destination_address: Option<String>,
    #[serde(default)]
    pub x_signature: Option<String>,
    #[serde(default)]
    pub x_nonce: Option<String>,
    #[serde(default)]
    pub x_public_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/price/{}/{}", input.payment_type, input.amount);
    let mut req = ardrive_get(&ctx, &path);
    req = apply_auth(
        req,
        &ArDriveAuth {
            x_signature: input.x_signature,
            x_nonce: input.x_nonce,
            x_public_key: input.x_public_key,
        },
    );
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(ref code) = input.promo_code {
        query.push(("promoCode", code.clone()));
    }
    if let Some(ref addr) = input.destination_address {
        query.push(("destinationAddress", addr.clone()));
    }
    if !query.is_empty() {
        req = req.query(&query);
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
