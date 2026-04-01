use super::{DELEGATION_PROGRAM_ID, ETOKEN_PROGRAM_ID, PERMISSION_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "delegate_ephemeral_ata_permission";
const DEFINITION: &str =
    flow_lib::node_definition!("magicblock/delegate_ephemeral_ata_permission.jsonc");

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
    pub permission: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub validator: Pubkey,
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
    let buffer = pda::delegation_buffer(&input.permission, &PERMISSION_PROGRAM_ID);
    let delegation_record = pda::delegation_record(&input.permission);
    let delegation_metadata = pda::delegation_metadata(&input.permission);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new(ephemeral_ata, false),           // ephemeral_ata PDA (writable)
        AccountMeta::new_readonly(PERMISSION_PROGRAM_ID, false), // permission_program (readonly)
        AccountMeta::new(input.permission, false),        // permission (writable)
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
        AccountMeta::new(buffer, false),                  // buffer (writable)
        AccountMeta::new(delegation_record, false),       // delegation_record (writable)
        AccountMeta::new(delegation_metadata, false),     // delegation_metadata (writable)
        AccountMeta::new_readonly(DELEGATION_PROGRAM_ID, false), // delegation_program (readonly)
        AccountMeta::new_readonly(input.validator, false), // validator (readonly)
    ];

    let data = discriminators::DELEGATE_EPHEMERAL_ATA_PERMISSION.to_vec();

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
