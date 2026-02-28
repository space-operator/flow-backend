use super::{helper, pda};
use crate::prelude::*;
use borsh::BorshSerialize;
use solana_program::instruction::AccountMeta;

const NAME: &str = "update_spl_deposit_limit";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/update_spl_deposit_limit.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    pub new_limit: u64,
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
    let (tree_account, _) = pda::find_merkle_tree_spl(&input.mint);

    tracing::info!(
        "update_spl_deposit_limit: authority={}, mint={}, new_limit={}",
        input.authority.pubkey(),
        input.mint,
        input.new_limit
    );

    // Accounts: UpdateDepositLimitForSplToken context
    let accounts = vec![
        AccountMeta::new(tree_account, false), // tree_account (mut, PDA)
        AccountMeta::new_readonly(input.mint, false), // mint
        AccountMeta::new_readonly(input.authority.pubkey(), true), // authority (signer)
    ];

    let mut args_data = Vec::new();
    BorshSerialize::serialize(&input.new_limit, &mut args_data)?;

    let instruction =
        helper::build_instruction("update_deposit_limit_for_spl_token", accounts, args_data);

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
    let signature = ctx.execute(ins, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;
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
        let mint: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        let (tree_account, _) = pda::find_merkle_tree_spl(&mint);

        let accounts = vec![
            AccountMeta::new(tree_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(authority, true),
        ];

        let new_limit: u64 = 5_000_000; // 5 USDC
        let mut args_data = Vec::new();
        BorshSerialize::serialize(&new_limit, &mut args_data).unwrap();

        let ix =
            helper::build_instruction("update_deposit_limit_for_spl_token", accounts, args_data);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(
            ix.accounts.len(),
            3,
            "update_spl_deposit_limit needs 3 accounts"
        );
        // 8 (disc) + 8 (u64) = 16 bytes
        assert_eq!(ix.data.len(), 16);
        assert!(ix.accounts[2].is_signer, "authority must be signer");
        assert!(ix.accounts[0].is_writable, "tree_account must be writable");
        assert!(!ix.accounts[1].is_writable, "mint should be read-only");
    }

    #[test]
    fn test_discriminator_uses_anchor_name() {
        // The on-chain instruction is "update_deposit_limit_for_spl_token",
        // not "update_spl_deposit_limit" (the node name)
        let disc = helper::anchor_discriminator("update_deposit_limit_for_spl_token");
        let wrong_disc = helper::anchor_discriminator("update_spl_deposit_limit");
        assert_ne!(
            disc, wrong_disc,
            "must use on-chain instruction name, not node name"
        );
    }

    #[tokio::test]
    #[ignore = "requires devnet admin key and funded wallet"]
    async fn test_devnet_update_spl_deposit_limit() {
        let ctx = CommandContext::default();
        let keypair = solana_keypair::Keypair::new();
        let wallet: Wallet = keypair.into();
        let mint: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();

        let output = run(
            ctx,
            Input {
                fee_payer: wallet.clone(),
                authority: wallet,
                mint,
                new_limit: 10_000_000,
                submit: false,
            },
        )
        .await
        .unwrap();

        assert!(output.signature.is_none());
    }
}
