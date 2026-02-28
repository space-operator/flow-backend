use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "claim_protocol_fee";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/claim_protocol_fee.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_base_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_account: Pubkey,
    pub operator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_base_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_program: Pubkey,
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
    #[serde_as(as = "AsPubkey")]
    pub quote_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub claim_fee_operator: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let base_vault = pda::base_vault(&input.base_mint, &input.pool);
    let quote_vault = pda::quote_vault(&input.quote_mint, &input.pool);
    let claim_fee_operator = pda::claim_fee_operator(&input.operator.pubkey());

    let accounts = vec![
        AccountMeta::new_readonly(POOL_AUTHORITY, false),
        AccountMeta::new_readonly(input.config, false),
        AccountMeta::new(input.pool, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new_readonly(input.base_mint, false),
        AccountMeta::new_readonly(input.quote_mint, false),
        AccountMeta::new(input.token_base_account, false),
        AccountMeta::new(input.token_quote_account, false),
        AccountMeta::new_readonly(claim_fee_operator, false),
        AccountMeta::new_readonly(input.operator.pubkey(), true),
        AccountMeta::new_readonly(input.token_base_program, false),
        AccountMeta::new_readonly(input.token_quote_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    let data = discriminators::CLAIM_PROTOCOL_FEE.to_vec();

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.operator].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, base_vault, quote_vault, claim_fee_operator })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
