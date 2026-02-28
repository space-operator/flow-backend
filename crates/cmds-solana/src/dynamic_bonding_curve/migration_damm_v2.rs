use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "migration_damm_v2";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/migration_damm_v2.jsonc");

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
    pub first_position_nft_mint: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub first_position_nft_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub first_position: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    pub second_position_nft_mint: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    pub second_position_nft_account: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    pub second_position: Option<Pubkey>,
    #[serde_as(as = "AsPubkey")]
    pub damm_pool_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub amm_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_base_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_2022_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub damm_event_authority: Pubkey,
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
    let base_vault = pda::base_vault(&input.base_mint, &input.pool);
    let quote_vault = pda::quote_vault(&input.quote_mint, &input.pool);

    let mut accounts = vec![
        AccountMeta::new(input.virtual_pool, false),                       // 0: virtual_pool (writable)
        AccountMeta::new_readonly(migration_metadata, false),              // 1: migration_metadata (readonly)
        AccountMeta::new_readonly(input.config, false),                    // 2: config (readonly)
        AccountMeta::new(POOL_AUTHORITY, false),                           // 3: pool_authority (writable)
        AccountMeta::new(input.pool, false),                               // 4: pool (writable)
        AccountMeta::new(input.first_position_nft_mint.pubkey(), true),    // 5: first_position_nft_mint (writable, signer)
        AccountMeta::new(input.first_position_nft_account, false),         // 6: first_position_nft_account (writable)
        AccountMeta::new(input.first_position, false),                     // 7: first_position (writable)
    ];

    // 8-10: optional second position accounts
    if let Some(second_mint) = input.second_position_nft_mint {
        accounts.push(AccountMeta::new(second_mint, true));                // 8: second_position_nft_mint (writable, signer, optional)
    }
    if let Some(second_nft_account) = input.second_position_nft_account {
        accounts.push(AccountMeta::new(second_nft_account, false));        // 9: second_position_nft_account (writable, optional)
    }
    if let Some(second_pos) = input.second_position {
        accounts.push(AccountMeta::new(second_pos, false));                // 10: second_position (writable, optional)
    }

    accounts.extend_from_slice(&[
        AccountMeta::new_readonly(input.damm_pool_authority, false),       // 11: damm_pool_authority (readonly)
        AccountMeta::new_readonly(input.amm_program, false),               // 12: amm_program (readonly)
        AccountMeta::new(input.base_mint, false),                          // 13: base_mint (writable)
        AccountMeta::new(input.quote_mint, false),                         // 14: quote_mint (writable)
        AccountMeta::new(input.token_a_vault, false),                      // 15: token_a_vault (writable)
        AccountMeta::new(input.token_b_vault, false),                      // 16: token_b_vault (writable)
        AccountMeta::new(base_vault, false),                                // 17: base_vault (writable)
        AccountMeta::new(quote_vault, false),                              // 18: quote_vault (writable)
        AccountMeta::new(input.payer.pubkey(), true),                      // 19: payer (writable, signer)
        AccountMeta::new_readonly(input.token_base_program, false),        // 20: token_base_program (readonly)
        AccountMeta::new_readonly(input.token_quote_program, false),       // 21: token_quote_program (readonly)
        AccountMeta::new_readonly(input.token_2022_program, false),        // 22: token_2022_program (readonly)
        AccountMeta::new_readonly(input.damm_event_authority, false),      // 23: damm_event_authority (readonly)
        AccountMeta::new_readonly(input.system_program, false),            // 24: system_program (readonly)
    ]);

    let data = discriminators::MIGRATION_DAMM_V2.to_vec();

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.first_position_nft_mint, input.payer].into(),
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
