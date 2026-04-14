use super::derive_ata;
use crate::prelude::*;

const NAME: &str = "transfer_checked_t22";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/transfer_checked.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub destination_owner: Pubkey,
    pub authority: Wallet,
    /// Optional source-account owner for delegate / clawback transfers.
    /// When omitted, the source ATA is derived from `authority.pubkey()` (normal
    /// owner-initiated transfer). When set, the source ATA is derived from this
    /// pubkey and `authority` acts as a signing delegate (PermanentDelegate
    /// clawback, or an account previously approved via spl_token::approve).
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub source_owner: Option<Pubkey>,
    pub amount: u64,
    pub decimals: u8,
    /// Resolve the mint's TransferHook extension: fetch the ExtraAccountMetaList
    /// PDA and splice the hook program + extra accounts into the instruction.
    /// Required when transferring on a TransferHook-enabled mint — otherwise the
    /// token program will reject the transfer with missing accounts. Default
    /// false keeps behavior unchanged and skips the extra RPC round-trips.
    #[serde(default)]
    pub resolve_transfer_hook: bool,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub source: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let source_owner = input
        .source_owner
        .unwrap_or_else(|| input.authority.pubkey());
    let source = derive_ata(&source_owner, &input.mint);
    let destination = derive_ata(&input.destination_owner, &input.mint);

    let ix = if input.resolve_transfer_hook {
        build_transfer_with_hook(
            ctx.solana_client().clone(),
            &source,
            &input.mint,
            &destination,
            &input.authority.pubkey(),
            input.amount,
            input.decimals,
        )
        .await?
    } else {
        spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::ID,
            &source,
            &input.mint,
            &destination,
            &input.authority.pubkey(),
            &[],
            input.amount,
            input.decimals,
        )?
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output {
        signature,
        source,
        destination,
    })
}

/// Build a transfer_checked instruction with the TransferHook extension's
/// ExtraAccountMetaList resolved. Uses the `spl_token_2022` v7 offchain helper
/// (built on legacy `solana_program 2.x` types) and converts the resulting
/// instruction to the SDK-v3 `solana_instruction::Instruction` used by the
/// rest of cmds-solana. Byte-level pubkey conversion is safe because both
/// `Pubkey` types are transparent `[u8; 32]` wrappers.
async fn build_transfer_with_hook(
    rpc: Arc<RpcClient>,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<Instruction, CommandError> {
    use spl_token_2022::solana_program::pubkey::Pubkey as LegacyPubkey;

    let to_legacy = |pk: &Pubkey| LegacyPubkey::new_from_array(pk.to_bytes());
    let source_l = to_legacy(source);
    let mint_l = to_legacy(mint);
    let dest_l = to_legacy(destination);
    let auth_l = to_legacy(authority);
    let program_l = spl_token_2022::id();

    let legacy_ix = spl_token_2022::offchain::create_transfer_checked_instruction_with_extra_metas(
        &program_l,
        &source_l,
        &mint_l,
        &dest_l,
        &auth_l,
        &[],
        amount,
        decimals,
        |addr: LegacyPubkey| {
            let rpc = rpc.clone();
            async move {
                let v3 = Pubkey::new_from_array(addr.to_bytes());
                let resp = rpc
                    .get_account_with_commitment(&v3, rpc.commitment())
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
                Ok(resp.value.map(|acc| acc.data))
            }
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("transfer hook resolution failed: {e}"))?;

    Ok(Instruction {
        program_id: Pubkey::new_from_array(legacy_ix.program_id.to_bytes()),
        accounts: legacy_ix
            .accounts
            .into_iter()
            .map(|m| solana_program::instruction::AccountMeta {
                pubkey: Pubkey::new_from_array(m.pubkey.to_bytes()),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            })
            .collect(),
        data: legacy_ix.data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let destination_owner = Pubkey::new_unique();
        let source = derive_ata(&authority, &mint);
        let destination = derive_ata(&destination_owner, &mint);

        let ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::ID,
            &source,
            &mint,
            &destination,
            &authority,
            &[],
            1000,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }

    #[test]
    fn test_delegate_source_override() {
        // When source_owner is set, the source ATA must derive from it, not from authority.
        let mint = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let source_owner = Pubkey::new_unique();
        let destination_owner = Pubkey::new_unique();

        let source = derive_ata(&source_owner, &mint);
        let destination = derive_ata(&destination_owner, &mint);

        assert_ne!(source, derive_ata(&delegate, &mint));

        let ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::ID,
            &source,
            &mint,
            &destination,
            &delegate,
            &[],
            1000,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
    }
}
