use crate::prelude::*;
use super::{to_pubkey_v2, to_instruction_v3};
use super::photon_rpc;
use super::compress_tokens::derive_ata_v2;
use light_compressed_token_sdk::compressed_token::{
    CTokenAccount, TokenAccountMeta,
    transfer::instruction::{DecompressInputs, decompress},
};
use light_compressed_token_sdk::spl_interface::derive_spl_interface_pda;
use light_compressed_token_sdk::constants::SPL_TOKEN_PROGRAM_ID;
use light_compressed_account::instruction_data::compressed_proof::{ValidityProof, CompressedProof};
use light_sdk_types::instruction::PackedStateTreeInfo;

const NAME: &str = "decompress_tokens";

const DEFINITION: &str = flow_lib::node_definition!("zk_compression/decompress_tokens.jsonc");

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
    amount: u64,
    #[serde(with = "value::pubkey")]
    merkle_tree: Pubkey,
    /// Optional: override destination token account. Defaults to owner's ATA.
    #[serde(default, with = "value::pubkey::opt")]
    destination_token_account: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rpc_url = &ctx.solana_config().url;
    let http = ctx.http().clone();

    let owner_pk = input.owner.pubkey();
    let mint_v2 = to_pubkey_v2(&input.mint);
    let owner_v2 = to_pubkey_v2(&owner_pk);
    let tree_v2 = to_pubkey_v2(&input.merkle_tree);

    // 1. Fetch compressed token accounts for the owner + mint
    let accounts = photon_rpc::get_compressed_token_accounts_by_owner(
        &http, rpc_url, &owner_pk, &input.mint,
    )
    .await
    .map_err(|e| CommandError::msg(format!("Failed to fetch compressed accounts: {e}")))?;

    if accounts.is_empty() {
        return Err(CommandError::msg("No compressed token accounts found for this owner/mint"));
    }

    // 2. Select accounts with enough balance (greedy)
    let mut selected = Vec::new();
    let mut total = 0u64;
    for acc in &accounts {
        if total >= input.amount {
            break;
        }
        selected.push(acc.clone());
        total += acc.token_data.amount;
    }
    if total < input.amount {
        return Err(CommandError::msg(format!(
            "Insufficient compressed balance: have {total}, need {}",
            input.amount
        )));
    }

    // 3. Get validity proof
    let hashes: Vec<String> = selected.iter().map(|a| a.hash.clone()).collect();
    let proof_resp = photon_rpc::get_validity_proof(&http, rpc_url, hashes)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to get validity proof: {e}")))?;

    // 4. Parse proof
    let compressed_proof = CompressedProof {
        a: photon_rpc::decode_proof_component(&proof_resp.compressed_proof.a)
            .map_err(CommandError::msg)?,
        b: photon_rpc::decode_proof_component(&proof_resp.compressed_proof.b)
            .map_err(CommandError::msg)?,
        c: photon_rpc::decode_proof_component(&proof_resp.compressed_proof.c)
            .map_err(CommandError::msg)?,
    };
    let validity_proof = ValidityProof::new(Some(compressed_proof));

    // 5. Build token account metas
    let mut token_data = Vec::new();
    for (i, acc) in selected.iter().enumerate() {
        let root_index = proof_resp.root_indices.get(i).copied().unwrap_or(0) as u16;
        token_data.push(TokenAccountMeta {
            amount: acc.token_data.amount,
            delegate_index: None,
            packed_tree_info: PackedStateTreeInfo {
                root_index,
                prove_by_index: false,
                merkle_tree_pubkey_index: (i as u8),
                queue_pubkey_index: (i as u8),
                leaf_index: acc.leaf_index,
            },
            lamports: None,
            tlv: None,
        });
    }

    // 6. Build CTokenAccount and call decompress
    let mut sender_account = CTokenAccount::new(mint_v2, owner_v2, token_data, 0);

    // 7. Tree pubkeys from proof response
    let mut tree_pubkeys = Vec::new();
    for tree_str in &proof_resp.merkle_trees {
        tree_pubkeys.push(
            photon_rpc::parse_pubkey_v2(tree_str)
                .map_err(CommandError::msg)?,
        );
    }
    // Add the output merkle tree
    tree_pubkeys.push(tree_v2);

    // 8. Derive SPL interface PDA (token pool) and recipient token account
    let pda = derive_spl_interface_pda(&mint_v2, 0, false);

    let recipient_ata = if let Some(dest) = input.destination_token_account {
        to_pubkey_v2(&dest)
    } else {
        derive_ata_v2(&owner_v2, &mint_v2)
    };

    let inputs = DecompressInputs {
        fee_payer: to_pubkey_v2(&input.fee_payer.pubkey()),
        validity_proof,
        sender_account,
        amount: input.amount,
        tree_pubkeys,
        config: None,
        spl_interface_pda: pda.pubkey,
        recipient_token_account: recipient_ata,
        spl_token_program: SPL_TOKEN_PROGRAM_ID,
    };

    let ix_v2 = decompress(inputs)
        .map_err(|e| CommandError::msg(format!("Failed to create decompress instruction: {e}")))?;

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
