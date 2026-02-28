use crate::prelude::*;
use solana_system_interface::instruction as system_instruction;

const NAME: &str = "create_account_allow_prefund";
const DEFINITION: &str =
    flow_lib::node_definition!("system_program/create_account_allow_prefund.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub new_account: Wallet,
    pub payer: Option<Wallet>,
    pub lamports: Option<u64>,
    pub space: u64,
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let new_account_pubkey = input.new_account.pubkey();

    // Build instructions depending on whether a payer + lamports are provided.
    // If a payer is given, use create_account (which funds the account).
    // If no payer, use allocate + assign (account may already be prefunded).
    let mut signers: Vec<Wallet> = vec![input.fee_payer, input.new_account];
    let ixs: Vec<Instruction> = match (input.payer, input.lamports) {
        (Some(payer), Some(lamports)) => {
            let payer_pubkey = payer.pubkey();
            signers.push(payer);
            vec![system_instruction::create_account(
                &payer_pubkey,
                &new_account_pubkey,
                lamports,
                input.space,
                &input.owner,
            )]
        }
        _ => {
            // No payer: allocate space and assign owner without transferring lamports
            vec![
                system_instruction::allocate(&new_account_pubkey, input.space),
                system_instruction::assign(&new_account_pubkey, &input.owner),
            ]
        }
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: signers[0].pubkey(),
        signers,
        instructions: ixs,
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

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "new_account" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "space" => 100u64,
            "owner" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
