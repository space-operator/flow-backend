use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, discriminators};
use super::RemainingAccountsInfo;

const NAME: &str = "perform_unsold_base_token_action";
const DEFINITION: &str = flow_lib::node_definition!("presale/perform_unsold_base_token_action.jsonc");

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
    /// Creator's base token account to receive unsold tokens
    #[serde_as(as = "AsPubkey")]
    pub creator_base_token: Pubkey,
    #[serde(default)]
    pub slices: Vec<super::RemainingAccountsSlice>,
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
        AccountMeta::new(input.presale, false),
        AccountMeta::new(input.base_token_vault, false),
        AccountMeta::new(input.base_mint, false),
        AccountMeta::new_readonly(input.presale_authority, false),
        AccountMeta::new(input.creator_base_token, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(spl_memo_interface::v3::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),
    ];

    let mut data = discriminators::PERFORM_UNSOLD_BASE_TOKEN_ACTION.to_vec();
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
        signers: [input.fee_payer.clone()].into(),
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
            "creator_base_token" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
