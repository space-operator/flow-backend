use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "create_virtual_pool_metadata";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/create_virtual_pool_metadata.jsonc");

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
    pub pool: Pubkey,
    pub creator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    pub padding: Vec<u64>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub virtual_pool_metadata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let virtual_pool_metadata = pda::virtual_pool_metadata(&input.pool);

    let accounts = vec![
        AccountMeta::new(virtual_pool_metadata, false),
        AccountMeta::new_readonly(input.pool, false),
        AccountMeta::new(input.creator.pubkey(), true),
        AccountMeta::new_readonly(input.system_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let mut data = discriminators::CREATE_VIRTUAL_POOL_METADATA.to_vec();
    data.extend(borsh::to_vec(&input.padding)?);

    let instruction = Instruction { program_id: DBC_PROGRAM_ID, accounts, data };
    let ins = Instructions {
        lookup_tables: None, fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(), instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, virtual_pool_metadata })
}

#[cfg(test)]
mod tests { use super::*; #[test] fn test_build() { build().unwrap(); } }
