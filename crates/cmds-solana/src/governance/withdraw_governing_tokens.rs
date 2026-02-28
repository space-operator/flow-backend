use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, spl_token_program_id};

const NAME: &str = "withdraw_governing_tokens";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/withdraw_governing_tokens.jsonc");
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
    #[serde(with = "value::pubkey")]
    pub realm: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governing_token_destination: Pubkey,
    pub governing_token_owner: Wallet,
    #[serde(with = "value::pubkey")]
    pub governing_token_mint: Pubkey,
    /// Set to true if the governing token mint is a Token-2022 mint
    #[serde(default)]
    pub is_token_2022: bool,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// v3.1.2 account layout:
/// 0. `[]` Realm account
/// 1. `[writable]` Governing token holding (PDA)
/// 2. `[writable]` Governing token destination
/// 3. `[signer]` Governing token owner
/// 4. `[writable]` TokenOwnerRecord (PDA)
/// 5. `[]` SPL Token program (Token or Token-2022)
/// 6. `[]` Realm config (PDA)
///    (if is_token_2022):
/// 7. `[writable]` Governing token mint
pub fn withdraw_governing_tokens(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_destination: &Pubkey,
    governing_token_owner: &Pubkey,
    governing_token_mint: &Pubkey,
    is_token_2022: bool,
) -> (Instruction, Pubkey, Pubkey, Pubkey) {
    let seeds = [
        b"governance",
        realm.as_ref(),
        governing_token_mint.as_ref(),
        governing_token_owner.as_ref(),
    ];
    let token_owner_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    let seeds = [b"governance", realm.as_ref(), governing_token_mint.as_ref()];
    let governing_token_holding_address = Pubkey::find_program_address(&seeds, program_id).0;

    let seeds = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_destination, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new_readonly(spl_token_program_id(is_token_2022), false),
        AccountMeta::new_readonly(realm_config_address, false),
    ];

    if is_token_2022 {
        accounts.push(AccountMeta::new(*governing_token_mint, false));
    }

    let data = GovernanceInstruction::WithdrawGoverningTokens {};

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (
        instruction,
        realm_config_address,
        governing_token_holding_address,
        token_owner_record_address,
    )
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, realm_config_address, governing_token_holding_address, token_owner_record_address) =
        withdraw_governing_tokens(
            &program_id,
            &input.realm,
            &input.governing_token_destination,
            &input.governing_token_owner.pubkey(),
            &input.governing_token_mint,
            input.is_token_2022,
        );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governing_token_owner].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "realm_config_address" => realm_config_address,
                "governing_token_holding_address" => governing_token_holding_address,
                "token_owner_record_address" => token_owner_record_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::super::SPL_GOVERNANCE_ID;
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let realm = Pubkey::new_unique();
        let governing_token_destination = Pubkey::new_unique();
        let governing_token_owner = Pubkey::new_unique();
        let governing_token_mint = Pubkey::new_unique();

        let (ix, _, _, _) = withdraw_governing_tokens(
            &SPL_GOVERNANCE_ID,
            &realm,
            &governing_token_destination,
            &governing_token_owner,
            &governing_token_mint,
            false,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 7);
    }
}
