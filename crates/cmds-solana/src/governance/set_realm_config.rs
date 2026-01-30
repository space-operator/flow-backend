use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;
use tracing::info;

use crate::{
    governance::{RealmConfigArgs, create_realm::with_governing_token_config_args},
    prelude::*,
};

use super::{
    GovernanceInstruction, GoverningTokenConfigAccountArgs, MintMaxVoterWeightSource,
    SPL_GOVERNANCE_ID,
};

const NAME: &str = "set_realm_config";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/set_realm_config.json");
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

    pub realm_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub community_token_mint: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub council_token_mint: Option<Pubkey>,
    pub community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    pub council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    pub min_weight: u64,
    pub max_weight_source: MintMaxVoterWeightSource,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

#[allow(clippy::too_many_arguments)]
pub fn set_realm_config(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    council_token_mint: Option<Pubkey>,
    payer: &Pubkey,
    // Accounts  Args
    community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    // Args
    min_community_weight_to_create_governance: u64,
    community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new_readonly(*realm_authority, true),
    ];

    let use_council_mint = if let Some(council_token_mint) = council_token_mint {
        let seeds = [b"governance", realm.as_ref(), council_token_mint.as_ref()];
        let council_token_holding_address = Pubkey::find_program_address(&seeds, program_id).0;
        info!(
            "council_token_holding_address: {:?}",
            council_token_holding_address
        );

        accounts.push(AccountMeta::new_readonly(council_token_mint, false));
        accounts.push(AccountMeta::new(council_token_holding_address, false));
        true
    } else {
        false
    };

    accounts.push(AccountMeta::new_readonly(system_program::id(), false));

    // Always pass realm_config_address because it's needed when
    // use_community_voter_weight_addin is set to true but also when it's set to
    // false and the addin is being  removed from the realm
    let seeds = [b"realm-config", realm.as_ref()];
    let realm_config_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("realm_config_address: {:?}", realm_config_address);
    accounts.push(AccountMeta::new(realm_config_address, false));

    let community_token_config_args =
        with_governing_token_config_args(&mut accounts, community_token_config_args);

    let council_token_config_args =
        with_governing_token_config_args(&mut accounts, council_token_config_args);

    accounts.push(AccountMeta::new(*payer, true));

    let instruction = GovernanceInstruction::SetRealmConfig {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_weight_to_create_governance,
            community_mint_max_voter_weight_source,
            community_token_config_args,
            council_token_config_args,
        },
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = set_realm_config(
        &program_id,
        &input.realm,
        &input.realm_authority.pubkey(),
        input.council_token_mint,
        &input.fee_payer.pubkey(),
        input.community_token_config_args,
        input.council_token_config_args,
        input.min_weight,
        input.max_weight_source,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.realm_authority].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
