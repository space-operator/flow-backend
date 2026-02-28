use super::{helper, pda};
use crate::prelude::*;
use borsh::BorshSerialize;
use solana_program::instruction::AccountMeta;

const NAME: &str = "update_deposit_limit";
const DEFINITION: &str = flow_lib::node_definition!("privacy_cash/update_deposit_limit.jsonc");

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
    let (tree_account, _) = pda::find_merkle_tree();

    tracing::info!(
        "update_deposit_limit: authority={}, new_limit={}",
        input.authority.pubkey(),
        input.new_limit
    );

    // Accounts: UpdateDepositLimit context
    let accounts = vec![
        AccountMeta::new(tree_account, false), // tree_account (mut, PDA)
        AccountMeta::new_readonly(input.authority.pubkey(), true), // authority (signer)
    ];

    let mut args_data = Vec::new();
    BorshSerialize::serialize(&input.new_limit, &mut args_data)?;

    let instruction = helper::build_instruction("update_deposit_limit", accounts, args_data);

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
        let (tree_account, _) = pda::find_merkle_tree();

        let accounts = vec![
            AccountMeta::new(tree_account, false),
            AccountMeta::new_readonly(authority, true),
        ];

        let new_limit: u64 = 1_000_000_000; // 1 SOL
        let mut args_data = Vec::new();
        BorshSerialize::serialize(&new_limit, &mut args_data).unwrap();

        let ix = helper::build_instruction("update_deposit_limit", accounts, args_data);

        assert_eq!(ix.program_id, pda::program_id());
        assert_eq!(
            ix.accounts.len(),
            2,
            "update_deposit_limit needs 2 accounts"
        );
        // 8 (discriminator) + 8 (u64) = 16 bytes
        assert_eq!(ix.data.len(), 16, "instruction data = discriminator + u64");
        assert!(ix.accounts[1].is_signer, "authority must be signer");
        assert!(ix.accounts[0].is_writable, "tree_account must be writable");
    }

    #[test]
    fn test_args_serialization() {
        let limit: u64 = 5_000_000_000;
        let mut data = Vec::new();
        BorshSerialize::serialize(&limit, &mut data).unwrap();
        assert_eq!(data.len(), 8);
        // Little-endian check
        assert_eq!(u64::from_le_bytes(data.try_into().unwrap()), 5_000_000_000);
    }

    #[tokio::test]
    #[ignore = "requires devnet admin key and funded wallet"]
    async fn test_devnet_update_deposit_limit() {
        let ctx = CommandContext::default();
        let keypair = solana_keypair::Keypair::new();
        let wallet: Wallet = keypair.into();

        let output = run(
            ctx,
            Input {
                fee_payer: wallet.clone(),
                authority: wallet,
                new_limit: 2_000_000_000,
                submit: false,
            },
        )
        .await
        .unwrap();

        assert!(output.signature.is_none());
    }
}
