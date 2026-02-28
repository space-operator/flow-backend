use crate::prelude::*;
use super::helper::{ardrive_get, apply_auth, check_response, ArDriveAuth};

pub const NAME: &str = "ardrive_get_topup";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_get_topup.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub method: String,
    pub address: String,
    pub currency: String,
    pub amount: String,
    #[serde(default)]
    pub promo_code: Option<String>,
    #[serde(default)]
    pub ui_mode: Option<String>,
    #[serde(default)]
    pub return_url: Option<String>,
    #[serde(default)]
    pub success_url: Option<String>,
    #[serde(default)]
    pub cancel_url: Option<String>,
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
    let path = format!(
        "/top-up/{}/{}/{}/{}",
        input.method, input.address, input.currency, input.amount
    );
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
    if let Some(ref mode) = input.ui_mode {
        query.push(("uiMode", mode.clone()));
    }
    if let Some(ref url) = input.return_url {
        query.push(("returnUrl", url.clone()));
    }
    if let Some(ref url) = input.success_url {
        query.push(("successUrl", url.clone()));
    }
    if let Some(ref url) = input.cancel_url {
        query.push(("cancelUrl", url.clone()));
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
