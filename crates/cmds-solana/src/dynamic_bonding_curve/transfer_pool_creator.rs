use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "transfer_pool_creator";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/transfer_pool_creator.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?.check_name(NAME)?.simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub virtual_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub config: Pubkey,
    pub creator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub new_creator: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output { #[serde(default, with = "value::signature::opt")] pub signature: Option<Signature> }

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let accounts = vec![
        // 0: virtual_pool (writable)
        AccountMeta::new(input.virtual_pool, false),
        // 1: config (readonly)
        AccountMeta::new_readonly(input.config, false),
        // 2: creator (signer)
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        // 3: new_creator (readonly)
        AccountMeta::new_readonly(input.new_creator, false),
        // 4: event_authority (readonly, PDA)
        AccountMeta::new_readonly(event_authority, false),
        // 5: program (readonly)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];
    let data = discriminators::TRANSFER_POOL_CREATOR.to_vec();
    let instruction = Instruction { program_id: DBC_PROGRAM_ID, accounts, data };
    let ins = Instructions { lookup_tables: None, fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(), instructions: [instruction].into() };
    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature })
}

#[cfg(test)]
mod tests { use super::*; #[test] fn test_build() { build().unwrap(); } }
