use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "migration_meteora_damm_create_metadata";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/migration_meteora_damm_create_metadata.jsonc");

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
    pub payer: Wallet,
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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let migration_metadata = pda::migration_metadata(&input.virtual_pool);

    let accounts = vec![
        AccountMeta::new_readonly(input.virtual_pool, false),    // 0: virtual_pool (readonly)
        AccountMeta::new_readonly(input.config, false),          // 1: config (readonly)
        AccountMeta::new(migration_metadata, false),             // 2: migration_metadata (writable)
        AccountMeta::new(input.payer.pubkey(), true),            // 3: payer (writable, signer)
        AccountMeta::new_readonly(input.system_program, false),  // 4: system_program (readonly)
        AccountMeta::new_readonly(event_authority, false),       // 5: event_authority (readonly, PDA)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),        // 6: program (readonly)
    ];

    let data = discriminators::MIGRATION_METEORA_DAMM_CREATE_METADATA.to_vec();

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
