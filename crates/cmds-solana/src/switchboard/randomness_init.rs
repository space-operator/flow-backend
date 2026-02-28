use crate::prelude::*;

use super::helper::*;

pub const NAME: &str = "switchboard_randomness_init";
const DEFINITION: &str =
    flow_lib::node_definition!("switchboard/switchboard_randomness_init.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    queue: Pubkey,
    randomness_account: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    #[serde(with = "value::pubkey")]
    randomness_pubkey: Pubkey,
    #[serde(with = "value::pubkey")]
    reward_escrow: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rng_pubkey = input.randomness_account.pubkey();
    let payer_pubkey = input.fee_payer.pubkey();

    // Derive accounts
    let program_state = state_pda(&SB_ON_DEMAND_PID);
    let reward_escrow = get_ata(&WSOL_MINT, &rng_pubkey);
    let lut_signer = lut_signer_pda(&SB_ON_DEMAND_PID, &rng_pubkey);

    // The LUT key requires the current finalized slot. We fetch it from RPC.
    let recent_slot = ctx.solana_client().get_slot().await.map_err(|e| {
        CommandError::msg(format!("Failed to get recent slot: {e}"))
    })?;
    // derive_lookup_table_address: PDA of [lut_signer, recent_slot as le bytes] under the ALT program
    let lut_key = Pubkey::find_program_address(
        &[lut_signer.as_ref(), &recent_slot.to_le_bytes()],
        &ALT_PROGRAM,
    )
    .0;

    // Instruction data: { recent_slot: u64 }
    let args_data = recent_slot.to_le_bytes();

    let accounts = vec![
        AccountMeta::new(rng_pubkey, true),                             // randomness (signer, writable)
        AccountMeta::new_readonly(input.queue, false),                  // queue
        AccountMeta::new_readonly(payer_pubkey, true),                  // authority (signer)
        AccountMeta::new(payer_pubkey, true),                           // payer (signer, writable)
        AccountMeta::new(reward_escrow, false),                         // rewardEscrow (writable)
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // systemProgram
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM, false),            // tokenProgram
        AccountMeta::new_readonly(SPL_ATA_PROGRAM, false),              // associatedTokenProgram
        AccountMeta::new_readonly(WSOL_MINT, false),                    // wrappedSolMint
        AccountMeta::new_readonly(program_state, false),                // programState
        AccountMeta::new_readonly(lut_signer, false),                   // lutSigner
        AccountMeta::new(lut_key, false),                               // lut (writable)
        AccountMeta::new_readonly(ALT_PROGRAM, false),                  // addressLookupTableProgram
    ];

    let instruction = build_sb_instruction("randomness_init", accounts, &args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: payer_pubkey,
        signers: [input.fee_payer, input.randomness_account].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        randomness_pubkey: rng_pubkey,
        reward_escrow,
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
