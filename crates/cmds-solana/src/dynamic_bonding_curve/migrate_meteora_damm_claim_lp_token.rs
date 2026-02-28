use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "migrate_meteora_damm_claim_lp_token";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/migrate_meteora_damm_claim_lp_token.jsonc");

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
    pub pool_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub lp_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub source_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub destination_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    pub sender: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let migration_metadata = pda::migration_metadata(&input.virtual_pool);

    let accounts = vec![
        AccountMeta::new_readonly(input.virtual_pool, false),        // 0: virtual_pool (readonly)
        AccountMeta::new(migration_metadata, false),                 // 1: migration_metadata (writable)
        AccountMeta::new(input.pool_authority, false),               // 2: pool_authority (writable)
        AccountMeta::new_readonly(input.lp_mint, false),             // 3: lp_mint (readonly)
        AccountMeta::new(input.source_token, false),                 // 4: source_token (writable)
        AccountMeta::new(input.destination_token, false),            // 5: destination_token (writable)
        AccountMeta::new_readonly(input.owner, false),               // 6: owner (readonly)
        AccountMeta::new_readonly(input.sender.pubkey(), true),      // 7: sender (signer)
        AccountMeta::new_readonly(input.token_program, false),       // 8: token_program (readonly)
    ];

    let data = discriminators::MIGRATE_METEORA_DAMM_CLAIM_LP_TOKEN.to_vec();

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.sender].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, migration_metadata })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
