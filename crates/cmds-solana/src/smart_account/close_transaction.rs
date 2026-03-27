use super::{build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_close_transaction";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/close_transaction.jsonc");

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
    pub transaction_index: u64,
    #[serde_as(as = "AsPubkey")]
    pub proposal_rent_collector: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub transaction_rent_collector: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (proposal, _) = pda::find_proposal(&input.settings, input.transaction_index);
    let (transaction, _) = pda::find_transaction(&input.settings, input.transaction_index);

    let accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new(proposal, false),
        AccountMeta::new(transaction, false),
        AccountMeta::new(input.proposal_rent_collector, false),
        AccountMeta::new(input.transaction_rent_collector, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    let instruction = build_instruction("close_transaction", accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone()].into_iter().collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
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
