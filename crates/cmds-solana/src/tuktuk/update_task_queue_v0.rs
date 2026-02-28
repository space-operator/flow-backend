use crate::prelude::*;
use super::{build_ix, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "update_task_queue_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/update_task_queue_v0.jsonc");

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
    pub update_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    pub min_crank_reward: Option<u64>,
    pub capacity: Option<u16>,
    #[serde_as(as = "Option<Vec<AsPubkey>>")]
    pub lookup_tables: Option<Vec<Pubkey>>,
    /// New update authority to transfer ownership to (renamed from `update_authority` to avoid
    /// collision with the signer account field).
    #[serde_as(as = "Option<AsPubkey>")]
    pub new_update_authority: Option<Pubkey>,
    pub stale_task_age: Option<u32>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // IDL discriminator for update_task_queue_v0
    let mut data = vec![107, 147, 81, 119, 75, 1, 18, 41];

    // Borsh-serialize UpdateTaskQueueArgsV0:
    //   min_crank_reward: Option<u64>
    //   capacity: Option<u16>
    //   lookup_tables: Option<Vec<Pubkey>>
    //   update_authority: Option<Pubkey>
    //   stale_task_age: Option<u32>

    // min_crank_reward
    match input.min_crank_reward {
        Some(v) => { data.push(1); data.extend_from_slice(&v.to_le_bytes()); }
        None => { data.push(0); }
    }
    // capacity
    match input.capacity {
        Some(v) => { data.push(1); data.extend_from_slice(&v.to_le_bytes()); }
        None => { data.push(0); }
    }
    // lookup_tables
    match &input.lookup_tables {
        Some(tables) => {
            data.push(1);
            data.extend_from_slice(&(tables.len() as u32).to_le_bytes());
            for pk in tables {
                data.extend_from_slice(&pk.to_bytes());
            }
        }
        None => { data.push(0); }
    }
    // update_authority (new)
    match input.new_update_authority {
        Some(pk) => { data.push(1); data.extend_from_slice(&pk.to_bytes()); }
        None => { data.push(0); }
    }
    // stale_task_age
    match input.stale_task_age {
        Some(v) => { data.push(1); data.extend_from_slice(&v.to_le_bytes()); }
        None => { data.push(0); }
    }

    // Accounts per IDL order:
    // payer: writable, signer
    // update_authority: signer
    // task_queue: writable
    // system_program: readonly
    let accounts = vec![
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_signer(&input.update_authority.pubkey()),
        account_meta_mut(&input.task_queue),
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.payer.clone(),
            input.update_authority.clone(),
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
