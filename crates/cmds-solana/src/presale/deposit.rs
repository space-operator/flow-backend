use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, derive_quote_vault, derive_escrow, fetch_presale_account, discriminators};
use super::RemainingAccountsInfo;

const NAME: &str = "deposit";
const DEFINITION: &str = flow_lib::node_definition!("presale/deposit.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    /// The presale to deposit into
    #[serde_as(as = "AsPubkey")]
    pub presale: Pubkey,
    /// Payer's quote token account (e.g. their USDC ATA)
    #[serde_as(as = "AsPubkey")]
    pub payer_quote_token: Pubkey,
    /// Payer wallet (signer)
    pub payer: Wallet,
    /// Maximum amount to deposit
    pub max_amount: u64,
    /// Registry index (default: 0 for permissionless)
    #[serde(default)]
    pub registry_index: u8,
    /// Transfer hook remaining accounts (optional)
    #[serde(default)]
    pub slices: Vec<super::RemainingAccountsSlice>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    /// Derived escrow address
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    /// Derived quote vault address
    #[serde_as(as = "AsPubkey")]
    pub quote_token_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Fetch presale account to get quote_mint
    let presale_data = fetch_presale_account(&ctx, &input.presale).await?;
    let quote_mint = presale_data.quote_mint;

    // Derive PDAs
    let quote_token_vault = derive_quote_vault(&input.presale);
    let escrow = derive_escrow(&input.presale, &input.payer.pubkey(), input.registry_index);
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.presale, false),
        AccountMeta::new(quote_token_vault, false),
        AccountMeta::new_readonly(quote_mint, false),
        AccountMeta::new(escrow, false),
        AccountMeta::new(input.payer_quote_token, false),
        AccountMeta::new_readonly(input.payer.pubkey(), true),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),
    ];

    let mut data = discriminators::DEPOSIT.to_vec();
    borsh::to_writer(&mut data, &input.max_amount)?;
    let remaining = RemainingAccountsInfo { slices: input.slices.clone() };
    data.extend(borsh::to_vec(&remaining)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.payer.clone()].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        escrow,
        quote_token_vault,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "presale" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer_quote_token" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "max_amount" => 1000000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
