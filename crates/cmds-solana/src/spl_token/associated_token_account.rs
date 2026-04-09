use crate::prelude::*;
use solana_commitment_config::CommitmentConfig;
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;

const SOLANA_ASSOCIATED_TOKEN_ACCOUNT: &str = "associated_token_account";

const DEFINITION: &str = flow_lib::node_definition!("spl_token/associated_token_account.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_ASSOCIATED_TOKEN_ACCOUNT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(
    SOLANA_ASSOCIATED_TOKEN_ACCOUNT,
    |_| { build() }
));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    owner: Pubkey,
    fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    mint_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Query the mint account on-chain to determine which token program owns it
    let mint_account_data = ctx
        .solana_client()
        .get_account_with_commitment(&input.mint_account, CommitmentConfig::confirmed())
        .await
        .map_err(|e| CommandError::msg(format!("Failed to read mint account: {e}")))?
        .value
        .ok_or_else(|| {
            CommandError::msg(format!("Mint account {} not found", input.mint_account))
        })?;

    let token_program_id = mint_account_data.owner;

    // Derive ATA using the mint's actual token program
    let (associated_token_account, _) = Pubkey::find_program_address(
        &[
            input.owner.as_ref(),
            token_program_id.as_ref(),
            input.mint_account.as_ref(),
        ],
        &spl_associated_token_account_interface::program::ID,
    );

    let instruction = create_associated_token_account_idempotent(
        &input.fee_payer.pubkey(),
        &input.owner,
        &input.mint_account,
        &token_program_id,
    );

    let instructions = if input.submit {
        Instructions {
            lookup_tables: None,
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer].into(),
            instructions: [instruction].into(),
        }
    } else {
        <_>::default()
    };

    let signature = ctx
        .execute(
            instructions,
            value::map! {
                "associated_token_account" => associated_token_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_accounts() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program = spl_token_interface::ID;

        let (ata, _) = Pubkey::find_program_address(
            &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
            &spl_associated_token_account_interface::program::ID,
        );

        let ix =
            create_associated_token_account_idempotent(&fee_payer, &owner, &mint, &token_program);

        assert_eq!(ix.accounts.len(), 6);
        // [funding, ata, wallet, mint, system_program, token_program]
        assert_eq!(ix.accounts[0].pubkey, fee_payer);
        assert!(ix.accounts[0].is_signer);
        assert!(ix.accounts[0].is_writable);
        assert_eq!(ix.accounts[1].pubkey, ata);
        assert!(ix.accounts[1].is_writable);
        assert_eq!(ix.accounts[2].pubkey, owner);
        assert_eq!(ix.accounts[3].pubkey, mint);
        assert_eq!(ix.accounts[5].pubkey, token_program);
    }
}
