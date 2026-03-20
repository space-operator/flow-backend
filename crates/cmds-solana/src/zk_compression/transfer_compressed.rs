use super::photon_rpc;
use super::{to_instruction_v3, to_pubkey_v2};
use crate::prelude::*;
use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
use light_compressed_token_sdk::compressed_token::{
    CTokenAccount, TokenAccountMeta,
    transfer::instruction::{TransferInputs, transfer},
};
use light_sdk_types::instruction::PackedStateTreeInfo;

const NAME: &str = "transfer_compressed";

const DEFINITION: &str = flow_lib::node_definition!("zk_compression/transfer_compressed.jsonc");

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
    let tree_v2 = to_pubkey_v2(&input.merkle_tree);

    // 1. Fetch compressed token accounts for the owner + mint
    let accounts =
        photon_rpc::get_compressed_token_accounts_by_owner(&http, rpc_url, &owner_pk, &input.mint)
            .await
            .map_err(|e| CommandError::msg(format!("Failed to fetch compressed accounts: {e}")))?;

    if accounts.is_empty() {
        return Err(CommandError::msg(
            "No compressed token accounts found for this owner/mint",
        ));
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

    // 3. Get validity proof for selected accounts
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

    // 5. Build token account metas from the selected accounts + proof response
    let mut token_data = Vec::new();
    for (i, acc) in selected.iter().enumerate() {
        let root_index = proof_resp.root_indices.get(i).copied().unwrap_or(0) as u16;
        token_data.push(TokenAccountMeta {
            amount: acc.token_data.amount,
            delegate_index: None,
            packed_tree_info: PackedStateTreeInfo {
                root_index,
                prove_by_index: false,
                merkle_tree_pubkey_index: (i as u8), // index into tree_pubkeys
                queue_pubkey_index: (i as u8),
                leaf_index: acc.leaf_index,
            },
            lamports: None,
            tlv: None,
        });
    }

    // 6. Build CTokenAccount
    let owner_v2 = to_pubkey_v2(&owner_pk);
    let recipient_v2 = to_pubkey_v2(&input.recipient);
    let sender_account = CTokenAccount::new(mint_v2, owner_v2, token_data, 0);

    // 7. Build tree pubkeys from the proof response
    let mut tree_pubkeys = Vec::new();
    for tree_str in &proof_resp.merkle_trees {
        tree_pubkeys.push(photon_rpc::parse_pubkey_v2(tree_str).map_err(CommandError::msg)?);
    }
    // Add the output merkle tree
    tree_pubkeys.push(tree_v2);

    // 8. Build and submit transfer instruction
    let inputs = TransferInputs {
        fee_payer: to_pubkey_v2(&input.fee_payer.pubkey()),
        validity_proof,
        sender_account,
        amount: input.amount,
        recipient: recipient_v2,
        tree_pubkeys,
        config: None,
    };

    let ix_v2 = transfer(inputs)
        .map_err(|e| CommandError::msg(format!("Failed to create transfer instruction: {e}")))?;

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
