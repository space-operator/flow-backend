use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, discriminators, RemainingAccountsInfo, RemainingAccountsSlice};

const NAME: &str = "presale_claim";
const DEFINITION: &str = flow_lib::node_definition!("presale/claim.jsonc");

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
    pub presale: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub presale_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner_base_token: Pubkey,
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub memo_program: Pubkey,
    pub slices: Vec<RemainingAccountsSlice>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.presale, false),              // presale (writable)
        AccountMeta::new(input.base_token_vault, false),     // base_token_vault (writable)
        AccountMeta::new_readonly(input.base_mint, false),   // base_mint (readonly)
        AccountMeta::new_readonly(input.presale_authority, false), // presale_authority (readonly)
        AccountMeta::new(input.escrow, false),               // escrow (writable)
        AccountMeta::new(input.owner_base_token, false),     // owner_base_token (writable)
        AccountMeta::new_readonly(input.owner.pubkey(), true), // owner (signer)
        AccountMeta::new_readonly(input.token_program, false), // token_program (readonly)
        AccountMeta::new_readonly(input.memo_program, false), // memo_program (readonly)
        AccountMeta::new_readonly(event_authority, false),   // event_authority (PDA)
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false), // program
    ];

    let remaining_accounts_info = RemainingAccountsInfo {
        slices: input.slices,
    };
    let mut data = discriminators::CLAIM.to_vec();
    data.extend(borsh::to_vec(&remaining_accounts_info)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
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
            "base_token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "presale_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner_base_token" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "memo_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "slices" => value::Value::Array(vec![]),
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
