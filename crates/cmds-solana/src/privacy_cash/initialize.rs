use super::{helper, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "initialize";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/initialize.jsonc");

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
    pub authority: Wallet,
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
    let (tree_account, _) = pda::find_merkle_tree();
    let (tree_token_account, _) = pda::find_tree_token();
    let (global_config, _) = pda::find_global_config();

    tracing::info!(
        "initialize: authority={}, tree={}, tree_token={}, global_config={}",
        input.authority.pubkey(),
        tree_account,
        tree_token_account,
        global_config
    );

    // Accounts: Initialize context
    let accounts = vec![
        AccountMeta::new(tree_account, false), // tree_account (init, PDA)
        AccountMeta::new(tree_token_account, false), // tree_token_account (init, PDA)
        AccountMeta::new(global_config, false), // global_config (init, PDA)
        AccountMeta::new(input.authority.pubkey(), true), // authority (mut, signer)
        AccountMeta::new_readonly(helper::system_program(), false), // system_program
    ];

    let instruction = helper::build_instruction_no_args("initialize", accounts);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.authority.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx
        .execute(
            ins,
            value::map! {
                "tree_account" => tree_account,
                "tree_token_account" => tree_token_account,
                "global_config" => global_config,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::instruction::AccountMeta;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_construction() {
        let authority: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();
        let (tree_account, _) = pda::find_merkle_tree();
        let (tree_token_account, _) = pda::find_tree_token();
        let (global_config, _) = pda::find_global_config();

        let accounts = vec![
            AccountMeta::new(tree_account, false),
            AccountMeta::new(tree_token_account, false),
            AccountMeta::new(global_config, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(helper::system_program(), false),
        ];

        let ix = helper::build_instruction_no_args("initialize", accounts);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(ix.accounts.len(), 5, "initialize needs 5 accounts");
        assert_eq!(
            ix.data.len(),
            8,
            "initialize has no args, just 8-byte discriminator"
        );
        assert!(ix.accounts[3].is_signer, "authority must be signer");
        assert!(ix.accounts[0].is_writable, "tree_account must be writable");
    }

    #[tokio::test]
    #[ignore = "requires devnet admin key and funded wallet"]
    async fn test_devnet_initialize() {
        let ctx = CommandContext::default();
        let keypair = solana_keypair::Keypair::new();
        let wallet: Wallet = keypair.into();

        let output = run(
            ctx,
            Input {
                fee_payer: wallet.clone(),
                authority: wallet,
                submit: false,
            },
        )
        .await
        .unwrap();

        // With submit=false, no signature is returned
        assert!(output.signature.is_none());
    }
}
