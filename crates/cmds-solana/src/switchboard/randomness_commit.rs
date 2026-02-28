use crate::prelude::*;

use super::helper::*;

pub const NAME: &str = "switchboard_randomness_commit";
const DEFINITION: &str =
    flow_lib::node_definition!("switchboard/switchboard_randomness_commit.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    randomness_account: Pubkey,
    #[serde(with = "value::pubkey")]
    queue: Pubkey,
    #[serde(with = "value::pubkey")]
    oracle: Pubkey,
    authority: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // randomness_commit takes no args (empty struct `{}`)
    let accounts = vec![
        AccountMeta::new(input.randomness_account, false),     // randomness (writable)
        AccountMeta::new_readonly(input.queue, false),         // queue
        AccountMeta::new_readonly(input.oracle, false),        // oracle
        AccountMeta::new_readonly(SLOT_HASHES_SYSVAR, false),  // recentSlothashes
        AccountMeta::new_readonly(input.authority.pubkey(), true), // authority (signer)
    ];

    let instruction = build_sb_instruction("randomness_commit", accounts, &[]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
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
