use crate::prelude::*;
use anchor_lang::AnchorSerialize;
use borsh::{BorshDeserialize, BorshSerialize};
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata, MetadataDelegateRecord, TokenRecord},
    instructions::{
        DelegateAuthorityItemV1InstructionArgs, DelegateCollectionItemV1InstructionArgs,
        DelegateCollectionV1InstructionArgs, DelegateDataItemV1InstructionArgs,
        DelegateDataV1InstructionArgs, DelegateLockedTransferV1InstructionArgs,
        DelegateProgrammableConfigItemV1InstructionArgs,
        DelegateProgrammableConfigV1InstructionArgs, DelegateSaleV1InstructionArgs,
        DelegateStakingV1InstructionArgs, DelegateStandardV1InstructionArgs,
        DelegateTransferV1InstructionArgs, DelegateUtilityV1InstructionArgs, InstructionAccount,
    },
    types::{MetadataDelegateRole, TokenDelegateRole},
};
use solana_program::{system_program, sysvar};

use super::AuthorizationData;

// Command Name
const NAME: &str = "delegate_v1";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/v1/delegate_v1.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    pub delegate: Keypair,
    #[serde(with = "value::keypair")]
    update_authority: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    pub delegate_args: DelegateArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub enum DelegateType {
    Metadata(MetadataDelegateRole),
    Token(TokenDelegateRole),
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let (master_edition_account, _) = MasterEdition::find_pda(&input.mint_account);

    // get associated token account pda
    let token_account = spl_associated_token_account::get_associated_token_address(
        &input.fee_payer.pubkey(),
        &input.mint_account,
    );

    let token_record = TokenRecord::find_pda(&input.mint_account, &token_account).0;

    let delegate_role: DelegateType = match input.delegate_args {
        DelegateArgs::CollectionV1 { .. } => {
            DelegateType::Metadata(MetadataDelegateRole::Collection)
        }
        DelegateArgs::SaleV1 { .. } => DelegateType::Token(TokenDelegateRole::Sale),
        DelegateArgs::TransferV1 { .. } => DelegateType::Token(TokenDelegateRole::Transfer),
        DelegateArgs::DataV1 { .. } => DelegateType::Metadata(MetadataDelegateRole::Data),
        DelegateArgs::DataItemV1 { .. } => DelegateType::Metadata(MetadataDelegateRole::DataItem),
        DelegateArgs::UtilityV1 { .. } => DelegateType::Token(TokenDelegateRole::Utility),
        DelegateArgs::StakingV1 { .. } => DelegateType::Token(TokenDelegateRole::Staking),
        DelegateArgs::StandardV1 { amount: _ } => DelegateType::Token(TokenDelegateRole::Standard),
        DelegateArgs::LockedTransferV1 { .. } => {
            DelegateType::Token(TokenDelegateRole::LockedTransfer)
        }
        DelegateArgs::ProgrammableConfigV1 { .. } => {
            DelegateType::Metadata(MetadataDelegateRole::ProgrammableConfig)
        }
        DelegateArgs::AuthorityItemV1 { .. } => {
            DelegateType::Metadata(MetadataDelegateRole::AuthorityItem)
        }
        DelegateArgs::CollectionItemV1 { .. } => {
            DelegateType::Metadata(MetadataDelegateRole::CollectionItem)
        }
        DelegateArgs::ProgrammableConfigItemV1 { .. } => {
            DelegateType::Metadata(MetadataDelegateRole::ProgrammableConfigItem)
        }
    };

    let delegate_record = match delegate_role {
        DelegateType::Metadata(role) => {
            MetadataDelegateRecord::find_pda(
                &input.mint_account,
                role,
                &input.update_authority.pubkey(),
                &input.delegate.pubkey(),
            )
            .0
        }
        DelegateType::Token(..) => TokenRecord::find_pda(&input.mint_account, &token_account).0,
    };

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<DelegateV1>())
        .await?;

    let delegate_v1 = DelegateV1 {
        delegate_record: Some(delegate_record),
        delegate: input.delegate.pubkey(),
        metadata: metadata_account,
        master_edition: Some(master_edition_account),
        token_record: Some(token_record),
        mint: input.mint_account,
        // TODO: check if token account is correct
        token: Some(token_account),
        authority: input.update_authority.pubkey(),
        payer: input.fee_payer.pubkey(),
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
        spl_token_program: Some(spl_token::id()),
        authorization_rules_program: None,
        authorization_rules: None,
    };

    let create_ix = delegate_v1.instruction(input.delegate_args.into());

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.update_authority.clone_keypair(),
        ]
        .into(),
        instructions: [create_ix].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "delegate_record" => delegate_record,
                "token_record" => token_record,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

/// Accounts.
pub struct DelegateV1 {
    /// Delegate record account
    pub delegate_record: Option<solana_program::pubkey::Pubkey>,
    /// Owner of the delegated account
    pub delegate: solana_program::pubkey::Pubkey,
    /// Metadata account
    pub metadata: solana_program::pubkey::Pubkey,
    /// Master Edition account
    pub master_edition: Option<solana_program::pubkey::Pubkey>,
    /// Token record account
    pub token_record: Option<solana_program::pubkey::Pubkey>,
    /// Mint of metadata
    pub mint: solana_program::pubkey::Pubkey,
    /// Token account of mint
    pub token: Option<solana_program::pubkey::Pubkey>,
    /// Update authority or token owner
    pub authority: solana_program::pubkey::Pubkey,
    /// Payer
    pub payer: solana_program::pubkey::Pubkey,
    /// System Program
    pub system_program: solana_program::pubkey::Pubkey,
    /// Instructions sysvar account
    pub sysvar_instructions: solana_program::pubkey::Pubkey,
    /// SPL Token Program
    pub spl_token_program: Option<solana_program::pubkey::Pubkey>,
    /// Token Authorization Rules Program
    pub authorization_rules_program: Option<solana_program::pubkey::Pubkey>,
    /// Token Authorization Rules account
    pub authorization_rules: Option<solana_program::pubkey::Pubkey>,
}

impl DelegateV1 {
    pub fn instruction(
        &self,
        args: mpl_token_metadata::types::DelegateArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: mpl_token_metadata::types::DelegateArgs,
        remaining_accounts: &[InstructionAccount],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(14 + remaining_accounts.len());
        if let Some(delegate_record) = self.delegate_record {
            accounts.push(solana_program::instruction::AccountMeta::new(
                delegate_record,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.delegate,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.metadata,
            false,
        ));
        if let Some(master_edition) = self.master_edition {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                master_edition,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        if let Some(token_record) = self.token_record {
            accounts.push(solana_program::instruction::AccountMeta::new(
                token_record,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.mint, false,
        ));
        if let Some(token) = self.token {
            accounts.push(solana_program::instruction::AccountMeta::new(token, false));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.authority,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.payer, true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.system_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.sysvar_instructions,
            false,
        ));
        if let Some(spl_token_program) = self.spl_token_program {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                spl_token_program,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        if let Some(authorization_rules_program) = self.authorization_rules_program {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                authorization_rules_program,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        if let Some(authorization_rules) = self.authorization_rules {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                authorization_rules,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        remaining_accounts
            .iter()
            .for_each(|remaining_account| accounts.push(remaining_account.to_account_meta()));

        let (mut args, mut data) = match args {
            mpl_token_metadata::types::DelegateArgs::AuthorityItemV1 { authorization_data } => (
                DelegateAuthorityItemV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateAuthorityItemV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::CollectionItemV1 { authorization_data } => (
                DelegateCollectionItemV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateCollectionItemV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::DataItemV1 { authorization_data } => (
                DelegateDataItemV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateDataItemV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::ProgrammableConfigItemV1 {
                authorization_data,
            } => (
                DelegateProgrammableConfigItemV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateProgrammableConfigItemV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::CollectionV1 { authorization_data } => (
                DelegateCollectionV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateCollectionV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::SaleV1 {
                amount,
                authorization_data,
            } => (
                DelegateSaleV1InstructionArgs {
                    amount,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                DelegateSaleV1InstructionData::new().try_to_vec().unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::TransferV1 {
                amount,
                authorization_data,
            } => (
                DelegateTransferV1InstructionArgs {
                    amount,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                DelegateTransferV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::DataV1 { authorization_data } => (
                DelegateDataV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateDataV1InstructionData::new().try_to_vec().unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::UtilityV1 {
                amount,
                authorization_data,
            } => (
                DelegateUtilityV1InstructionArgs {
                    amount,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                DelegateUtilityV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::StakingV1 {
                amount,
                authorization_data,
            } => (
                DelegateStakingV1InstructionArgs {
                    amount,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                DelegateStakingV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::StandardV1 { amount } => (
                DelegateStandardV1InstructionArgs { amount }
                    .try_to_vec()
                    .unwrap(),
                DelegateStandardV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::LockedTransferV1 {
                amount,
                locked_address,
                authorization_data,
            } => (
                DelegateLockedTransferV1InstructionArgs {
                    amount,
                    locked_address,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                DelegateLockedTransferV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::DelegateArgs::ProgrammableConfigV1 {
                authorization_data,
            } => (
                DelegateProgrammableConfigV1InstructionArgs { authorization_data }
                    .try_to_vec()
                    .unwrap(),
                DelegateProgrammableConfigV1InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
        };

        data.append(&mut args);

        solana_program::instruction::Instruction {
            program_id: mpl_token_metadata::ID,
            accounts,
            data,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateAuthorityItemV1InstructionData {
    discriminator: u8,
    delegate_authority_item_v1_discriminator: u8,
}

impl DelegateAuthorityItemV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_authority_item_v1_discriminator: 9,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateCollectionItemV1InstructionData {
    discriminator: u8,
    delegate_collection_item_v1_discriminator: u8,
}

impl DelegateCollectionItemV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_collection_item_v1_discriminator: 11,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateCollectionV1InstructionData {
    discriminator: u8,
    delegate_collection_v1_discriminator: u8,
}

impl DelegateCollectionV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_collection_v1_discriminator: 0,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateDataItemV1InstructionData {
    discriminator: u8,
    delegate_data_item_v1_discriminator: u8,
}

impl DelegateDataItemV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_data_item_v1_discriminator: 10,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateDataV1InstructionData {
    discriminator: u8,
    delegate_data_v1_discriminator: u8,
}

impl DelegateDataV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_data_v1_discriminator: 3,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateLockedTransferV1InstructionData {
    discriminator: u8,
    delegate_locked_transfer_v1_discriminator: u8,
}

impl DelegateLockedTransferV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_locked_transfer_v1_discriminator: 7,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateProgrammableConfigItemV1InstructionData {
    discriminator: u8,
    delegate_programmable_config_item_v1_discriminator: u8,
}

impl DelegateProgrammableConfigItemV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_programmable_config_item_v1_discriminator: 12,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateProgrammableConfigV1InstructionData {
    discriminator: u8,
    delegate_programmable_config_v1_discriminator: u8,
}

impl DelegateProgrammableConfigV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_programmable_config_v1_discriminator: 8,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateSaleV1InstructionData {
    discriminator: u8,
    delegate_sale_v1_discriminator: u8,
}

impl DelegateSaleV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_sale_v1_discriminator: 1,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateStakingV1InstructionData {
    discriminator: u8,
    delegate_staking_v1_discriminator: u8,
}

impl DelegateStakingV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_staking_v1_discriminator: 5,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateStandardV1InstructionData {
    discriminator: u8,
    delegate_standard_v1_discriminator: u8,
}

impl DelegateStandardV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_standard_v1_discriminator: 6,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateTransferV1InstructionData {
    discriminator: u8,
    delegate_transfer_v1_discriminator: u8,
}

impl DelegateTransferV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_transfer_v1_discriminator: 2,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct DelegateUtilityV1InstructionData {
    discriminator: u8,
    delegate_utility_v1_discriminator: u8,
}

impl DelegateUtilityV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 44,
            delegate_utility_v1_discriminator: 4,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum DelegateArgs {
    CollectionV1 {
        authorization_data: Option<AuthorizationData>,
    },
    SaleV1 {
        amount: u64,
        authorization_data: Option<AuthorizationData>,
    },
    TransferV1 {
        amount: u64,
        authorization_data: Option<AuthorizationData>,
    },
    DataV1 {
        authorization_data: Option<AuthorizationData>,
    },
    UtilityV1 {
        amount: u64,
        authorization_data: Option<AuthorizationData>,
    },
    StakingV1 {
        amount: u64,
        authorization_data: Option<AuthorizationData>,
    },
    StandardV1 {
        amount: u64,
    },
    LockedTransferV1 {
        amount: u64,
        #[cfg_attr(
            feature = "serde",
            serde(with = "serde_with::As::<serde_with::DisplayFromStr>")
        )]
        locked_address: Pubkey,
        authorization_data: Option<AuthorizationData>,
    },
    ProgrammableConfigV1 {
        authorization_data: Option<AuthorizationData>,
    },
    AuthorityItemV1 {
        authorization_data: Option<AuthorizationData>,
    },
    DataItemV1 {
        authorization_data: Option<AuthorizationData>,
    },
    CollectionItemV1 {
        authorization_data: Option<AuthorizationData>,
    },
    ProgrammableConfigItemV1 {
        authorization_data: Option<AuthorizationData>,
    },
}

// implement from for DelegateArgs to mpl_token_metadata::types::DelegateArgs
impl From<DelegateArgs> for mpl_token_metadata::types::DelegateArgs {
    fn from(args: DelegateArgs) -> Self {
        match args {
            DelegateArgs::CollectionV1 { authorization_data } => Self::CollectionV1 {
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::SaleV1 {
                amount,
                authorization_data,
            } => Self::SaleV1 {
                amount,
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::TransferV1 {
                amount,
                authorization_data,
            } => Self::TransferV1 {
                amount,
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::DataV1 { authorization_data } => Self::DataV1 {
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::UtilityV1 {
                amount,
                authorization_data,
            } => Self::UtilityV1 {
                amount,
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::StakingV1 {
                amount,
                authorization_data,
            } => Self::StakingV1 {
                amount,
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::StandardV1 { amount } => Self::StandardV1 { amount },
            DelegateArgs::LockedTransferV1 {
                amount,
                locked_address,
                authorization_data,
            } => Self::LockedTransferV1 {
                amount,
                locked_address: locked_address
                    .to_bytes()
                    .try_into()
                    .expect("locked_address should be 32 bytes"),
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::ProgrammableConfigV1 { authorization_data } => {
                Self::ProgrammableConfigV1 {
                    authorization_data: authorization_data.map(Into::into),
                }
            }
            DelegateArgs::AuthorityItemV1 { authorization_data } => Self::AuthorityItemV1 {
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::DataItemV1 { authorization_data } => Self::DataItemV1 {
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::CollectionItemV1 { authorization_data } => Self::CollectionItemV1 {
                authorization_data: authorization_data.map(Into::into),
            },
            DelegateArgs::ProgrammableConfigItemV1 { authorization_data } => {
                Self::ProgrammableConfigItemV1 {
                    authorization_data: authorization_data.map(Into::into),
                }
            }
        }
    }
}
