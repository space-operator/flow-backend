use crate::prelude::*;
use super::{build_ix, pda, account_meta_signer_mut, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "remove_queue_authority_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/remove_queue_authority_v0.jsonc");

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
    pub rent_refund: Pubkey,
    pub update_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub queue_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
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
    let (task_queue_authority, _) =
        pda::find_task_queue_authority(&input.task_queue, &input.queue_authority);

    // IDL discriminator for remove_queue_authority_v0
    let data = vec![180, 26, 196, 229, 80, 40, 245, 187];

    // Accounts per IDL order:
    // payer: writable, signer
    // rent_refund: writable
    // update_authority: signer
    // queue_authority: readonly
    // task_queue_authority: writable (PDA)
    // task_queue: writable
    let accounts = vec![
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_mut(&input.rent_refund),
        account_meta_signer(&input.update_authority.pubkey()),
        account_meta_readonly(&input.queue_authority),
        account_meta_mut(&task_queue_authority),
        account_meta_mut(&input.task_queue),
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
