use crate::prelude::*;
use crate::privacy_cash::helper;
use crate::privacy_cash::pda;
use borsh::BorshSerialize;
use solana_program::instruction::AccountMeta;

const NAME: &str = "init_spl_tree";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/init_spl_tree.jsonc");

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
    pub max_deposit_amount: u64,
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
    let (global_config, _) = pda::find_global_config();

    tracing::info!(
        "init_spl_tree: authority={}, mint={}, tree={}, max_deposit={}",
        input.authority.pubkey(),
        input.mint,
        tree_account,
        input.max_deposit_amount
    );

    // Accounts: InitializeTreeAccountForSplToken context
    let accounts = vec![
        AccountMeta::new(tree_account, false), // tree_account (init, PDA with mint seed)
        AccountMeta::new_readonly(input.mint, false), // mint
        AccountMeta::new_readonly(global_config, false), // global_config (PDA)
        AccountMeta::new(input.authority.pubkey(), true), // authority (mut, signer)
        AccountMeta::new_readonly(helper::system_program(), false), // system_program
    ];

    let mut args_data = Vec::new();
    BorshSerialize::serialize(&input.max_deposit_amount, &mut args_data)?;

    let instruction =
        helper::build_instruction("initialize_tree_account_for_spl_token", accounts, args_data);

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
            },
        )
        .await?
        .signature;

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
        let (global_config, _) = pda::find_global_config();

        let accounts = vec![
            AccountMeta::new(tree_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(helper::system_program(), false),
        ];

        let mut args_data = Vec::new();
        BorshSerialize::serialize(&1_000_000u64, &mut args_data).unwrap(); // 1 USDC (6 decimals)

        let ix =
            helper::build_instruction("initialize_tree_account_for_spl_token", accounts, args_data);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(ix.accounts.len(), 5, "init_spl_tree needs 5 accounts");
        // 8 (disc) + 8 (u64) = 16 bytes
        assert_eq!(ix.data.len(), 16);
        assert!(ix.accounts[3].is_signer, "authority must be signer");
        assert!(ix.accounts[0].is_writable, "tree_account must be writable");
        assert!(!ix.accounts[1].is_writable, "mint should be read-only");
    }

    #[test]
    fn test_spl_tree_pda_uses_mint_seed() {
        let usdc: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        let wsol: Pubkey = "So11111111111111111111111111111111111111112"
            .parse()
            .unwrap();
        let (tree_usdc, _) = pda::find_merkle_tree_spl(&usdc);
        let (tree_wsol, _) = pda::find_merkle_tree_spl(&wsol);
        let (tree_sol, _) = pda::find_merkle_tree();

        // Different mints produce different tree PDAs
        assert_ne!(tree_usdc, tree_wsol);
        // SPL tree PDAs differ from SOL tree PDA
        assert_ne!(tree_usdc, tree_sol);
    }

    #[tokio::test]
    #[ignore = "requires devnet admin key and funded wallet"]
    async fn test_devnet_init_spl_tree() {
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
                max_deposit_amount: 1_000_000,
                submit: false,
            },
        )
        .await
        .unwrap();

        assert!(output.signature.is_none());
    }
}
