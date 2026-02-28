use crate::prelude::*;
use super::{build_ix, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "close_task_queue_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/close_task_queue_v0.jsonc");

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
    pub rent_refund: Pubkey,
    pub payer: Wallet,
    pub update_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub tuktuk_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue_name_mapping: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // IDL discriminator for close_task_queue_v0
    let data = vec![196, 228, 35, 71, 131, 69, 175, 176];

    // Accounts per IDL order:
    // rent_refund: writable
    // payer: writable, signer
    // update_authority: signer
    // tuktuk_config: writable
    // task_queue: writable
    // task_queue_name_mapping: writable
    // system_program: readonly
    let accounts = vec![
        account_meta_mut(&input.rent_refund),
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_signer(&input.update_authority.pubkey()),
        account_meta_mut(&input.tuktuk_config),
        account_meta_mut(&input.task_queue),
        account_meta_mut(&input.task_queue_name_mapping),
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
