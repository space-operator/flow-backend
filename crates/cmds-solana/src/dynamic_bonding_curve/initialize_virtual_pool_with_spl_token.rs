use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{DBC_PROGRAM_ID, POOL_AUTHORITY, pda, discriminators};

const NAME: &str = "initialize_virtual_pool_with_spl_token";
const DEFINITION: &str = flow_lib::node_definition!("dynamic_bonding_curve/initialize_virtual_pool_with_spl_token.jsonc");

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
    pub creator: Wallet,
    pub base_mint: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint_metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub metadata_program: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_quote_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    /// Metadata URI
    pub uri: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = pda::event_authority();
    let pool = pda::pool(&input.config, &input.base_mint.pubkey(), &input.quote_mint);
    let base_vault = pda::base_vault(&input.base_mint.pubkey(), &pool);
    let quote_vault = pda::quote_vault(&input.quote_mint, &pool);

    let accounts = vec![
        // 0: config (readonly)
        AccountMeta::new_readonly(input.config, false),
        // 1: pool_authority (readonly)
        AccountMeta::new_readonly(POOL_AUTHORITY, false),
        // 2: creator (signer)
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        // 3: base_mint (writable, signer - new keypair)
        AccountMeta::new(input.base_mint.pubkey(), true),
        // 4: quote_mint (readonly)
        AccountMeta::new_readonly(input.quote_mint, false),
        // 5: pool (writable)
        AccountMeta::new(pool, false),
        // 6: base_vault (writable)
        AccountMeta::new(base_vault, false),
        // 7: quote_vault (writable)
        AccountMeta::new(quote_vault, false),
        // 8: mint_metadata (writable)
        AccountMeta::new(input.mint_metadata, false),
        // 9: metadata_program (readonly)
        AccountMeta::new_readonly(input.metadata_program, false),
        // 10: payer (writable, signer)
        AccountMeta::new(input.payer.pubkey(), true),
        // 11: token_quote_program (readonly)
        AccountMeta::new_readonly(input.token_quote_program, false),
        // 12: token_program (readonly)
        AccountMeta::new_readonly(input.token_program, false),
        // 13: system_program (readonly)
        AccountMeta::new_readonly(input.system_program, false),
        // 14: event_authority (readonly, PDA)
        AccountMeta::new_readonly(event_authority, false),
        // 15: program (readonly)
        AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
    ];

    // InitializePoolParameters { name, symbol, uri }
    let mut data = discriminators::INITIALIZE_VIRTUAL_POOL_WITH_SPL_TOKEN.to_vec();
    data.extend(borsh::to_vec(&input.name)?);
    data.extend(borsh::to_vec(&input.symbol)?);
    data.extend(borsh::to_vec(&input.uri)?);

    let instruction = Instruction {
        program_id: DBC_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator, input.base_mint, input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, pool, base_vault, quote_vault })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
