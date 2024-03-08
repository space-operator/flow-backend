use std::time::{SystemTime, UNIX_EPOCH};

use flow_lib::{command::prelude::*, solana::Pubkey};
use pyth_sdk_solana::state::SolanaPriceAccount;

const NAME: &str = "pyth_price";

fn build() -> BuildResult {
    Ok(CmdBuilder::new(flow_lib::node_definition!("pyth.json"))?
        .check_name(NAME)?
        .build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::pubkey")]
    price_feed_id: Pubkey,
}

#[derive(Serialize, Debug)]
struct Output {
    price: Option<i64>,
}

async fn run(ctx: Context, mut input: Input) -> Result<Output, CommandError> {
    let mut sol_price_account = ctx
        .solana_client
        .get_account(&input.price_feed_id)
        .await
        .map_err(|_| CommandError::msg("Failed to get price feed account"))?;

    let sol_price_feed =
        SolanaPriceAccount::account_to_feed(&input.price_feed_id, &mut sol_price_account)
            .map_err(|_| CommandError::msg("Invalid price feed account"))?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let maybe_price = sol_price_feed.get_price_no_older_than(current_time, 60);
    match maybe_price {
        Some(p) => Ok(Output {
            price: Some(p.price),
        }),
        None => Ok(Output { price: None }),
    }
}
