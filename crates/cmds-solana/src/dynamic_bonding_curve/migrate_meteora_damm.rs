use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "migrate_meteora_damm";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/migrate_meteora_damm.jsonc");

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
    pub virtual_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub damm_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub lp_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub a_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub b_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub a_vault_lp_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub b_vault_lp_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub a_vault_lp: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub b_vault_lp: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub virtual_pool_lp: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub protocol_token_a_fee: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub protocol_token_b_fee: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub rent: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint_metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub metadata_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub amm_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub vault_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub associated_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub migration_metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let migration_metadata = pda::migration_metadata(&input.virtual_pool);
    let base_vault = pda::base_vault(&input.token_a_mint, &input.pool);
    let quote_vault = pda::quote_vault(&input.token_b_mint, &input.pool);

    let accounts = vec![
        AccountMeta::new(input.virtual_pool, false),              // 0: virtual_pool (writable)
        AccountMeta::new(migration_metadata, false),              // 1: migration_metadata (writable)
        AccountMeta::new_readonly(input.config, false),           // 2: config (readonly)
        AccountMeta::new(POOL_AUTHORITY, false),                  // 3: pool_authority (writable)
        AccountMeta::new(input.pool, false),                      // 4: pool (writable)
        AccountMeta::new_readonly(input.damm_config, false),      // 5: damm_config (readonly)
        AccountMeta::new(input.lp_mint, false),                   // 6: lp_mint (writable)
        AccountMeta::new(input.token_a_mint, false),              // 7: token_a_mint (writable)
        AccountMeta::new_readonly(input.token_b_mint, false),     // 8: token_b_mint (readonly)
        AccountMeta::new(input.a_vault, false),                   // 9: a_vault (writable)
        AccountMeta::new(input.b_vault, false),                   // 10: b_vault (writable)
        AccountMeta::new(input.a_token_vault, false),             // 11: a_token_vault (writable)
        AccountMeta::new(input.b_token_vault, false),             // 12: b_token_vault (writable)
        AccountMeta::new(input.a_vault_lp_mint, false),           // 13: a_vault_lp_mint (writable)
        AccountMeta::new(input.b_vault_lp_mint, false),           // 14: b_vault_lp_mint (writable)
        AccountMeta::new(input.a_vault_lp, false),                // 15: a_vault_lp (writable)
        AccountMeta::new(input.b_vault_lp, false),                // 16: b_vault_lp (writable)
        AccountMeta::new(base_vault, false),                       // 17: base_vault (writable)
        AccountMeta::new(quote_vault, false),                     // 18: quote_vault (writable)
        AccountMeta::new(input.virtual_pool_lp, false),           // 19: virtual_pool_lp (writable)
        AccountMeta::new(input.protocol_token_a_fee, false),      // 20: protocol_token_a_fee (writable)
        AccountMeta::new(input.protocol_token_b_fee, false),      // 21: protocol_token_b_fee (writable)
        AccountMeta::new(input.payer.pubkey(), true),              // 22: payer (writable, signer)
        AccountMeta::new_readonly(input.rent, false),             // 23: rent (readonly)
        AccountMeta::new(input.mint_metadata, false),             // 24: mint_metadata (writable)
        AccountMeta::new_readonly(input.metadata_program, false), // 25: metadata_program (readonly)
        AccountMeta::new_readonly(input.amm_program, false),      // 26: amm_program (readonly)
        AccountMeta::new_readonly(input.vault_program, false),    // 27: vault_program (readonly)
        AccountMeta::new_readonly(input.token_program, false),    // 28: token_program (readonly)
        AccountMeta::new_readonly(input.associated_token_program, false), // 29: associated_token_program (readonly)
        AccountMeta::new_readonly(input.system_program, false),   // 30: system_program (readonly)
    ];

    let data = discriminators::MIGRATE_METEORA_DAMM.to_vec();

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, migration_metadata, base_vault, quote_vault })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
