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
        restrict_intermediate_tokens: Some(true),
        // only_direct_routes: Some(true),
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
                    use_shared_accounts: true,
                    ..Default::default()
                },
            })
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error getting swap instructions: {}", e)))?;

    info!("swap_instructions: {swap_instructions:?}");

    let mut instructions = Vec::new();
    // instructions.extend(swap_instructions.compute_budget_instructions);
    // instructions.extend(swap_instructions.setup_instructions);
    instructions.push(swap_instructions.swap_instruction);
    // instructions.extend(swap_instructions.cleanup_instruction);

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

// #[test]
// fn test_base64_decode() {
//     let base64_str = "AQALGBkDEyVwHzVDiUOUxcGExcXKZZDyXRbt28+XpQBbpElaDgRKZH3R9xSERJ3WqurSlGA/bPQtOB0BqT6Jy6sKbJQQL8MhtzVyNwTTRcIEgSRctdCUhihwfFsqsFqnVwuegC9AcoTLPf2P06zMP6TlpArAEmFWOgt2JOyZrRRrFxRuVKJMFsG3ZY84R6OmOYDLWVFJyrLHjMlASSyEqUCgN7teu/B1KY1rHqjAfyDQA4RQ314Cd6sLsfznEsAbm7Eg8qIdmklnVTL3GrXKI61rcFwE4jsO/6OYROzRbst6fL4CsUt7cYFk34PLGbd9g1wstE9d4kXgAKnwKlDnKM53dDy16+kBCjw6zyWmncodNsdnIA0t2Bghwu8Hbthjg9BCWb147LXiSdQboCwNNzSwAt7ChKByDU4TTkQFXtHpo8p6496l3tIKIz6gt/KurQX9t33ssgsFcgohw2Jdh3k7SljnbLorLZ2ovMn87AV5qWf4sgVkYtGjLWzeY90y/fNh6+rYkFPvGkDQXa5j8bN5jWCQEzEFFVdRaQLh/XC+skAFAwZGb+UhFzL/7K26csOb57yM5bvF9xJrLEObOkAAAAAEedVb8jHAbu50xW7OaBUH/bGy3qP0jlECsc2iVrwTjwVKU1D4XciC1hSlVnJ4iilt3x6rq9CmBniISTL07vagBpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAEG3fbh12Whk9nL4UbO63msHLSF7V9bN5E6jPWFfv8AqU9LbA5BCP0qaiR46uCsxZ2yG7Tt9RHBYs9gLR4MCQH7hcSznTxVnguaIJxiZkAnurDmn3MWR0PC2GLghp2KJqGl1cqeBM9dtZC3FLov4yyxWRM/wcGStyJX/QfTnLBAHrQ/+if11/ZKdMCbHylYed5LCas238ndUUsyGqezjOXowjkmLIsm7an7+wTPuDHYXXk/MpA3dy+Cpgqowz8g5xLjRKUuABk5m+8C92Lh7YzAX4z9p72lV5a5uYQar+AGP7s4NL8Zq0BTMCnYuaviMo6ppVUawJB+I6R1qddVS9TfBA0BFgkDZAAAAAAAAAANAAUCfUsAAA4cERIAAwYLChAXDg4VDhQSEwUGCwQMAhEHCAEJDiTBIJszQdacgQcBAAAAGmQAAQDKmjsAAAAAiIR6JgAAAAAeAAAPAMABc29sYW5hLWFjdGlvbjpFNUFkdWRYR1Q3WmV4Y0hydFFxY3I5MW1QeGpURVExVGlKYWpTNTVxcTN3RjpCdFpUZnRCMzhFcXJBRXJYVHVudERBemtkMTJRV29VY2N3YmZOb2s5SmRrajo1a2NTbURFY1RSazlNNkxGRFREOUNVeXc5VkRFd0cxWlNYV1FHS2tIdFltY3RpbjliUkprelFzZkpBOXhFd1NQcXVxaHpXdDFNbzVDZjF2SnFzRVJtZXBw";

//     let decoded = base64::decode(base64_str).expect("Failed to decode base64");
//     let transaction_size = decoded.len();

//     println!("Transaction size: {} bytes", transaction_size);
//     assert!(
//         transaction_size <= 1232,
//         "Transaction exceeds Solana's size limit"
//     );
// }
