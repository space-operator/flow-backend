use super::{to_instruction_v3, to_pubkey_v2};
use crate::prelude::*;
use light_compressed_token_sdk::compressed_token::mint_to_compressed::{
    MintToCompressedInputs, create_mint_to_compressed_instruction,
};
use light_token_interface::instructions::mint_action::MintWithContext;
use light_token_interface::instructions::mint_action::Recipient;

const NAME: &str = "mint_to_compressed";

const DEFINITION: &str = flow_lib::node_definition!("zk_compression/mint_to_compressed.jsonc");

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
    mint_authority: Wallet,
    #[serde(with = "value::pubkey")]
    mint: Pubkey,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    amount: u64,
    /// State merkle tree — also used as input_queue, output_queue_mint, and output_queue_tokens
    #[serde(with = "value::pubkey")]
    merkle_tree: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let payer_v2 = to_pubkey_v2(&input.fee_payer.pubkey());
    let authority_v2 = to_pubkey_v2(&input.mint_authority.pubkey());
    let recipient_v2 = to_pubkey_v2(&input.recipient);
    let tree_v2 = to_pubkey_v2(&input.merkle_tree);

    let recipients = vec![Recipient {
        recipient: recipient_v2.to_bytes().into(),
        amount: input.amount,
    }];

    // For minting to compressed without an existing compressed mint,
    // we use zeroed MintWithContext fields.
    let compressed_mint_inputs = MintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: 0,
        address: [0u8; 32],
        mint: None,
    };

    // Use the single merkle_tree for all queue/tree slots
    let inputs = MintToCompressedInputs {
        compressed_mint_inputs,
        recipients,
        mint_authority: authority_v2,
        payer: payer_v2,
        state_merkle_tree: tree_v2,
        input_queue: tree_v2,
        output_queue_mint: tree_v2,
        output_queue_tokens: tree_v2,
        decompressed_mint_config: None,
        proof: None,
        token_account_version: 0,
        cpi_context_pubkey: None,
        spl_interface_pda: None,
    };

    let ix_v2 = create_mint_to_compressed_instruction(inputs, None).map_err(|e| {
        CommandError::msg(format!(
            "Failed to create mint_to_compressed instruction: {e}"
        ))
    })?;

    let instruction = to_instruction_v3(ix_v2);

    let ins = if input.submit {
        Instructions {
            lookup_tables: None,
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer, input.mint_authority].into(),
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
