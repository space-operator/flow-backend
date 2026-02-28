use crate::prelude::*;
use super::{build_ix, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_readonly, account_meta_mut};

const NAME: &str = "initialize_task_queue_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/initialize_task_queue_v0.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub tuktuk_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub update_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue_name_mapping: Pubkey,
    pub min_crank_reward: u64,
    pub name: String,
    pub capacity: u16,
    #[serde_as(as = "Vec<AsPubkey>")]
    pub lookup_tables: Vec<Pubkey>,
    pub stale_task_age: u32,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // IDL discriminator for initialize_task_queue_v0
    let mut data = vec![150, 100, 6, 8, 32, 179, 250, 186];

    // Borsh-serialize InitializeTaskQueueArgsV0:
    //   min_crank_reward: u64
    //   name: String (4-byte len + bytes)
    //   capacity: u16
    //   lookup_tables: Vec<Pubkey> (4-byte len + 32-byte pubkeys)
    //   stale_task_age: u32
    data.extend_from_slice(&input.min_crank_reward.to_le_bytes());
    let name_bytes = input.name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);
    data.extend_from_slice(&input.capacity.to_le_bytes());
    data.extend_from_slice(&(input.lookup_tables.len() as u32).to_le_bytes());
    for pk in &input.lookup_tables {
        data.extend_from_slice(&pk.to_bytes());
    }
    data.extend_from_slice(&input.stale_task_age.to_le_bytes());

    // Accounts per IDL order:
    // payer: writable, signer
    // tuktuk_config: writable
    // update_authority: readonly
    // task_queue: writable
    // task_queue_name_mapping: writable
    // system_program: readonly
    let accounts = vec![
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_mut(&input.tuktuk_config),
        account_meta_readonly(&input.update_authority),
        account_meta_mut(&input.task_queue),
        account_meta_mut(&input.task_queue_name_mapping),
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.payer.clone()]
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
