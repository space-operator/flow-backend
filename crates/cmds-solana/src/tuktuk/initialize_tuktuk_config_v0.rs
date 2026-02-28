use crate::prelude::*;
use super::{build_ix, pda, SYSTEM_PROGRAM_ID, account_meta_signer_mut, account_meta_signer, account_meta_readonly, account_meta_mut};

const NAME: &str = "initialize_tuktuk_config_v0";
const DEFINITION: &str = flow_lib::node_definition!("tuktuk/initialize_tuktuk_config_v0.jsonc");

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
    pub approver: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub authority: Pubkey,
    pub min_deposit: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Derive the tuktuk_config PDA
    let (tuktuk_config, _) = pda::find_tuktuk_config();

    // IDL discriminator for initialize_tuktuk_config_v0
    let mut data = vec![67, 128, 98, 227, 103, 60, 179, 214];

    // Borsh-serialize InitializeTuktukConfigArgsV0:
    //   min_deposit: u64
    data.extend_from_slice(&input.min_deposit.to_le_bytes());

    // Accounts per IDL order:
    // payer: writable, signer
    // approver: signer
    // authority: readonly
    // tuktuk_config: writable (PDA)
    // system_program: readonly
    let accounts = vec![
        account_meta_signer_mut(&input.payer.pubkey()),
        account_meta_signer(&input.approver.pubkey()),
        account_meta_readonly(&input.authority),
        account_meta_mut(&tuktuk_config),
        account_meta_readonly(&SYSTEM_PROGRAM_ID),
    ];

    let instruction = build_ix(accounts, data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone(),
            input.payer.clone(),
            input.approver.clone(),
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
