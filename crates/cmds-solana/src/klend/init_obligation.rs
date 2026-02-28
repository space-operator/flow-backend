use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KLEND_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_obligation, derive_user_metadata};

const NAME: &str = "init_obligation";
const DEFINITION: &str = flow_lib::node_definition!("klend/init_obligation.jsonc");

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
    pub obligation_owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub lending_market: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seed1_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub seed2_account: Pubkey,
    pub tag: u8,
    pub id: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub obligation: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner_user_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let obligation = derive_obligation(&input.lending_market, &input.obligation_owner.pubkey(), &input.seed1_account, &input.seed2_account);
    let owner_user_metadata = derive_user_metadata(&input.obligation_owner.pubkey());

    let accounts = vec![
        AccountMeta::new(input.obligation_owner.pubkey(), true),
        AccountMeta::new(obligation, false),
        AccountMeta::new(input.lending_market, false),
        AccountMeta::new_readonly(input.seed1_account, false),
        AccountMeta::new_readonly(input.seed2_account, false),
        AccountMeta::new(owner_user_metadata, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.tag)?);
    data.extend(borsh::to_vec(&input.id)?);

    let instruction = Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.obligation_owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, obligation, owner_user_metadata })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, obligation_owner, lending_market, seed1_account, seed2_account, tag, id
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "obligation_owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "lending_market" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "seed1_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "seed2_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tag" => 0_u8,
            "id" => 0_u8,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
