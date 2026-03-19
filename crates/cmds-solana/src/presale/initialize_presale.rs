use super::{
    InitializePresaleArgs, PRESALE_PROGRAM_ID, RemainingAccountsInfo, RemainingAccountsSlice,
    derive_base_vault, derive_event_authority, derive_presale, derive_presale_authority,
    derive_quote_vault, discriminators,
};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_sdk_ids::system_program;

const NAME: &str = "initialize_presale";
const DEFINITION: &str = flow_lib::node_definition!("presale/initialize_presale.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub presale_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub payer_presale_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub creator: Pubkey,
    pub base: Wallet,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub base_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_token_program: Pubkey,
    pub params: InitializePresaleArgs,
    #[serde(default)]
    pub slices: Vec<RemainingAccountsSlice>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub presale: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub presale_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub presale_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_token_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = derive_event_authority();
    let presale = derive_presale(
        &input.base.pubkey(),
        &input.presale_mint,
        &input.quote_token_mint,
    );
    let presale_authority = derive_presale_authority();
    let presale_vault = derive_base_vault(&presale);
    let quote_token_vault = derive_quote_vault(&presale);

    let accounts = vec![
        AccountMeta::new_readonly(input.presale_mint, false), // presale_mint (readonly)
        AccountMeta::new(presale, false),                     // presale (writable, PDA)
        AccountMeta::new_readonly(presale_authority, false),  // presale_authority (readonly)
        AccountMeta::new_readonly(input.quote_token_mint, false), // quote_token_mint (readonly)
        AccountMeta::new(presale_vault, false),               // presale_vault (writable, PDA)
        AccountMeta::new(quote_token_vault, false),           // quote_token_vault (writable, PDA)
        AccountMeta::new(input.payer_presale_token, false),   // payer_presale_token (writable)
        AccountMeta::new_readonly(input.creator, false),      // creator (readonly)
        AccountMeta::new_readonly(input.base.pubkey(), true), // base (signer)
        AccountMeta::new(input.payer.pubkey(), true),         // payer (writable, signer)
        AccountMeta::new_readonly(input.base_token_program, false), // base_token_program (readonly)
        AccountMeta::new_readonly(input.quote_token_program, false), // quote_token_program (readonly)
        AccountMeta::new_readonly(system_program::ID, false),        // system_program (readonly)
        AccountMeta::new_readonly(event_authority, false),           // event_authority (PDA)
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),        // program
    ];

    let remaining_accounts_info = RemainingAccountsInfo {
        slices: input.slices,
    };

    let mut data = discriminators::INITIALIZE_PRESALE.to_vec();
    data.extend(borsh::to_vec(&input.params)?);
    data.extend(borsh::to_vec(&remaining_accounts_info)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.base, input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        presale,
        presale_authority,
        presale_vault,
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
            "presale_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "quote_token_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer_presale_token" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "creator" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "base_token_program" => "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "quote_token_program" => "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "system_program" => "11111111111111111111111111111111",
            "params" => value::map! {
                "presale_params" => value::map! {
                    "presale_maximum_cap" => 1000000u64,
                    "presale_minimum_cap" => 100u64,
                    "presale_start_time" => 1700000000u64,
                    "presale_end_time" => 1700100000u64,
                    "whitelist_mode" => 0u64,
                    "presale_mode" => 0u64,
                    "unsold_token_action" => 0u64,
                    "disable_earlier_presale_end_once_cap_reached" => 0u64,
                    "padding" => vec![0u8; 30],
                },
                "locked_vesting_params" => value::map! {
                    "immediately_release_bps" => 10000u64,
                    "lock_duration" => 0u64,
                    "vest_duration" => 0u64,
                    "immediate_release_timestamp" => 1700100000u64,
                    "padding" => vec![0u8; 24],
                },
                "padding" => vec![0u8; 32],
                "presale_registries" => value::Value::Array(vec![]),
            },
            "submit" => false,
        };
        // presale, presale_authority, presale_vault, quote_token_vault omitted — should auto-derive
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
