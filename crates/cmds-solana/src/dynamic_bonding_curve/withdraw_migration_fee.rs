use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};


const NAME: &str = "withdraw_migration_fee";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/withdraw_migration_fee.jsonc");

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
    pub token_quote_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    pub sender: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_program: Pubkey,
    pub flag: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub quote_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let quote_vault = pda::quote_vault(&input.quote_mint, &input.virtual_pool);
    let accounts = vec![
        // 0: pool_authority (readonly)
        AccountMeta::new_readonly(POOL_AUTHORITY, false),
        // 1: config (readonly)
        AccountMeta::new_readonly(input.config, false),
        // 2: virtual_pool (writable)
        AccountMeta::new(input.virtual_pool, false),
        // 3: token_quote_account (writable)
        AccountMeta::new(input.token_quote_account, false),
        // 4: quote_vault (writable)
        AccountMeta::new(quote_vault, false),
        // 5: quote_mint (readonly)
        AccountMeta::new_readonly(input.quote_mint, false),
        // 6: sender (signer)
        AccountMeta::new_readonly(input.sender.pubkey(), true),
        // 7: token_quote_program (readonly)
        AccountMeta::new_readonly(input.token_quote_program, false),
        // 8: event_authority (readonly, PDA)
        AccountMeta::new_readonly(event_authority, false),
        // 9: program (readonly)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];
    let mut data = discriminators::WITHDRAW_MIGRATION_FEE.to_vec();
    data.extend(borsh::to_vec(&input.flag)?);
    let instruction = Instruction { program_id: DBC_PROGRAM_ID, accounts, data };
    let ins = Instructions { lookup_tables: None, fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.sender].into(), instructions: [instruction].into() };
    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, quote_vault })
}

#[cfg(test)]
mod tests { use super::*; #[test] fn test_build() { build().unwrap(); } }
