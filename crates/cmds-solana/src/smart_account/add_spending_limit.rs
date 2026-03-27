use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_add_spending_limit";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/add_spending_limit.jsonc");

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
    pub settings_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub seed: Pubkey,
    pub account_index: u8,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub amount: u64,
    /// 0=OneTime, 1=Day, 2=Week, 3=Month
    pub period: u8,
    #[serde_as(as = "Vec<AsPubkey>")]
    pub signers: Vec<Pubkey>,
    #[serde_as(as = "Vec<AsPubkey>")]
    #[serde(default)]
    pub destinations: Vec<Pubkey>,
    pub expiration: i64,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub spending_limit: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (spending_limit, _) = pda::find_spending_limit(&input.settings, &input.seed);

    let accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new_readonly(input.settings_authority.pubkey(), true),
        AccountMeta::new(spending_limit, false),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // AddSpendingLimitArgs
    let mut args_data = Vec::new();
    args_data.extend_from_slice(input.seed.as_ref());
    args_data.push(input.account_index);
    args_data.extend_from_slice(input.mint.as_ref());
    args_data.extend_from_slice(&input.amount.to_le_bytes());
    // Period enum (borsh: single byte for unit variants)
    args_data.push(input.period);
    // signers: Vec<Pubkey>
    args_data.extend_from_slice(&(input.signers.len() as u32).to_le_bytes());
    for s in &input.signers {
        args_data.extend_from_slice(s.as_ref());
    }
    // destinations: Vec<Pubkey>
    args_data.extend_from_slice(&(input.destinations.len() as u32).to_le_bytes());
    for d in &input.destinations {
        args_data.extend_from_slice(d.as_ref());
    }
    args_data.extend_from_slice(&input.expiration.to_le_bytes());
    match &input.memo {
        Some(s) => {
            args_data.push(1);
            args_data.extend_from_slice(&(s.len() as u32).to_le_bytes());
            args_data.extend_from_slice(s.as_bytes());
        }
        None => args_data.push(0),
    }

    let instruction = build_instruction("add_spending_limit_as_authority", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.settings_authority.clone()]
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
        spending_limit,
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
