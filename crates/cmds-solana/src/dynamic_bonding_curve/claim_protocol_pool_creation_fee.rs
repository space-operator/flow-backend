use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, pda, discriminators};

const NAME: &str = "claim_protocol_pool_creation_fee";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/claim_protocol_pool_creation_fee.jsonc");

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
    pub config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    pub signer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub treasury: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub claim_fee_operator: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let claim_fee_operator = pda::claim_fee_operator(&input.signer.pubkey());

    let accounts = vec![
        AccountMeta::new_readonly(input.config, false),
        AccountMeta::new(input.pool, false),
        AccountMeta::new_readonly(claim_fee_operator, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(input.treasury, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let data = discriminators::CLAIM_PROTOCOL_POOL_CREATION_FEE.to_vec();

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.signer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, claim_fee_operator })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
