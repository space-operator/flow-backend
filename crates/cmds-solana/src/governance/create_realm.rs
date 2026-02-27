use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;
use tracing::info;

use super::prelude::*;

use super::{
    GovernanceInstruction, GoverningTokenConfigAccountArgs, GoverningTokenConfigArgs,
    MintMaxVoterWeightSource, RealmConfigArgs, SPL_GOVERNANCE_ID, spl_token_program_id,
};

const NAME: &str = "create_realm";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/create_realm.jsonc");
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
    pub realm_authority: Pubkey,
    #[serde(with = "value::pubkey")]
    pub community_token_mint: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub council_token_mint: Option<Pubkey>,
    pub community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    pub council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    pub name: String,
    pub min_weight: u64,
    pub max_weight_source: MintMaxVoterWeightSource,
    /// Set to true if the community token mint is a Token-2022 mint
    #[serde(default)]
    pub is_token_2022_for_community: bool,
    /// Set to true if the council token mint is a Token-2022 mint
    #[serde(default)]
    pub is_token_2022_for_council: bool,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// Adds accounts specified by GoverningTokenConfigAccountArgs
/// and returns GoverningTokenConfigArgs
pub fn with_governing_token_config_args(
    accounts: &mut Vec<AccountMeta>,
    governing_token_config_args: Option<GoverningTokenConfigAccountArgs>,
) -> GoverningTokenConfigArgs {
    let governing_token_config_args = governing_token_config_args.unwrap_or_default();

    let use_voter_weight_addin =
        if let Some(voter_weight_addin) = governing_token_config_args.voter_weight_addin {
            accounts.push(AccountMeta::new_readonly(voter_weight_addin, false));
            true
        } else {
            false
        };

    let use_max_voter_weight_addin =
        if let Some(max_voter_weight_addin) = governing_token_config_args.max_voter_weight_addin {
            accounts.push(AccountMeta::new_readonly(max_voter_weight_addin, false));
            true
        } else {
            false
        };

    GoverningTokenConfigArgs {
        use_voter_weight_addin,
        use_max_voter_weight_addin,
        token_type: governing_token_config_args.token_type,
    }
}

/// v3.1.2 account layout:
/// 0. `[writable]` Realm account (PDA)
/// 1. `[]` Realm authority
/// 2. `[]` Community token mint
/// 3. `[writable]` Community token holding (PDA)
/// 4. `[writable, signer]` Payer
/// 5. `[]` System program
/// 6. `[]` SPL Token program (Token or Token-2022 based on is_token_2022_for_community)
/// 7. `[]` Rent sysvar
/// (if council):
/// 8. `[]` Council token mint
/// 9. `[writable]` Council token holding (PDA)
/// 10. `[]` SPL Token program for council (Token or Token-2022)
/// Then: realm_config, voter_weight_addins...
#[allow(clippy::too_many_arguments)]
pub fn create_realm(
    program_id: &Pubkey,
    realm_authority: &Pubkey,
    community_token_mint: &Pubkey,
    payer: &Pubkey,
    council_token_mint: Option<Pubkey>,
    community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    name: String,
    min_community_weight_to_create_governance: u64,
    community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
    is_token_2022_for_community: bool,
    is_token_2022_for_council: bool,
) -> (Instruction, Pubkey, Pubkey) {
    let seeds = [b"governance", name.as_bytes()];
    let realm_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("realm_address: {:?}", realm_address);

    let seeds = [
        b"governance",
        realm_address.as_ref(),
        community_token_mint.as_ref(),
    ];
    let community_token_holding_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!(
        "community_token_holding_address: {:?}",
        community_token_holding_address
    );

    let mut accounts = vec![
        AccountMeta::new(realm_address, false),
        AccountMeta::new_readonly(*realm_authority, false),
        AccountMeta::new_readonly(*community_token_mint, false),
        AccountMeta::new(community_token_holding_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token_program_id(is_token_2022_for_community), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let use_council_mint = if let Some(council_token_mint) = council_token_mint {
        let seeds = [
            b"governance",
            realm_address.as_ref(),
            council_token_mint.as_ref(),
        ];
        let council_token_holding_address = Pubkey::find_program_address(&seeds, program_id).0;
        info!(
            "council_token_holding_address: {:?}",
            council_token_holding_address
        );

        accounts.push(AccountMeta::new_readonly(council_token_mint, false));
        accounts.push(AccountMeta::new(council_token_holding_address, false));
        accounts.push(AccountMeta::new_readonly(
            spl_token_program_id(is_token_2022_for_council),
            false,
        ));
        true
    } else {
        false
    };

    let seeds = [b"realm-config", realm_address.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("realm_config_address: {:?}", realm_config_address);
    accounts.push(AccountMeta::new(realm_config_address, false));

    let community_token_config_args =
        with_governing_token_config_args(&mut accounts, community_token_config_args);

    let council_token_config_args =
        with_governing_token_config_args(&mut accounts, council_token_config_args);

    let instruction = GovernanceInstruction::CreateRealm {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_weight_to_create_governance,
            community_mint_max_voter_weight_source,
            community_token_config_args,
            council_token_config_args,
        },
        name,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    };

    (instruction, realm_address, community_token_holding_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, realm, community_token) = create_realm(
        &program_id,
        &input.realm_authority,
        &input.community_token_mint,
        &input.fee_payer.pubkey(),
        input.council_token_mint,
        input.community_token_config_args,
        input.council_token_config_args,
        input.name,
        input.min_weight,
        input.max_weight_source,
        input.is_token_2022_for_community,
        input.is_token_2022_for_council,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "realm" => realm,
                "community_token_holding" => community_token
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::super::{MintMaxVoterWeightSource, SPL_GOVERNANCE_ID};
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let realm_authority = Pubkey::new_unique();
        let community_token_mint = Pubkey::new_unique();
        let payer = Pubkey::new_unique();

        let (ix, _, _) = create_realm(
            &SPL_GOVERNANCE_ID,
            &realm_authority,
            &community_token_mint,
            &payer,
            None,
            None,
            None,
            "test".to_string(),
            1000u64,
            MintMaxVoterWeightSource::SupplyFraction(10_000_000_000),
            false,
            false,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 8);
    }
}
