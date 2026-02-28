use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "swap";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/swap.jsonc");

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
    pub input_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub output_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_base_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_program: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    pub referral_token_account: Option<Pubkey>,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive PDAs
    let event_authority = pda::event_authority();
    let base_vault = pda::base_vault(&input.base_mint, &input.pool);
    let quote_vault = pda::quote_vault(&input.quote_mint, &input.pool);

    // Build accounts list matching IDL order
    let mut accounts = vec![
        AccountMeta::new_readonly(POOL_AUTHORITY, false),     // pool_authority (constant)
        AccountMeta::new_readonly(input.config, false),        // config
        AccountMeta::new(input.pool, false),                   // pool (writable)
        AccountMeta::new(input.input_token_account, false),    // input_token_account (writable)
        AccountMeta::new(input.output_token_account, false),   // output_token_account (writable)
        AccountMeta::new(base_vault, false),                     // base_vault (writable)
        AccountMeta::new(quote_vault, false),                   // quote_vault (writable)
        AccountMeta::new_readonly(input.base_mint, false),     // base_mint
        AccountMeta::new_readonly(input.quote_mint, false),    // quote_mint
        AccountMeta::new_readonly(input.payer.pubkey(), true), // payer (signer)
        AccountMeta::new_readonly(input.token_base_program, false),  // token_base_program
        AccountMeta::new_readonly(input.token_quote_program, false), // token_quote_program
    ];
    
    // Add optional referral account
    if let Some(referral) = input.referral_token_account {
        accounts.push(AccountMeta::new(referral, false));
    }
    
    accounts.push(AccountMeta::new_readonly(event_authority, false)); // event_authority (PDA)
    accounts.push(AccountMeta::new_readonly(DBC_PROGRAM_ID, false));  // program

    // Build instruction data: discriminator + SwapParameters
    // SwapParameters { amount_in: u64, minimum_amount_out: u64 }
    let mut data = discriminators::SWAP.to_vec();
    data.extend(borsh::to_vec(&input.amount_in)?);
    data.extend(borsh::to_vec(&input.minimum_amount_out)?);

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
    Ok(Output { signature, base_vault, quote_vault })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
