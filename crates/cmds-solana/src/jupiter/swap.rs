use crate::prelude::*;

use jupiter_swap_api_client::{
    quote::{QuoteRequest, SwapMode},
    swap::SwapRequest,
    transaction_config::{ComputeUnitPriceMicroLamports, TransactionConfig},
    JupiterSwapApiClient,
};
use tracing::info;

const NAME: &str = "jupiter_swap";

const DEFINITION: &str = flow_lib::node_definition!("jupiter/swap.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    input_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    output_mint: Pubkey,
    #[serde(default = "value::default::bool_true")]
    auto_slippage: bool,
    slippage_percent: Option<u16>,
    amount: u64,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    const API_BASE_URL: &str = "https://quote-api.jup.ag/v6";
    const MAX_AUTO_SLIPPAGE_BPS: u16 = 300;
    const DEXES: &str = "Whirlpool,Meteora DLMM,Raydium CLMM";

    info!("Using base url: {}", API_BASE_URL);
    info!("Using max auto slippage: {}", MAX_AUTO_SLIPPAGE_BPS);
    info!("Using dexes: {}", DEXES);


    let jupiter_swap_api_client = JupiterSwapApiClient::new(API_BASE_URL.into());

    let mut quote_request = QuoteRequest {
        amount: input.amount,
        input_mint: input.input_mint,
        output_mint: input.output_mint,
        dexes: Some(DEXES.into()),
        swap_mode: Some(SwapMode::ExactIn),
        as_legacy_transaction: Some(true),
        ..QuoteRequest::default()
    };

    if input.auto_slippage {
        quote_request.auto_slippage = Some(true);
        quote_request.max_auto_slippage_bps = Some(MAX_AUTO_SLIPPAGE_BPS);
        quote_request.compute_auto_slippage = true;
    } else if let Some(slippage_percent) = input.slippage_percent {
        quote_request.auto_slippage = Some(false);
        quote_request.slippage_bps = slippage_percent as u16 * 100;
    };

    // GET /quote
    let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();
    info!("{quote_response:#?}");

    // POST /swap-instructions
    let swap_instructions: jupiter_swap_api_client::swap::SwapInstructionsResponse =
        jupiter_swap_api_client
            .swap_instructions(&SwapRequest {
                user_public_key: input.fee_payer.pubkey(),
                quote_response,
                config: TransactionConfig {
                    wrap_and_unwrap_sol: true,
                    allow_optimized_wrapped_sol_token_account: true,
                    compute_unit_price_micro_lamports: Some(ComputeUnitPriceMicroLamports::Auto),
                    dynamic_compute_unit_limit: true,
                    as_legacy_transaction: true,
                    ..Default::default()
                },
            })
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error getting swap instructions: {}", e)))?;

    info!("swap_instructions: {swap_instructions:?}");

    let mut instructions = Vec::new();
    instructions.extend(swap_instructions.compute_budget_instructions);
    instructions.extend(swap_instructions.setup_instructions);
    instructions.push(swap_instructions.swap_instruction);
    instructions.extend(swap_instructions.cleanup_instruction);

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: instructions.into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

// !! NOTE: Swap instructions not executable on devnet
//
// #[cfg(test)]
// mod tests {
//     use solana_sdk::pubkey;

//     use super::*;

//     #[tokio::test]
//     async fn test_build() {
//         let ctx = Context::default();

//         let payer: Wallet = Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into();

//         const INPUT_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
//         const OUTPUT_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
//         let input = value::map! {
//             "fee_payer" => value::to_value(&payer).unwrap(),
//             "input_mint" => INPUT_MINT,
//             "output_mint" => OUTPUT_MINT,
//             "amount" => 10,
//             "submit" => true,
//         };
//         build().unwrap().run(ctx, input).await.unwrap();
//     }
// }
