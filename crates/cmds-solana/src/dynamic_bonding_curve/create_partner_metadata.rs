use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "create_partner_metadata";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/create_partner_metadata.jsonc");

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
    pub fee_claimer: Pubkey,
    pub creator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    pub name: String,
    pub website: String,
    pub logo: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub partner_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let partner_metadata = pda::partner_metadata(&input.fee_claimer);

    let accounts = vec![
        AccountMeta::new(partner_metadata, false),
        AccountMeta::new_readonly(input.fee_claimer, false),
        AccountMeta::new(input.creator.pubkey(), true),
        AccountMeta::new_readonly(input.system_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let mut data = discriminators::CREATE_PARTNER_METADATA.to_vec();
    data.extend(borsh::to_vec(&input.name)?);
    data.extend(borsh::to_vec(&input.website)?);
    data.extend(borsh::to_vec(&input.logo)?);

    let instruction = Instruction { program_id: DBC_PROGRAM_ID, accounts, data };
    let ins = Instructions {
        lookup_tables: None, fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(), instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, partner_metadata })
}

#[cfg(test)]
mod tests { use super::*; #[test] fn test_build() { build().unwrap(); } }
