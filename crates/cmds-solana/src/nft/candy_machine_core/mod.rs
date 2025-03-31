use anchor_lang::{error::Error, error_code};
use flow_lib::solana::Pubkey;
use mpl_core_candy_machine_core::constants::{
    HIDDEN_SECTION, MAX_NAME_LENGTH, MAX_URI_LENGTH, REPLACEMENT_INDEX, REPLACEMENT_INDEX_INCREMENT,
};
use serde::{Deserialize, Serialize};
use struct_convert::Convert;

pub mod add_config_lines_core;
pub mod initialize_core_candy_guards;
pub mod initialize_core_candy_machine;
pub mod mint_core;
pub mod wrap_core;

pub fn replace_patterns(value: String, index: usize) -> String {
    let mut mutable = value;
    // check for pattern $ID+1$
    if mutable.contains(REPLACEMENT_INDEX_INCREMENT) {
        mutable = mutable.replace(REPLACEMENT_INDEX_INCREMENT, &(index + 1).to_string());
    }
    // check for pattern $ID$
    if mutable.contains(REPLACEMENT_INDEX) {
        mutable = mutable.replace(REPLACEMENT_INDEX, &index.to_string());
    }

    mutable
}

#[error_code]
pub enum CandyError {
    #[msg("Account does not have correct owner")]
    IncorrectOwner,

    #[msg("Account is not initialized")]
    Uninitialized,

    #[msg("Mint Mismatch")]
    MintMismatch,

    #[msg("Index greater than length")]
    IndexGreaterThanLength,

    #[msg("Numerical overflow error")]
    NumericalOverflowError,

    #[msg("Can only provide up to 4 creators to candy machine (because candy machine is one)")]
    TooManyCreators,

    #[msg("Candy machine is empty")]
    CandyMachineEmpty,

    #[msg("Candy machines using hidden uris do not have config lines, they have a single hash representing hashed order")]
    HiddenSettingsDoNotHaveConfigLines,

    #[msg("Cannot change number of lines unless is a hidden config")]
    CannotChangeNumberOfLines,

    #[msg("Cannot switch to hidden settings after items available is greater than 0")]
    CannotSwitchToHiddenSettings,

    #[msg("Incorrect collection NFT authority")]
    IncorrectCollectionAuthority,

    #[msg("The metadata account has data in it, and this must be empty to mint a new NFT")]
    MetadataAccountMustBeEmpty,

    #[msg("Can't change collection settings after items have begun to be minted")]
    NoChangingCollectionDuringMint,

    #[msg("Value longer than expected maximum value")]
    ExceededLengthError,

    #[msg("Missing config lines settings")]
    MissingConfigLinesSettings,

    #[msg("Cannot increase the length in config lines settings")]
    CannotIncreaseLength,

    #[msg("Cannot switch from hidden settings")]
    CannotSwitchFromHiddenSettings,

    #[msg("Cannot change sequential index generation after items have begun to be minted")]
    CannotChangeSequentialIndexGeneration,

    #[msg("Collection public key mismatch")]
    CollectionKeyMismatch,

    #[msg("Could not retrive config line data")]
    CouldNotRetrieveConfigLineData,

    #[msg("Not all config lines were added to the candy machine")]
    NotFullyLoaded,

    #[msg("Instruction could not be created")]
    InstructionBuilderFailed,

    #[msg("Missing collection authority record")]
    MissingCollectionAuthorityRecord,

    #[msg("Missing metadata delegate record")]
    MissingMetadataDelegateRecord,

    #[msg("Invalid token standard")]
    InvalidTokenStandard,

    #[msg("Missing token account")]
    MissingTokenAccount,

    #[msg("Missing token record")]
    MissingTokenRecord,

    #[msg("Missing instructions sysvar account")]
    MissingInstructionsSysvar,

    #[msg("Missing SPL ATA program")]
    MissingSplAtaProgram,

    #[msg("Invalid account version")]
    InvalidAccountVersion,

    #[msg("Invalid plugin authority")]
    IncorrectPluginAuthority,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_machine_core::types::CandyMachineData")]
pub struct CandyMachineData {
    /// Number of assets available
    pub items_available: u64,
    /// Max supply of each individual asset (default 0)
    pub max_supply: u64,
    /// Indicates if the asset is mutable or not (default yes)
    pub is_mutable: bool,
    /// Config line settings
    pub config_line_settings: Option<ConfigLineSettings>,
    /// Hidden setttings
    pub hidden_settings: Option<HiddenSettings>,
}

/// Hidden settings for large mints used with off-chain data.
#[derive(Serialize, Deserialize, Clone, Default, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_machine_core::types::HiddenSettings")]
pub struct HiddenSettings {
    /// Asset prefix name
    pub name: String,
    /// Shared URI
    pub uri: String,
    /// Hash of the hidden settings file
    pub hash: [u8; 32],
}

/// Config line struct for storing asset (NFT) data pre-mint.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_machine_core::types::ConfigLine")]
pub struct ConfigLine {
    /// Name of the asset.
    pub name: String,
    /// URI to JSON metadata.
    pub uri: String,
}

/// Config line settings to allocate space for individual name + URI.
#[derive(Serialize, Deserialize, Clone, Default, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_machine_core::types::ConfigLineSettings")]
pub struct ConfigLineSettings {
    /// Common name prefix
    pub prefix_name: String,
    /// Length of the remaining part of the name
    pub name_length: u32,
    /// Common URI prefix
    pub prefix_uri: String,
    /// Length of the remaining part of the URI
    pub uri_length: u32,
    /// Indicates whether to use a senquential index generator or not
    pub is_sequential: bool,
}

impl CandyMachineData {
    pub fn get_space_for_candy(&self) -> Result<usize, CandyError> {
        Ok(if self.hidden_settings.is_some() {
            HIDDEN_SECTION
        } else {
            HIDDEN_SECTION
                + 4
                + (self.items_available as usize) * self.get_config_line_size()
                + (self
                    .items_available
                    .checked_div(8)
                    .ok_or(CandyError::NumericalOverflowError)?
                    + 1) as usize
                + (self.items_available as usize) * 4
        })
    }

    pub fn get_config_line_size(&self) -> usize {
        if let Some(config_line) = &self.config_line_settings {
            (config_line.name_length + config_line.uri_length) as usize
        } else {
            0
        }
    }

    /// Validates the hidden and config lines settings against the maximum
    /// allowed values for name and URI.
    ///
    /// Hidden settings take precedence over config lines since when hidden
    /// settings are used, the account does not need to include space for
    /// config lines.
    pub fn validate(&self) -> Result<(), Error> {
        // validation substitutes any variable for the maximum allowed index
        // to check the longest possible name and uri that can result from the
        // replacement of the variables

        if let Some(hidden) = &self.hidden_settings {
            // config line settings should not be enabled at the same time as hidden settings
            if self.config_line_settings.is_some() {
                return Err(CandyError::HiddenSettingsDoNotHaveConfigLines.into());
            }

            let expected = replace_patterns(hidden.name.clone(), self.items_available as usize);
            if MAX_NAME_LENGTH < expected.len() {
                return Err(CandyError::ExceededLengthError.into());
            }

            let expected = replace_patterns(hidden.uri.clone(), self.items_available as usize);
            if MAX_URI_LENGTH < expected.len() {
                return Err(CandyError::ExceededLengthError.into());
            }
        } else if let Some(config_line) = &self.config_line_settings {
            let expected = replace_patterns(
                config_line.prefix_name.clone(),
                self.items_available as usize,
            );
            if MAX_NAME_LENGTH < (expected.len() + config_line.name_length as usize) {
                return Err(CandyError::ExceededLengthError.into());
            }

            let expected = replace_patterns(
                config_line.prefix_uri.clone(),
                self.items_available as usize,
            );
            if MAX_URI_LENGTH < (expected.len() + config_line.uri_length as usize) {
                return Err(CandyError::ExceededLengthError.into());
            }
        } else {
            return Err(CandyError::MissingConfigLinesSettings.into());
        }

        Ok(())
    }
}

/// A group represent a specific set of guards. When groups are used, transactions
/// must specify which group should be used during validation.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::Group")]
pub struct Group {
    pub label: String,
    pub guards: GuardSet,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CandyGuardData {
    pub default: GuardSet,
    pub groups: Option<Vec<Group>>,
}

impl From<CandyGuardData> for mpl_core_candy_guard::types::CandyGuardData {
    fn from(value: CandyGuardData) -> Self {
        Self {
            default: value.default.into(),
            groups: value
                .groups
                .map(|vec| vec.into_iter().map(Into::into).collect()),
        }
    }
}

/// The set of guards available.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::GuardSet")]
pub struct GuardSet {
    /// Last instruction check and bot tax (penalty for invalid transactions).
    pub bot_tax: Option<BotTax>,
    /// Sol payment guard (set the price for the mint in lamports).
    pub sol_payment: Option<SolPayment>,
    /// Token payment guard (set the price for the mint in spl-token amount).
    pub token_payment: Option<TokenPayment>,
    /// Start data guard (controls when minting is allowed).
    pub start_date: Option<StartDate>,
    /// Third party signer guard (requires an extra signer for the transaction).
    pub third_party_signer: Option<ThirdPartySigner>,
    /// Token gate guard (restrict access to holders of a specific token).
    pub token_gate: Option<TokenGate>,
    /// Gatekeeper guard (captcha challenge).
    pub gatekeeper: Option<Gatekeeper>,
    /// End date guard (set an end date to stop the mint).
    pub end_date: Option<EndDate>,
    /// Allow list guard (curated list of allowed addresses).
    pub allow_list: Option<AllowList>,
    /// Mint limit guard (add a limit on the number of mints per wallet).
    pub mint_limit: Option<MintLimit>,
    /// NFT Payment (charge an NFT in order to mint).
    pub nft_payment: Option<NftPayment>,
    /// Redeemed amount guard (add a limit on the overall number of items minted).
    pub redeemed_amount: Option<RedeemedAmount>,
    /// Address gate (check access against a specified address).
    pub address_gate: Option<AddressGate>,
    /// NFT gate guard (check access based on holding a specified NFT).
    pub nft_gate: Option<NftGate>,
    /// NFT burn guard (burn a specified NFT).
    pub nft_burn: Option<NftBurn>,
    /// Token burn guard (burn a specified amount of spl-token).
    pub token_burn: Option<TokenBurn>,
    /// Freeze sol payment guard (set the price for the mint in lamports with a freeze period).
    pub freeze_sol_payment: Option<FreezeSolPayment>,
    /// Freeze token payment guard (set the price for the mint in spl-token amount with a freeze period).
    pub freeze_token_payment: Option<FreezeTokenPayment>,
    /// Program gate guard (restricts the programs that can be in a mint transaction).
    pub program_gate: Option<ProgramGate>,
    /// Allocation guard (specify the maximum number of mints in a group).
    pub allocation: Option<Allocation>,
    /// Token2022 payment guard (set the price for the mint in spl-token-2022 amount).
    pub token2022_payment: Option<Token2022Payment>,
    /// Sol fixed fee for launchpads, marketplaces to define custom fees
    pub sol_fixed_fee: Option<SolFixedFee>,
    /// NFT mint limit guard (add a limit on the number of mints per NFT).
    pub nft_mint_limit: Option<NftMintLimit>,
    /// NFT mint limit guard (add a limit on the number of mints per NFT).
    pub edition: Option<Edition>,
    /// Asset Payment (charge an Asset in order to mint).
    pub asset_payment: Option<AssetPayment>,
    /// Asset Burn (burn an Asset).
    pub asset_burn: Option<AssetBurn>,
    /// Asset mint limit guard (add a limit on the number of mints per asset).
    pub asset_mint_limit: Option<AssetMintLimit>,
    /// Asset Burn Multi (multi burn Assets).
    pub asset_burn_multi: Option<AssetBurnMulti>,
    /// Asset Payment Multi (multi pay Assets).
    pub asset_payment_multi: Option<AssetPaymentMulti>,
    /// Asset Gate (restrict access to holders of a specific asset).
    pub asset_gate: Option<AssetGate>,
    /// Vanity Mint (the address of the new asset must match a pattern).
    pub vanity_mint: Option<VanityMint>,
}

/// Available guard types.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GuardType {
    BotTax,
    SolPayment,
    TokenPayment,
    StartDate,
    ThirdPartySigner,
    TokenGate,
    Gatekeeper,
    EndDate,
    AllowList,
    MintLimit,
    NftPayment,
    RedeemedAmount,
    AddressGate,
    NftGate,
    NftBurn,
    TokenBurn,
    FreezeSolPayment,
    FreezeTokenPayment,
    ProgramGate,
    Allocation,
    Token2022Payment,
    SolFixedFee,
    NftMintLimit,
    Edition,
    AssetPayment,
    AssetBurn,
    AssetMintLimit,
    AssetBurnMulti,
    AssetPaymentMulti,
    AssetGate,
    VanityMint,
}

impl GuardType {
    pub fn as_mask(guard_type: GuardType) -> u64 {
        0b1u64 << (guard_type as u8)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetPayment")]
pub struct AssetPayment {
    pub required_collection: Pubkey,
    pub destination: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetBurn")]
pub struct AssetBurn {
    pub required_collection: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetMintLimit")]
pub struct AssetMintLimit {
    /// Unique identifier of the mint limit.
    pub id: u8,
    /// Limit of mints per individual mint address.
    pub limit: u16,
    /// Required collection of the mint.
    pub required_collection: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetBurnMulti")]
pub struct AssetBurnMulti {
    pub required_collection: Pubkey,
    pub num: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetPaymentMulti")]
pub struct AssetPaymentMulti {
    pub required_collection: Pubkey,
    pub destination: Pubkey,
    pub num: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AssetGate")]
pub struct AssetGate {
    pub required_collection: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::VanityMint")]
pub struct VanityMint {
    pub regex: String,
}

/// Guard that restricts access to a specific address.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AddressGate")]
pub struct AddressGate {
    pub address: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::Allocation")]
pub struct Allocation {
    /// Unique identifier of the allocation.
    pub id: u8,
    /// The limit of the allocation.
    pub limit: u32,
}

/// Guard is used to:
/// * charge a penalty for invalid transactions
/// * validate that the mint transaction is the last transaction
/// * verify that only authorized programs have instructions
///
/// The `bot_tax` is applied to any error that occurs during the
/// validation of the guards.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::BotTax")]
pub struct BotTax {
    pub lamports: u64,
    pub last_instruction: bool,
}

/// Guard that uses a merkle tree to specify the addresses allowed to mint.
///
/// List of accounts required:
///
///   0. `[]` Pda created by the merkle proof instruction (seeds `["allow_list", merke tree root,
///           payer key, candy guard pubkey, candy machine pubkey]`).
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::AllowList")]
pub struct AllowList {
    /// Merkle root of the addresses allowed to mint.
    pub merkle_root: [u8; 32],
}

/// Guard that adds an edition plugin to the asset.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::Edition")]
pub struct Edition {
    pub edition_start_offset: u32,
}

/// Guard that sets a specific date for the mint to stop.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::EndDate")]
pub struct EndDate {
    pub date: i64,
}

/// Guard that charges an amount in SOL (lamports) for the mint with a freeze period.
///
/// List of accounts required:
///
///   0. `[writable]` Freeze PDA to receive the funds (seeds `["freeze_escrow",
///           destination pubkey, candy guard pubkey, candy machine pubkey]`).
///   1. `[]` Associate token account of the NFT (seeds `[payer pubkey, token
///           program pubkey, nft mint pubkey]`).
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::FreezeSolPayment")]
pub struct FreezeSolPayment {
    pub lamports: u64,
    pub destination: Pubkey,
}

/// Guard that charges an amount in a specified spl-token as payment for the mint with a freeze period.
///
/// List of accounts required:
///
///   0. `[writable]` Freeze PDA to receive the funds (seeds `["freeze_escrow",
///           destination_ata pubkey, candy guard pubkey, candy machine pubkey]`).
///   1. `[writable]` Token account holding the required amount.
///   2. `[writable]` Associate token account of the Freeze PDA (seeds `[freeze PDA
///                   pubkey, token program pubkey, nft mint pubkey]`).
///   3. `[]` SPL Token program.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::FreezeTokenPayment")]
pub struct FreezeTokenPayment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}

/// Guard that validates if the payer of the transaction has a token from a specified
/// gateway network — in most cases, a token after completing a captcha challenge.
///
/// List of accounts required:
///
///   0. `[writeable]` Gatekeeper token account.
///   1. `[]` Gatekeeper program account.
///   2. `[]` Gatekeeper expire account.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::Gatekeeper")]
pub struct Gatekeeper {
    /// The network for the gateway token required
    pub gatekeeper_network: Pubkey,
    /// Whether or not the token should expire after minting.
    /// The gatekeeper network must support this if true.
    pub expire_on_use: bool,
}

/// Guard to set a limit of mints per wallet.
///
/// List of accounts required:
///
///   0. `[writable]` Mint counter PDA. The PDA is derived
///                   using the seed `["mint_limit", mint guard id, payer key,
///                   candy guard pubkey, candy machine pubkey]`.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::MintLimit")]
pub struct MintLimit {
    /// Unique identifier of the mint limit.
    pub id: u8,
    /// Limit of mints per individual address.
    pub limit: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::NftBurn")]
pub struct NftBurn {
    pub required_collection: Pubkey,
}

/// Guard that restricts the transaction to holders of a specified collection.
///
/// List of accounts required:
///
///   0. `[]` Token account of the NFT.
///   1. `[]` Metadata account of the NFT.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::NftGate")]
pub struct NftGate {
    pub required_collection: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::NftPayment")]
pub struct NftPayment {
    pub required_collection: Pubkey,
    pub destination: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::NftMintLimit")]
pub struct NftMintLimit {
    /// Unique identifier of the mint limit.
    pub id: u8,
    /// Limit of mints per individual mint address.
    pub limit: u16,
    /// Required collection of the mint.
    pub required_collection: Pubkey,
}

/// Guard that restricts the programs that can be in a mint transaction. The guard allows the
/// necessary programs for the mint and any other program specified in the configuration.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::ProgramGate")]
pub struct ProgramGate {
    pub additional: Vec<Pubkey>,
}

/// Guard that stop the mint once the specified amount of items
/// redeenmed is reached.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::RedeemedAmount")]
pub struct RedeemedAmount {
    pub maximum: u64,
}

/// Guard that charges an amount in SOL (lamports) for the mint.
///
/// List of accounts required:
///
///   0. `[]` Account to receive the fees.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::SolFixedFee")]
pub struct SolFixedFee {
    pub lamports: u64,
    pub destination: Pubkey,
}

/// Guard that charges an amount in SOL (lamports) for the mint.
///
/// List of accounts required:
///
///   0. `[]` Account to receive the funds.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::SolPayment")]
pub struct SolPayment {
    pub lamports: u64,
    pub destination: Pubkey,
}

/// Guard that sets a specific start date for the mint.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::StartDate")]
pub struct StartDate {
    pub date: i64,
}

/// Guard that requires a specified signer to validate the transaction.
///
/// List of accounts required:
///
///   0. `[signer]` Signer of the transaction.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::ThirdPartySigner")]
pub struct ThirdPartySigner {
    pub signer_key: Pubkey,
}

/// Guard that requires addresses that hold an amount of a specified spl-token
/// and burns them.
///
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Token mint account.
///   2. `[]` SPL token program.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::TokenBurn")]
pub struct TokenBurn {
    pub amount: u64,
    pub mint: Pubkey,
}

/// Guard that restricts access to addresses that hold the specified spl-token.
///
/// List of accounts required:
///
///   0. `[]` Token account holding the required amount.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::TokenGate")]
pub struct TokenGate {
    pub amount: u64,
    pub mint: Pubkey,
}

/// Guard that charges an amount in a specified spl-token as payment for the mint.
///
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Address of the ATA to receive the tokens.
///   2. `[]` SPL token program.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::TokenPayment")]
pub struct TokenPayment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}

/// Guard that charges an amount in a specified spl-token as payment for the mint.
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Address of the ATA to receive the tokens.
///   2. `[]` Mint account.
///   3. `[]` SPL Token-2022 program account.
#[derive(Serialize, Deserialize, Clone, Debug, Convert)]
#[convert(from_on = "mpl_core_candy_guard::types::Token2022Payment")]
pub struct Token2022Payment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}
