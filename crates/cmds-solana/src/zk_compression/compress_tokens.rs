use super::{to_instruction_v3, to_pubkey_v2};
use crate::prelude::*;
use light_compressed_token_sdk::compressed_token::batch_compress::{
    BatchCompressInputs, Recipient, create_batch_compress_instruction,
};
use light_compressed_token_sdk::constants::SPL_TOKEN_PROGRAM_ID;
use light_compressed_token_sdk::spl_interface::derive_spl_interface_pda;

/// Derive the Associated Token Account (ATA) address using v2 pubkey types.
/// ATA PDA seeds: [wallet_address, token_program_id, mint_address]
/// ATA program: ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe8bXh (v2 bytes)
pub fn derive_ata_v2(
    owner: &solana_program_v2::pubkey::Pubkey,
    mint: &solana_program_v2::pubkey::Pubkey,
) -> solana_program_v2::pubkey::Pubkey {
    const ATA_PROGRAM_ID: [u8; 32] = [
        140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153,
        218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
    ];
    let program_id = solana_program_v2::pubkey::Pubkey::new_from_array(ATA_PROGRAM_ID);
    let spl_token_id = SPL_TOKEN_PROGRAM_ID;
    let seeds: &[&[u8]] = &[owner.as_ref(), spl_token_id.as_ref(), mint.as_ref()];
    solana_program_v2::pubkey::Pubkey::find_program_address(seeds, &program_id).0
}

const NAME: &str = "compress_tokens";

const DEFINITION: &str = flow_lib::node_definition!("zk_compression/compress_tokens.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    owner: Wallet,
    #[serde(with = "value::pubkey")]
    mint: Pubkey,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    amount: u64,
    #[serde(with = "value::pubkey")]
    merkle_tree: Pubkey,
    /// Optional: override source token account. If omitted, the owner's ATA is used.
    #[serde(default, with = "value::pubkey::opt")]
    source_token_account: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let fee_payer_v2 = to_pubkey_v2(&input.fee_payer.pubkey());
    let authority_v2 = to_pubkey_v2(&input.owner.pubkey());
    let mint_v2 = to_pubkey_v2(&input.mint);
    let recipient_v2 = to_pubkey_v2(&input.recipient);
    let tree_v2 = to_pubkey_v2(&input.merkle_tree);

    // Derive source token account from owner's ATA if not provided
    let source_v2 = if let Some(ata) = input.source_token_account {
        to_pubkey_v2(&ata)
    } else {
        derive_ata_v2(&authority_v2, &mint_v2)
    };

    // Derive the SPL interface PDA (token pool) from the mint
    let pda = derive_spl_interface_pda(&mint_v2, 0, false);
    let token_program_v2 = SPL_TOKEN_PROGRAM_ID;

    let recipients = vec![Recipient {
        pubkey: recipient_v2,
        amount: input.amount,
    }];

    let inputs = BatchCompressInputs {
        fee_payer: fee_payer_v2,
        authority: authority_v2,
        spl_interface_pda: pda.pubkey,
        sender_token_account: source_v2,
        token_program: token_program_v2,
        merkle_tree: tree_v2,
        recipients,
        lamports: None,
        token_pool_index: pda.index,
        token_pool_bump: pda.bump,
        sol_pool_pda: None,
    };

    let ix_v2 = create_batch_compress_instruction(inputs).map_err(|e| {
        CommandError::msg(format!("Failed to create batch_compress instruction: {e}"))
    })?;

    let instruction = to_instruction_v3(ix_v2);

    let ins = if input.submit {
        Instructions {
            lookup_tables: None,
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer, input.owner].into(),
            instructions: [instruction].into(),
        }
    } else {
        <_>::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
