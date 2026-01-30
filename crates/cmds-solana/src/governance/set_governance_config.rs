use solana_program::instruction::AccountMeta;

use crate::prelude::*;

use super::{GovernanceConfig, GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "set_governance_config";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/set_governance_config.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,

    pub governance: Wallet,
    pub config: GovernanceConfig,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn set_governance_config(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    // Args
    config: GovernanceConfig,
) -> Instruction {
    let accounts = vec![AccountMeta::new(*governance, true)];

    let instruction = GovernanceInstruction::SetGovernanceConfig { config };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = set_governance_config(&program_id, &input.governance.pubkey(), input.config);

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
