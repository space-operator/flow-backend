use crate::prelude::*;
use super::{build_ix, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_readonly, account_meta_mut};

const NAME: &str = "run_task_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/run_task_v0.jsonc");

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
    pub crank_turner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub rent_refund: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task_queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub task: Pubkey,
    pub free_task_ids: Vec<u16>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Sysvar Instructions address
    let sysvar_instructions =
        solana_pubkey::pubkey!("Sysvar1nstructions1111111111111111111111111");

    // IDL discriminator for run_task_v0
    let mut data = vec![52, 184, 39, 129, 126, 245, 176, 237];

    // Borsh-serialize RunTaskArgsV0:
    //   free_task_ids: Vec<u16>
    data.extend_from_slice(&(input.free_task_ids.len() as u32).to_le_bytes());
    for id in &input.free_task_ids {
        data.extend_from_slice(&id.to_le_bytes());
    }

    // Accounts per IDL order:
    // crank_turner: writable, signer
    // rent_refund: writable
    // task_queue: writable
    // task: writable
    // system_program: readonly
    // sysvar_instructions: readonly
    let accounts = vec![
        account_meta_signer_mut(&input.crank_turner.pubkey()),
        account_meta_mut(&input.rent_refund),
        account_meta_mut(&input.task_queue),
        account_meta_mut(&input.task),
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
        account_meta_readonly(&sysvar_instructions),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.crank_turner.clone()]
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
