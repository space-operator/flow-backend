use super::{helper, pda};
use crate::prelude::*;
use borsh::BorshSerialize;
use solana_program::instruction::AccountMeta;

const NAME: &str = "transact";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/transact.jsonc");

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
    pub signer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub recipient: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub fee_recipient_account: Pubkey,
    /// Groth16 ZK proof fields (JSON object)
    pub proof: JsonValue,
    /// Minified ext data: { ext_amount: i64, fee: u64 }
    pub ext_data_minified: JsonValue,
    /// First encrypted output commitment (hex or byte array)
    pub encrypted_output1: Vec<u8>,
    /// Second encrypted output commitment (hex or byte array)
    pub encrypted_output2: Vec<u8>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// Parse 32-byte array from JSON (array of numbers).
fn parse_bytes32(val: &JsonValue, field: &str) -> Result<[u8; 32], CommandError> {
    let arr = val
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or_else(|| CommandError::msg(format!("missing proof field: {field}")))?;
    let bytes: Vec<u8> = arr.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect();
    bytes
        .try_into()
        .map_err(|_| CommandError::msg(format!("{field} must be 32 bytes")))
}

/// Parse fixed-size byte array from JSON.
fn parse_bytes<const N: usize>(val: &JsonValue, field: &str) -> Result<[u8; N], CommandError> {
    let arr = val
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or_else(|| CommandError::msg(format!("missing proof field: {field}")))?;
    let bytes: Vec<u8> = arr.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect();
    bytes
        .try_into()
        .map_err(|_| CommandError::msg(format!("{field} must be {N} bytes")))
}

/// Parse the Proof struct from JSON.
fn parse_proof(val: &JsonValue) -> Result<helper::Proof, CommandError> {
    let nullifier0 = parse_bytes32(val, "input_nullifier_0").or_else(|_| {
        let nullifiers = val.get("input_nullifiers").and_then(|v| v.as_array());
        nullifiers
            .and_then(|arr| arr.first())
            .ok_or_else(|| CommandError::msg("missing input_nullifiers[0]"))
            .and_then(|v| {
                let bytes: Vec<u8> = v
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u8)
                    .collect();
                bytes
                    .try_into()
                    .map_err(|_| CommandError::msg("nullifier must be 32 bytes"))
            })
    })?;

    let nullifier1 = parse_bytes32(val, "input_nullifier_1").or_else(|_| {
        let nullifiers = val.get("input_nullifiers").and_then(|v| v.as_array());
        nullifiers
            .and_then(|arr| arr.get(1))
            .ok_or_else(|| CommandError::msg("missing input_nullifiers[1]"))
            .and_then(|v| {
                let bytes: Vec<u8> = v
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u8)
                    .collect();
                bytes
                    .try_into()
                    .map_err(|_| CommandError::msg("nullifier must be 32 bytes"))
            })
    })?;

    let commitment0 = parse_bytes32(val, "output_commitment_0").or_else(|_| {
        let commitments = val.get("output_commitments").and_then(|v| v.as_array());
        commitments
            .and_then(|arr| arr.first())
            .ok_or_else(|| CommandError::msg("missing output_commitments[0]"))
            .and_then(|v| {
                let bytes: Vec<u8> = v
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u8)
                    .collect();
                bytes
                    .try_into()
                    .map_err(|_| CommandError::msg("commitment must be 32 bytes"))
            })
    })?;

    let commitment1 = parse_bytes32(val, "output_commitment_1").or_else(|_| {
        let commitments = val.get("output_commitments").and_then(|v| v.as_array());
        commitments
            .and_then(|arr| arr.get(1))
            .ok_or_else(|| CommandError::msg("missing output_commitments[1]"))
            .and_then(|v| {
                let bytes: Vec<u8> = v
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u8)
                    .collect();
                bytes
                    .try_into()
                    .map_err(|_| CommandError::msg("commitment must be 32 bytes"))
            })
    })?;

    Ok(helper::Proof {
        proof_a: parse_bytes(val, "proof_a")?,
        proof_b: parse_bytes(val, "proof_b")?,
        proof_c: parse_bytes(val, "proof_c")?,
        root: parse_bytes32(val, "root")?,
        public_amount: parse_bytes32(val, "public_amount")?,
        ext_data_hash: parse_bytes32(val, "ext_data_hash")?,
        input_nullifiers: [nullifier0, nullifier1],
        output_commitments: [commitment0, commitment1],
    })
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let proof = parse_proof(&input.proof)?;

    let ext_amount = input
        .ext_data_minified
        .get("ext_amount")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| CommandError::msg("missing ext_data_minified.ext_amount"))?;
    let fee = input
        .ext_data_minified
        .get("fee")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| CommandError::msg("missing ext_data_minified.fee"))?;
    let ext_data = helper::ExtDataMinified { ext_amount, fee };

    let (tree_account, _) = pda::find_merkle_tree();
    let (nullifier0, _) = pda::find_nullifier0(&proof.input_nullifiers[0]);
    let (nullifier1, _) = pda::find_nullifier1(&proof.input_nullifiers[1]);
    // Cross-nullifier PDAs for replay protection
    let (nullifier2, _) = pda::find_nullifier0(&proof.input_nullifiers[1]);
    let (nullifier3, _) = pda::find_nullifier1(&proof.input_nullifiers[0]);
    let (tree_token_account, _) = pda::find_tree_token();
    let (global_config, _) = pda::find_global_config();

    tracing::info!(
        "transact: signer={}, recipient={}, ext_amount={}, fee={}",
        input.signer.pubkey(),
        input.recipient,
        ext_amount,
        fee
    );

    // Accounts: Transact context (order must match on-chain struct)
    let accounts = vec![
        AccountMeta::new(tree_account, false),           // tree_account
        AccountMeta::new(nullifier0, false),             // nullifier0 (init)
        AccountMeta::new(nullifier1, false),             // nullifier1 (init)
        AccountMeta::new_readonly(nullifier2, false),    // nullifier2
        AccountMeta::new_readonly(nullifier3, false),    // nullifier3
        AccountMeta::new(tree_token_account, false),     // tree_token_account
        AccountMeta::new_readonly(global_config, false), // global_config
        AccountMeta::new(input.recipient, false),        // recipient
        AccountMeta::new(input.fee_recipient_account, false), // fee_recipient_account
        AccountMeta::new(input.signer.pubkey(), true),   // signer
        AccountMeta::new_readonly(helper::system_program(), false), // system_program
    ];

    let mut args_data = Vec::new();
    proof.serialize(&mut args_data)?;
    ext_data.serialize(&mut args_data)?;
    BorshSerialize::serialize(&input.encrypted_output1, &mut args_data)?;
    BorshSerialize::serialize(&input.encrypted_output2, &mut args_data)?;

    let instruction = helper::build_instruction("transact", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;
    use solana_program::instruction::AccountMeta;

    /// Build a valid dummy proof JSON with flat field names.
    fn dummy_proof_json_flat() -> JsonValue {
        let root = vec![1u8; 32];
        let public_amount = vec![2u8; 32];
        let ext_data_hash = vec![3u8; 32];
        let null0 = vec![4u8; 32];
        let null1 = vec![5u8; 32];
        let comm0 = vec![6u8; 32];
        let comm1 = vec![7u8; 32];
        serde_json::json!({
            "proof_a": (0..64).collect::<Vec<u8>>(),
            "proof_b": (0..128).collect::<Vec<u8>>(),
            "proof_c": (0..64).collect::<Vec<u8>>(),
            "root": root,
            "public_amount": public_amount,
            "ext_data_hash": ext_data_hash,
            "input_nullifier_0": null0,
            "input_nullifier_1": null1,
            "output_commitment_0": comm0,
            "output_commitment_1": comm1,
        })
    }

    /// Build a valid dummy proof JSON with nested array field names.
    fn dummy_proof_json_nested() -> JsonValue {
        let root = vec![1u8; 32];
        let public_amount = vec![2u8; 32];
        let ext_data_hash = vec![3u8; 32];
        let null0 = vec![4u8; 32];
        let null1 = vec![5u8; 32];
        let comm0 = vec![6u8; 32];
        let comm1 = vec![7u8; 32];
        serde_json::json!({
            "proof_a": (0..64).collect::<Vec<u8>>(),
            "proof_b": (0..128).collect::<Vec<u8>>(),
            "proof_c": (0..64).collect::<Vec<u8>>(),
            "root": root,
            "public_amount": public_amount,
            "ext_data_hash": ext_data_hash,
            "input_nullifiers": [null0, null1],
            "output_commitments": [comm0, comm1],
        })
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_parse_proof_flat_format() {
        let json = dummy_proof_json_flat();
        let proof = parse_proof(&json).unwrap();
        assert_eq!(proof.root, [1u8; 32]);
        assert_eq!(proof.public_amount, [2u8; 32]);
        assert_eq!(proof.ext_data_hash, [3u8; 32]);
        assert_eq!(proof.input_nullifiers[0], [4u8; 32]);
        assert_eq!(proof.input_nullifiers[1], [5u8; 32]);
        assert_eq!(proof.output_commitments[0], [6u8; 32]);
        assert_eq!(proof.output_commitments[1], [7u8; 32]);
    }

    #[test]
    fn test_parse_proof_nested_format() {
        let json = dummy_proof_json_nested();
        let proof = parse_proof(&json).unwrap();
        assert_eq!(proof.input_nullifiers[0], [4u8; 32]);
        assert_eq!(proof.input_nullifiers[1], [5u8; 32]);
        assert_eq!(proof.output_commitments[0], [6u8; 32]);
        assert_eq!(proof.output_commitments[1], [7u8; 32]);
    }

    #[test]
    fn test_parse_proof_wrong_size_fails() {
        let z32 = vec![0u8; 32];
        let json = serde_json::json!({
            "proof_a": vec![0u8; 64],
            "proof_b": vec![0u8; 128],
            "proof_c": vec![0u8; 64],
            "root": vec![0u8; 31],  // Wrong size!
            "public_amount": z32.clone(),
            "ext_data_hash": z32.clone(),
            "input_nullifiers": [z32.clone(), z32.clone()],
            "output_commitments": [z32.clone(), z32],
        });
        assert!(parse_proof(&json).is_err());
    }

    #[test]
    fn test_instruction_construction() {
        let signer: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();
        let recipient: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        let fee_recipient: Pubkey = "So11111111111111111111111111111111111111112"
            .parse()
            .unwrap();

        let proof = helper::Proof {
            proof_a: [0u8; 64],
            proof_b: [0u8; 128],
            proof_c: [0u8; 64],
            root: [1u8; 32],
            public_amount: [2u8; 32],
            ext_data_hash: [3u8; 32],
            input_nullifiers: [[4u8; 32], [5u8; 32]],
            output_commitments: [[6u8; 32], [7u8; 32]],
        };
        let ext_data = helper::ExtDataMinified {
            ext_amount: 100_000,
            fee: 500,
        };

        let (tree_account, _) = pda::find_merkle_tree();
        let (nullifier0, _) = pda::find_nullifier0(&proof.input_nullifiers[0]);
        let (nullifier1, _) = pda::find_nullifier1(&proof.input_nullifiers[1]);
        let (nullifier2, _) = pda::find_nullifier0(&proof.input_nullifiers[1]);
        let (nullifier3, _) = pda::find_nullifier1(&proof.input_nullifiers[0]);
        let (tree_token_account, _) = pda::find_tree_token();
        let (global_config, _) = pda::find_global_config();

        let accounts = vec![
            AccountMeta::new(tree_account, false),
            AccountMeta::new(nullifier0, false),
            AccountMeta::new(nullifier1, false),
            AccountMeta::new_readonly(nullifier2, false),
            AccountMeta::new_readonly(nullifier3, false),
            AccountMeta::new(tree_token_account, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new(recipient, false),
            AccountMeta::new(fee_recipient, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(helper::system_program(), false),
        ];

        let enc1 = vec![0u8; 32];
        let enc2 = vec![0u8; 32];

        let mut args_data = Vec::new();
        proof.serialize(&mut args_data).unwrap();
        ext_data.serialize(&mut args_data).unwrap();
        BorshSerialize::serialize(&enc1, &mut args_data).unwrap();
        BorshSerialize::serialize(&enc2, &mut args_data).unwrap();

        let ix = helper::build_instruction("transact", accounts, args_data);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(ix.accounts.len(), 11, "transact needs 11 accounts");
        assert!(ix.accounts[9].is_signer, "signer must be signer");
        // Data: 8 (disc) + 480 (proof) + 16 (ext_data) + 4+32 (enc1) + 4+32 (enc2) = 576
        assert_eq!(
            ix.data.len(),
            8 + 480 + 16 + (4 + 32) + (4 + 32),
            "transact instruction data size"
        );
    }

    #[test]
    fn test_cross_nullifier_derivation() {
        // Verify that nullifier2 = find_nullifier0(input_nullifiers[1])
        // and nullifier3 = find_nullifier1(input_nullifiers[0])
        let hash0 = [4u8; 32];
        let hash1 = [5u8; 32];

        let (n0, _) = pda::find_nullifier0(&hash0);
        let (n1, _) = pda::find_nullifier1(&hash1);
        let (n2, _) = pda::find_nullifier0(&hash1); // cross: nullifier0 with hash1
        let (n3, _) = pda::find_nullifier1(&hash0); // cross: nullifier1 with hash0

        assert_ne!(n0, n2, "same prefix but different hash");
        assert_ne!(n1, n3, "same prefix but different hash");
        assert_ne!(n0, n1, "different prefix and hash");
        assert_ne!(n2, n3, "different prefix and hash");
    }

    #[tokio::test]
    #[ignore = "requires valid ZK proof and devnet funds"]
    async fn test_devnet_transact() {
        let ctx = CommandContext::default();
        let keypair = solana_keypair::Keypair::new();
        let wallet: Wallet = keypair.into();
        let recipient: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();

        let output = run(
            ctx,
            Input {
                fee_payer: wallet.clone(),
                signer: wallet,
                recipient,
                fee_recipient_account: recipient,
                proof: dummy_proof_json_flat(),
                ext_data_minified: serde_json::json!({
                    "ext_amount": 100000i64,
                    "fee": 500u64,
                }),
                encrypted_output1: vec![0u8; 32],
                encrypted_output2: vec![0u8; 32],
                submit: false,
            },
        )
        .await
        .unwrap();

        assert!(output.signature.is_none());
    }
}
