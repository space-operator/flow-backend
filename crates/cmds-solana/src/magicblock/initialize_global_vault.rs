use super::{ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "initialize_global_vault";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/initialize_global_vault.jsonc");

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
    pub mint: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub vault_ephemeral_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let vault = pda::global_vault(&input.mint);
    let vault_ephemeral_ata = pda::vault_ephemeral_ata(&vault, &input.mint);
    let vault_token =
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &vault,
            &input.mint,
            &spl_token_interface::ID,
        );

    let accounts = vec![
        AccountMeta::new(vault, false), // vault PDA (writable)
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new_readonly(input.mint, false), // mint (readonly)
        AccountMeta::new(vault_ephemeral_ata, false), // vault_ephemeral_ata PDA (writable)
        AccountMeta::new(vault_token, false), // vault_token (writable)
        AccountMeta::new_readonly(spl_token_interface::ID, false), // spl_token program
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false), // ata program
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
    ];

    let data = discriminators::INITIALIZE_GLOBAL_VAULT.to_vec();

    let instruction = Instruction {
        program_id: ETOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
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
        vault,
        vault_ephemeral_ata,
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
