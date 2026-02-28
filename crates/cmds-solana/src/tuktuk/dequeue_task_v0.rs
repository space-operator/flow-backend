use crate::prelude::*;
use super::{build_ix, pda, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "dequeue_task_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/dequeue_task_v0.jsonc");

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
    pub queue_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub rent_refund: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive the task_queue_authority PDA
    let (task_queue_authority, _) = pda::find_task_queue_authority(
        &input.task_queue,
        &input.queue_authority.pubkey(),
    );

    // IDL discriminator for dequeue_task_v0
    let data = vec![92, 141, 249, 132, 219, 109, 215, 126];

    // Accounts per IDL order:
    // queue_authority: signer
    // rent_refund: writable
    // task_queue_authority: readonly (PDA)
    // task_queue: writable
    // task: writable
    let accounts = vec![
        account_meta_signer(&input.queue_authority.pubkey()),
        account_meta_mut(&input.rent_refund),
        account_meta_readonly(&task_queue_authority),
        account_meta_mut(&input.task_queue),
        account_meta_mut(&input.task),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.queue_authority.clone()]
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
