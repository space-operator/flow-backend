use super::RemainingAccountsInfo;
use super::{
    PRESALE_PROGRAM_ID, derive_event_authority, discriminators,
};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "withdraw";
const DEFINITION: &str = flow_lib::node_definition!("presale/withdraw.jsonc");

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
    pub quote_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub presale_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner_quote_token: Pubkey,
    pub owner: Wallet,
    pub amount: u64,
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
        AccountMeta::new(input.quote_token_vault, false),
        AccountMeta::new_readonly(input.quote_mint, false),
        AccountMeta::new_readonly(input.presale_authority, false),
        AccountMeta::new(input.escrow, false),
        AccountMeta::new(input.owner_quote_token, false),
        AccountMeta::new_readonly(input.owner.pubkey(), true),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(spl_memo_interface::v3::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),
    ];

    let mut data = discriminators::WITHDRAW.to_vec();
    borsh::to_writer(&mut data, &input.amount)?;
    let remaining = RemainingAccountsInfo {
        slices: input.slices.clone(),
    };
    data.extend(borsh::to_vec(&remaining)?);

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.owner.clone()].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
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
            "quote_token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "quote_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "presale_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner_quote_token" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "amount" => 1000000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
