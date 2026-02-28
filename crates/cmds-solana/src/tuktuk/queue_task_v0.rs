use crate::prelude::*;
use super::{build_ix, pda, types, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "queue_task_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/queue_task_v0.jsonc");

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
    pub payer: Wallet,
    pub queue_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task: Pubkey,
    pub id: u16,
    pub trigger: types::TriggerV0,
    pub transaction: types::TransactionSourceV0,
    pub crank_reward: Option<u64>,
    pub free_tasks: u8,
    pub description: String,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// Borsh-serialize TriggerV0
fn serialize_trigger(data: &mut Vec<u8>, trigger: &types::TriggerV0) {
    match trigger {
        types::TriggerV0::Now => {
            data.extend_from_slice(&0u32.to_le_bytes()); // variant index 0
        }
        types::TriggerV0::Timestamp { unix_timestamp } => {
            data.extend_from_slice(&1u32.to_le_bytes()); // variant index 1
            data.extend_from_slice(&unix_timestamp.to_le_bytes());
        }
    }
}

/// Borsh-serialize TransactionSourceV0
fn serialize_transaction(data: &mut Vec<u8>, tx: &types::TransactionSourceV0) {
    match tx {
        types::TransactionSourceV0::CompiledV0 {
            num_rw_signers,
            num_ro_signers,
            num_rw,
            data: tx_data,
        } => {
            data.extend_from_slice(&0u32.to_le_bytes()); // variant index 0
            // CompiledTransactionV0 struct:
            data.push(*num_rw_signers);
            data.push(*num_ro_signers);
            data.push(*num_rw);
            // accounts: Vec<Pubkey> - empty
            data.extend_from_slice(&0u32.to_le_bytes());
            // instructions: Vec<CompiledInstructionV0> - empty
            data.extend_from_slice(&0u32.to_le_bytes());
            // signer_seeds: Vec<Vec<Vec<u8>>> - empty
            data.extend_from_slice(&0u32.to_le_bytes());
            // Note: tx_data is currently unused in the original implementation
            let _ = tx_data;
        }
        types::TransactionSourceV0::RemoteV0 { url, signer } => {
            data.extend_from_slice(&1u32.to_le_bytes()); // variant index 1
            // url: String
            let url_bytes = url.as_bytes();
            data.extend_from_slice(&(url_bytes.len() as u32).to_le_bytes());
            data.extend_from_slice(url_bytes);
            // signer: Pubkey
            data.extend_from_slice(&signer.to_bytes());
        }
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive the task_queue_authority PDA
    let (task_queue_authority, _) = pda::find_task_queue_authority(
        &input.task_queue,
        &input.queue_authority.pubkey(),
    );

    // IDL discriminator for queue_task_v0
    let mut data = vec![177, 95, 195, 252, 241, 2, 178, 88];

    // Borsh-serialize QueueTaskArgsV0:
    //   id: u16
    //   trigger: TriggerV0 (enum)
    //   transaction: TransactionSourceV0 (enum)
    //   crank_reward: Option<u64>
    //   free_tasks: u8
    //   description: String
    data.extend_from_slice(&input.id.to_le_bytes());
    serialize_trigger(&mut data, &input.trigger);
    serialize_transaction(&mut data, &input.transaction);
    match input.crank_reward {
        Some(reward) => {
            data.push(1); // Option::Some
            data.extend_from_slice(&reward.to_le_bytes());
        }
        None => {
            data.push(0); // Option::None
        }
    }
    data.push(input.free_tasks);
    let desc_bytes = input.description.as_bytes();
    data.extend_from_slice(&(desc_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(desc_bytes);

    // Accounts per IDL order:
    // payer: writable, signer
    // queue_authority: signer
    // task_queue_authority: readonly (PDA)
    // task_queue: writable
    // task: writable
    // system_program: readonly
    let accounts = vec![
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_signer(&input.queue_authority.pubkey()),
        account_meta_readonly(&task_queue_authority),
        account_meta_mut(&input.task_queue),
        account_meta_mut(&input.task),
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.payer.clone(),
            input.queue_authority.clone(),
        ]
        .into_iter()
        .collect(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
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
