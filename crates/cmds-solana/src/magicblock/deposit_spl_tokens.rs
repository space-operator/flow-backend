use super::{ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "deposit_spl_tokens";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/deposit_spl_tokens.jsonc");

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
    pub user: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub source_token: Pubkey,
    pub amount: u64,
    pub authority: Wallet,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub ephemeral_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ephemeral_ata = pda::ephemeral_ata(&input.user, &input.mint);
    let vault = pda::global_vault(&input.mint);
    let vault_token =
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &vault,
            &input.mint,
            &spl_token_interface::ID,
        );

    let accounts = vec![
        AccountMeta::new(ephemeral_ata, false), // ephemeral_ata PDA (writable)
        AccountMeta::new_readonly(vault, false), // vault PDA (readonly)
        AccountMeta::new_readonly(input.mint, false), // mint (readonly)
        AccountMeta::new(input.source_token, false), // source_token (writable)
        AccountMeta::new(vault_token, false),   // vault_token (writable)
        AccountMeta::new_readonly(input.authority.pubkey(), true), // authority (signer, readonly)
        AccountMeta::new_readonly(spl_token_interface::ID, false), // spl_token program
    ];

    let mut data = discriminators::DEPOSIT_SPL_TOKENS.to_vec();
    data.extend(input.amount.to_le_bytes());

    let instruction = Instruction {
        program_id: ETOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        ephemeral_ata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
