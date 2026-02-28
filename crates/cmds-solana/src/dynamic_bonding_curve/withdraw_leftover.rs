use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "withdraw_leftover";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/withdraw_leftover.jsonc");

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
    pub config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub virtual_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_base_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub leftover_receiver: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_base_program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub base_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let base_vault = pda::base_vault(&input.base_mint, &input.virtual_pool);
    let accounts = vec![
        // 0: pool_authority (readonly)
        AccountMeta::new_readonly(POOL_AUTHORITY, false),
        // 1: config (readonly)
        AccountMeta::new_readonly(input.config, false),
        // 2: virtual_pool (writable)
        AccountMeta::new(input.virtual_pool, false),
        // 3: token_base_account (writable)
        AccountMeta::new(input.token_base_account, false),
        // 4: base_vault (writable)
        AccountMeta::new(base_vault, false),
        // 5: base_mint (readonly)
        AccountMeta::new_readonly(input.base_mint, false),
        // 6: leftover_receiver (readonly, NOT a signer)
        AccountMeta::new_readonly(input.leftover_receiver, false),
        // 7: token_base_program (readonly)
        AccountMeta::new_readonly(input.token_base_program, false),
        // 8: event_authority (readonly, PDA)
        AccountMeta::new_readonly(event_authority, false),
        // 9: program (readonly)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];
    let data = discriminators::WITHDRAW_LEFTOVER.to_vec();
    let instruction = Instruction { program_id: DBC_PROGRAM_ID, accounts, data };
    let ins = Instructions { lookup_tables: None, fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(), instructions: [instruction].into() };
    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, base_vault })
}

#[cfg(test)]
mod tests { use super::*; #[test] fn test_build() { build().unwrap(); } }
