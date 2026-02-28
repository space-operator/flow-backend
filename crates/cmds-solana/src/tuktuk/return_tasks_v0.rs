use crate::prelude::*;
use super::{build_ix, types, SYSTEM_PROGRAM_ID, account_meta_readonly};

const NAME: &str = "return_tasks_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/return_tasks_v0.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub tasks: Vec<types::TaskReturnV0>,
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
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        types::TriggerV0::Timestamp { unix_timestamp } => {
            data.extend_from_slice(&1u32.to_le_bytes());
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
            data.extend_from_slice(&0u32.to_le_bytes());
            data.push(*num_rw_signers);
            data.push(*num_ro_signers);
            data.push(*num_rw);
            data.extend_from_slice(&0u32.to_le_bytes()); // accounts
            data.extend_from_slice(&0u32.to_le_bytes()); // instructions
            data.extend_from_slice(&0u32.to_le_bytes()); // signer_seeds
            let _ = tx_data;
        }
        types::TransactionSourceV0::RemoteV0 { url, signer } => {
            data.extend_from_slice(&1u32.to_le_bytes());
            let url_bytes = url.as_bytes();
            data.extend_from_slice(&(url_bytes.len() as u32).to_le_bytes());
            data.extend_from_slice(url_bytes);
            data.extend_from_slice(&signer.to_bytes());
        }
    }
}

/// Borsh-serialize a single TaskReturnV0
fn serialize_task_return(data: &mut Vec<u8>, task: &types::TaskReturnV0) {
    serialize_trigger(data, &task.trigger);
    serialize_transaction(data, &task.transaction);
    match task.crank_reward {
        Some(reward) => {
            data.push(1);
            data.extend_from_slice(&reward.to_le_bytes());
        }
        None => {
            data.push(0);
        }
    }
    data.push(task.free_tasks);
    let desc_bytes = task.description.as_bytes();
    data.extend_from_slice(&(desc_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(desc_bytes);
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // IDL discriminator for return_tasks_v0
    let mut data = vec![38, 235, 111, 148, 44, 99, 189, 164];

    // Borsh-serialize ReturnTasksArgsV0:
    //   tasks: Vec<TaskReturnV0>
    data.extend_from_slice(&(input.tasks.len() as u32).to_le_bytes());
    for task in &input.tasks {
        serialize_task_return(&mut data, task);
    }

    // Accounts per IDL order:
    // system_program: readonly
    let accounts = vec![
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
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
