use super::{DELEGATION_PROGRAM_ID, ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "sponsored_lamports_transfer";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/sponsored_lamports_transfer.jsonc");

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
    pub destination: Pubkey,
    pub amount: u64,
    pub salt: Vec<u8>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub lamports_pda: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub rent_pda: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let salt: [u8; 32] = input
        .salt
        .try_into()
        .map_err(|_| CommandError::msg("salt must be exactly 32 bytes"))?;

    let rent_pda = pda::rent_pda();
    let lamports_pda = pda::lamports_pda(&input.fee_payer.pubkey(), &input.destination, &salt);
    let buffer = pda::delegation_buffer(&lamports_pda, &ETOKEN_PROGRAM_ID);
    let delegation_record = pda::delegation_record(&lamports_pda);
    let delegation_metadata = pda::delegation_metadata(&lamports_pda);
    let destination_delegation_record = pda::delegation_record(&input.destination);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new(rent_pda, false),                // rent_pda PDA (writable)
        AccountMeta::new(lamports_pda, false),            // lamports_pda PDA (writable)
        AccountMeta::new_readonly(ETOKEN_PROGRAM_ID, false), // owner_program (readonly)
        AccountMeta::new(buffer, false),                  // buffer (writable)
        AccountMeta::new(delegation_record, false),       // delegation_record (writable)
        AccountMeta::new(delegation_metadata, false),     // delegation_metadata (writable)
        AccountMeta::new_readonly(DELEGATION_PROGRAM_ID, false), // delegation program
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
        AccountMeta::new(input.destination, false),       // destination (writable)
        AccountMeta::new_readonly(destination_delegation_record, false), // destination_delegation_record (readonly)
    ];

    let mut data = discriminators::SPONSORED_LAMPORTS_TRANSFER.to_vec();
    data.extend(input.amount.to_le_bytes());
    data.extend(salt);

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
        lamports_pda,
        rent_pda,
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
