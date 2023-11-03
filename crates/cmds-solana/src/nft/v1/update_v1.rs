use crate::{
    nft::{CollectionDetails, NftCreator, NftUses, TokenStandard},
    prelude::*,
};
use anchor_lang::AnchorSerialize;
use borsh::{BorshDeserialize, BorshSerialize};
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    instructions::{
        InstructionAccount, UpdateAsAuthorityItemDelegateV2InstructionArgs,
        UpdateAsCollectionDelegateV2InstructionArgs,
        UpdateAsCollectionItemDelegateV2InstructionArgs, UpdateAsDataDelegateV2InstructionArgs,
        UpdateAsDataItemDelegateV2InstructionArgs,
        UpdateAsProgrammableConfigDelegateV2InstructionArgs,
        UpdateAsProgrammableConfigItemDelegateV2InstructionArgs,
        UpdateAsUpdateAuthorityV2InstructionArgs, UpdateV1InstructionArgs,
    },
};
use solana_program::{system_program, sysvar};

use super::AuthorizationData;

// Command Name
const NAME: &str = "update_v1";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/NFT/v1/update_v1.json");

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
    #[serde(default, with = "value::keypair::opt")]
    pub delegate: Option<Keypair>,
    #[serde(default, with = "value::pubkey::opt")]
    pub delegate_record: Option<Pubkey>,
    #[serde(default, with = "value::keypair::opt")]
    update_authority: Option<Keypair>,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    pub update_args: UpdateArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let (_master_edition_account, _) = MasterEdition::find_pda(&input.mint_account);

    // get associated token account pda
    let _token_account = spl_associated_token_account::get_associated_token_address(
        &input.fee_payer.pubkey(),
        &input.mint_account,
    );

    // let token_record = TokenRecord::find_pda(&input.mint_account, &token_account).0;

    let authority_or_delegate = input.delegate.unwrap_or_else(|| {
        input
            .update_authority
            .expect("update_authority field must be set")
    });

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<UpdateAsDelegateV1>())
        .await?;

    let delegate_v1 = UpdateAsDelegateV1 {
        authority: authority_or_delegate.pubkey(),
        delegate_record: input.delegate_record,
        // TODO
        token: None,
        mint: input.mint_account,
        metadata: metadata_account,
        // TODO: edition
        edition: None,
        payer: input.fee_payer.pubkey(),
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
        authorization_rules_program: None,
        authorization_rules: None,
    };

    let create_ix = delegate_v1.instruction(input.update_args.into());

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            authority_or_delegate.clone_keypair(),
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
                "authority or delegate" => authority_or_delegate.pubkey(),
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

/// Accounts.
pub struct UpdateAsDelegateV1 {
    /// Update authority or delegate
    pub authority: solana_program::pubkey::Pubkey,
    /// Delegate record PDA
    pub delegate_record: Option<solana_program::pubkey::Pubkey>,
    /// Token account
    pub token: Option<solana_program::pubkey::Pubkey>,
    /// Mint account
    pub mint: solana_program::pubkey::Pubkey,
    /// Metadata account
    pub metadata: solana_program::pubkey::Pubkey,
    /// Edition account
    pub edition: Option<solana_program::pubkey::Pubkey>,
    /// Payer
    pub payer: solana_program::pubkey::Pubkey,
    /// System program
    pub system_program: solana_program::pubkey::Pubkey,
    /// Instructions sysvar account
    pub sysvar_instructions: solana_program::pubkey::Pubkey,
    /// Token Authorization Rules Program
    pub authorization_rules_program: Option<solana_program::pubkey::Pubkey>,
    /// Token Authorization Rules account
    pub authorization_rules: Option<solana_program::pubkey::Pubkey>,
}

impl UpdateAsDelegateV1 {
    pub fn instruction(
        &self,
        args: mpl_token_metadata::types::UpdateArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: mpl_token_metadata::types::UpdateArgs,
        remaining_accounts: &[InstructionAccount],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(11 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.authority,
            true,
        ));
        if let Some(delegate_record) = self.delegate_record {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                delegate_record,
                false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
        if let Some(token) = self.token {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                token, false,
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
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.metadata,
            false,
        ));
        if let Some(edition) = self.edition {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                edition, false,
            ));
        } else {
            accounts.push(solana_program::instruction::AccountMeta::new_readonly(
                mpl_token_metadata::ID,
                false,
            ));
        }
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
            mpl_token_metadata::types::UpdateArgs::AsAuthorityItemDelegateV2 {
                new_update_authority,
                primary_sale_happened,
                is_mutable,
                token_standard,
                authorization_data,
            } => (
                UpdateAsAuthorityItemDelegateV2InstructionArgs {
                    new_update_authority,
                    primary_sale_happened,
                    is_mutable,
                    token_standard,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsAuthorityItemDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsCollectionDelegateV2 {
                collection,
                authorization_data,
            } => (
                UpdateAsCollectionDelegateV2InstructionArgs {
                    collection,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsCollectionDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsDataDelegateV2 {
                data,
                authorization_data,
            } => (
                UpdateAsDataDelegateV2InstructionArgs {
                    data,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsDataDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsProgrammableConfigDelegateV2 {
                rule_set,
                authorization_data,
            } => (
                UpdateAsProgrammableConfigDelegateV2InstructionArgs {
                    rule_set,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsProgrammableConfigDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsDataItemDelegateV2 {
                data,
                authorization_data,
            } => (
                UpdateAsDataItemDelegateV2InstructionArgs {
                    data,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsDataItemDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsCollectionItemDelegateV2 {
                collection,
                authorization_data,
            } => (
                UpdateAsCollectionItemDelegateV2InstructionArgs {
                    collection,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsCollectionItemDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsProgrammableConfigItemDelegateV2 {
                rule_set,
                authorization_data,
            } => (
                UpdateAsProgrammableConfigItemDelegateV2InstructionArgs {
                    rule_set,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsProgrammableConfigItemDelegateV2InstructionData::new()
                    .try_to_vec()
                    .unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::V1 {
                new_update_authority,
                data,
                primary_sale_happened,
                is_mutable,
                collection,
                collection_details,
                uses,
                rule_set,
                authorization_data,
            } => (
                UpdateV1InstructionArgs {
                    new_update_authority,
                    data,
                    primary_sale_happened,
                    is_mutable,
                    collection,
                    collection_details,
                    uses,
                    rule_set,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateV1InstructionData::new().try_to_vec().unwrap(),
            ),
            mpl_token_metadata::types::UpdateArgs::AsUpdateAuthorityV2 {
                new_update_authority,
                data,
                primary_sale_happened,
                is_mutable,
                collection,
                collection_details,
                uses,
                rule_set,
                token_standard,
                authorization_data,
            } => (
                UpdateAsUpdateAuthorityV2InstructionArgs {
                    new_update_authority,
                    data,
                    primary_sale_happened,
                    is_mutable,
                    collection,
                    collection_details,
                    uses,
                    rule_set,
                    token_standard,
                    authorization_data,
                }
                .try_to_vec()
                .unwrap(),
                UpdateAsUpdateAuthorityV2InstructionData::new()
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
struct UpdateAsAuthorityItemDelegateV2InstructionData {
    discriminator: u8,
    update_as_authority_item_delegate_v2_discriminator: u8,
}

impl UpdateAsAuthorityItemDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_authority_item_delegate_v2_discriminator: 2,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsCollectionDelegateV2InstructionData {
    discriminator: u8,
    update_as_collection_delegate_v2_discriminator: u8,
}

impl UpdateAsCollectionDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_collection_delegate_v2_discriminator: 3,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsCollectionItemDelegateV2InstructionData {
    discriminator: u8,
    update_as_collection_item_delegate_v2_discriminator: u8,
}

impl UpdateAsCollectionItemDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_collection_item_delegate_v2_discriminator: 7,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsDataDelegateV2InstructionData {
    discriminator: u8,
    update_as_data_delegate_v2_discriminator: u8,
}

impl UpdateAsDataDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_data_delegate_v2_discriminator: 4,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsDataItemDelegateV2InstructionData {
    discriminator: u8,
    update_as_data_item_delegate_v2_discriminator: u8,
}

impl UpdateAsDataItemDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_data_item_delegate_v2_discriminator: 6,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsProgrammableConfigDelegateV2InstructionData {
    discriminator: u8,
    update_as_programmable_config_delegate_v2_discriminator: u8,
}

impl UpdateAsProgrammableConfigDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_programmable_config_delegate_v2_discriminator: 5,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsProgrammableConfigItemDelegateV2InstructionData {
    discriminator: u8,
    update_as_programmable_config_item_delegate_v2_discriminator: u8,
}

impl UpdateAsProgrammableConfigItemDelegateV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_programmable_config_item_delegate_v2_discriminator: 8,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateAsUpdateAuthorityV2InstructionData {
    discriminator: u8,
    update_as_update_authority_v2_discriminator: u8,
}

impl UpdateAsUpdateAuthorityV2InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_as_update_authority_v2_discriminator: 1,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateMetadataAccountV2InstructionData {
    discriminator: u8,
}

impl Default for UpdateMetadataAccountV2InstructionData {
    fn default() -> Self {
        Self { discriminator: 15 }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
struct UpdateV1InstructionData {
    discriminator: u8,
    update_v1_discriminator: u8,
}

impl UpdateV1InstructionData {
    fn new() -> Self {
        Self {
            discriminator: 50,
            update_v1_discriminator: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum UpdateArgs {
    V1 {
        new_update_authority: Option<Pubkey>,
        data: Option<Data>,
        primary_sale_happened: Option<bool>,
        is_mutable: Option<bool>,
        collection: CollectionToggle,
        collection_details: CollectionDetailsToggle,
        uses: UsesToggle,
        rule_set: RuleSetToggle,
        authorization_data: Option<AuthorizationData>,
    },
    AsUpdateAuthorityV2 {
        new_update_authority: Option<Pubkey>,
        data: Option<Data>,
        primary_sale_happened: Option<bool>,
        is_mutable: Option<bool>,
        collection: CollectionToggle,
        collection_details: CollectionDetailsToggle,
        uses: UsesToggle,
        rule_set: RuleSetToggle,
        token_standard: Option<TokenStandard>,
        authorization_data: Option<AuthorizationData>,
    },
    AsAuthorityItemDelegateV2 {
        new_update_authority: Option<Pubkey>,
        primary_sale_happened: Option<bool>,
        is_mutable: Option<bool>,
        token_standard: Option<TokenStandard>,
        authorization_data: Option<AuthorizationData>,
    },
    AsCollectionDelegateV2 {
        collection: CollectionToggle,
        authorization_data: Option<AuthorizationData>,
    },
    AsDataDelegateV2 {
        data: Option<Data>,
        authorization_data: Option<AuthorizationData>,
    },
    AsProgrammableConfigDelegateV2 {
        rule_set: RuleSetToggle,
        authorization_data: Option<AuthorizationData>,
    },
    AsDataItemDelegateV2 {
        data: Option<Data>,
        authorization_data: Option<AuthorizationData>,
    },
    AsCollectionItemDelegateV2 {
        collection: CollectionToggle,
        authorization_data: Option<AuthorizationData>,
    },
    AsProgrammableConfigItemDelegateV2 {
        rule_set: RuleSetToggle,
        authorization_data: Option<AuthorizationData>,
    },
}

impl From<UpdateArgs> for mpl_token_metadata::types::UpdateArgs {
    fn from(args: UpdateArgs) -> Self {
        match args {
            UpdateArgs::AsAuthorityItemDelegateV2 {
                new_update_authority,
                primary_sale_happened,
                is_mutable,
                token_standard,
                authorization_data,
            } => Self::AsAuthorityItemDelegateV2 {
                new_update_authority,
                primary_sale_happened,
                is_mutable,
                token_standard: token_standard.map(Into::into),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsCollectionDelegateV2 {
                collection,
                authorization_data,
            } => Self::AsCollectionDelegateV2 {
                collection: collection.into(),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsDataDelegateV2 {
                data,
                authorization_data,
            } => Self::AsDataDelegateV2 {
                data: data.map(Into::into),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsProgrammableConfigDelegateV2 {
                rule_set,
                authorization_data,
            } => Self::AsProgrammableConfigDelegateV2 {
                rule_set: rule_set.into(),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsDataItemDelegateV2 {
                data,
                authorization_data,
            } => Self::AsDataItemDelegateV2 {
                data: data.map(Into::into),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsCollectionItemDelegateV2 {
                collection,
                authorization_data,
            } => Self::AsCollectionItemDelegateV2 {
                collection: collection.into(),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsProgrammableConfigItemDelegateV2 {
                rule_set,
                authorization_data,
            } => Self::AsProgrammableConfigItemDelegateV2 {
                rule_set: rule_set.into(),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::V1 {
                new_update_authority,
                data,
                primary_sale_happened,
                is_mutable,
                collection,
                collection_details,
                uses,
                rule_set,
                authorization_data,
            } => Self::V1 {
                new_update_authority,
                data: data.map(Into::into),
                primary_sale_happened,
                is_mutable,
                collection: collection.into(),
                collection_details: collection_details.into(),
                uses: uses.into(),
                rule_set: rule_set.into(),
                authorization_data: authorization_data.map(Into::into),
            },
            UpdateArgs::AsUpdateAuthorityV2 {
                new_update_authority,
                data,
                primary_sale_happened,
                is_mutable,
                collection,
                collection_details,
                uses,
                rule_set,
                token_standard,
                authorization_data,
            } => Self::AsUpdateAuthorityV2 {
                new_update_authority,
                data: data.map(Into::into),
                primary_sale_happened,
                is_mutable,
                collection: collection.into(),
                collection_details: collection_details.into(),
                uses: uses.into(),
                rule_set: rule_set.into(),
                token_standard: token_standard.map(Into::into),
                authorization_data: authorization_data.map(Into::into),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Data {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<NftCreator>>,
}

impl From<Data> for mpl_token_metadata::types::Data {
    fn from(data: Data) -> Self {
        Self {
            name: data.name,
            symbol: data.symbol,
            uri: data.uri,
            seller_fee_basis_points: data.seller_fee_basis_points,
            creators: data
                .creators
                .map(|creators| creators.into_iter().map(Into::into).collect()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum CollectionToggle {
    None,
    Clear,
    Set(Collection),
}

impl From<CollectionToggle> for mpl_token_metadata::types::CollectionToggle {
    fn from(toggle: CollectionToggle) -> Self {
        match toggle {
            CollectionToggle::None => Self::None,
            CollectionToggle::Clear => Self::Clear,
            CollectionToggle::Set(collection) => Self::Set(collection.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Collection {
    pub verified: bool,
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::DisplayFromStr>")
    )]
    pub key: Pubkey,
}

impl From<Collection> for mpl_token_metadata::types::Collection {
    fn from(collection: Collection) -> Self {
        Self {
            verified: collection.verified,
            key: collection.key,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum CollectionDetailsToggle {
    None,
    Clear,
    Set(CollectionDetails),
}

impl From<CollectionDetailsToggle> for mpl_token_metadata::types::CollectionDetailsToggle {
    fn from(toggle: CollectionDetailsToggle) -> Self {
        match toggle {
            CollectionDetailsToggle::None => Self::None,
            CollectionDetailsToggle::Clear => Self::Clear,
            CollectionDetailsToggle::Set(details) => Self::Set(details.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum UsesToggle {
    None,
    Clear,
    Set(NftUses),
}

impl From<UsesToggle> for mpl_token_metadata::types::UsesToggle {
    fn from(toggle: UsesToggle) -> Self {
        match toggle {
            UsesToggle::None => Self::None,
            UsesToggle::Clear => Self::Clear,
            UsesToggle::Set(uses) => Self::Set(uses.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum RuleSetToggle {
    None,
    Clear,
    Set(Pubkey),
}

impl From<RuleSetToggle> for mpl_token_metadata::types::RuleSetToggle {
    fn from(toggle: RuleSetToggle) -> Self {
        match toggle {
            RuleSetToggle::None => Self::None,
            RuleSetToggle::Clear => Self::Clear,
            RuleSetToggle::Set(pubkey) => Self::Set(pubkey),
        }
    }
}
