use super::{DELEGATION_PROGRAM_ID, ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "setup_and_delegate_shuttle_with_merge";
const DEFINITION: &str =
    flow_lib::node_definition!("magicblock/setup_and_delegate_shuttle_with_merge.jsonc");

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
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub shuttle_id: u32,
    pub amount: u64,
    #[serde_as(as = "AsPubkey")]
    pub destination_token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner_source_token: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub shuttle: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shuttle_ephemeral_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rent_pda = pda::rent_pda();
    let shuttle = pda::shuttle(&input.owner.pubkey(), &input.mint, input.shuttle_id);
    let shuttle_ephemeral_ata = pda::ephemeral_ata(&shuttle, &input.mint);
    let buffer = pda::delegation_buffer(&shuttle_ephemeral_ata, &ETOKEN_PROGRAM_ID);
    let delegation_record = pda::delegation_record(&shuttle_ephemeral_ata);
    let delegation_metadata = pda::delegation_metadata(&shuttle_ephemeral_ata);
    let shuttle_wallet_ata =
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &shuttle,
            &input.mint,
            &spl_token_interface::ID,
        );
    let vault = pda::global_vault(&input.mint);
    let vault_token =
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &vault,
            &input.mint,
            &spl_token_interface::ID,
        );

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new(rent_pda, false),                // rent_pda PDA (writable)
        AccountMeta::new(shuttle, false),                 // shuttle PDA (writable)
        AccountMeta::new(shuttle_ephemeral_ata, false),   // shuttle_ephemeral_ata PDA (writable)
        AccountMeta::new(shuttle_wallet_ata, false),      // shuttle_wallet_ata (writable)
        AccountMeta::new_readonly(input.owner.pubkey(), true), // owner (signer, readonly)
        AccountMeta::new_readonly(ETOKEN_PROGRAM_ID, false), // owner_program (readonly)
        AccountMeta::new(buffer, false),                  // buffer (writable)
        AccountMeta::new(delegation_record, false),       // delegation_record (writable)
        AccountMeta::new(delegation_metadata, false),     // delegation_metadata (writable)
        AccountMeta::new_readonly(DELEGATION_PROGRAM_ID, false), // delegation program
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false), // ata program
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
        AccountMeta::new(input.destination_token, false), // destination_token (writable)
        AccountMeta::new_readonly(input.mint, false),     // mint (readonly)
        AccountMeta::new_readonly(spl_token_interface::ID, false), // spl_token program
        AccountMeta::new_readonly(vault, false),          // vault PDA (readonly)
        AccountMeta::new(input.owner_source_token, false), // owner_source_token (writable)
        AccountMeta::new(vault_token, false),             // vault_token (writable)
    ];

    let mut data = discriminators::SETUP_AND_DELEGATE_SHUTTLE_WITH_MERGE.to_vec();
    data.extend(input.shuttle_id.to_le_bytes());
    data.extend(input.amount.to_le_bytes());

    let instruction = Instruction {
        program_id: ETOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.owner].into(),
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
        shuttle,
        shuttle_ephemeral_ata,
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
