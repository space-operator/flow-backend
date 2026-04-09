use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_create_proposal";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/create_proposal.jsonc");

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
    pub settings: Pubkey,
    pub creator: Wallet,
    pub transaction_index: u64,
    #[serde(default)]
    pub draft: bool,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub proposal: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (proposal, _) = pda::find_proposal(&input.settings, input.transaction_index);

    let accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new(proposal, false),
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // CreateProposalArgs { transaction_index: u64, draft: bool }
    let mut args_data = Vec::new();
    args_data.extend_from_slice(&input.transaction_index.to_le_bytes());
    args_data.push(input.draft as u8);

    let instruction = build_instruction("create_proposal", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.creator.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        proposal,
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
