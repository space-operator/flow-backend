use super::{ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "initialize_shuttle_ephemeral_ata";
const DEFINITION: &str =
    flow_lib::node_definition!("magicblock/initialize_shuttle_ephemeral_ata.jsonc");

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
    pub owner: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub shuttle_id: u32,
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
    let shuttle = pda::shuttle(&input.owner, &input.mint, input.shuttle_id);
    let shuttle_ephemeral_ata = pda::ephemeral_ata(&shuttle, &input.mint);
    let shuttle_wallet_ata =
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &shuttle,
            &input.mint,
            &spl_token_interface::ID,
        );

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new(shuttle, false),                 // shuttle PDA (writable)
        AccountMeta::new(shuttle_ephemeral_ata, false),   // shuttle_ephemeral_ata PDA (writable)
        AccountMeta::new(shuttle_wallet_ata, false),      // shuttle_wallet_ata (writable)
        AccountMeta::new_readonly(input.owner, false),    // owner (readonly)
        AccountMeta::new_readonly(input.mint, false),     // mint (readonly)
        AccountMeta::new_readonly(spl_token_interface::ID, false), // spl_token program
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false), // ata program
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
    ];

    let mut data = discriminators::INITIALIZE_SHUTTLE_EPHEMERAL_ATA.to_vec();
    data.extend(input.shuttle_id.to_le_bytes());

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
