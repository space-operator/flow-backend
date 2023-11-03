use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

pub mod add_config_lines;
pub mod initialize;
pub mod initialize_candy_guard;
pub mod mint;
pub mod wrap;

/// Config line struct for storing asset (NFT) data pre-mint.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigLine {
    /// Name of the asset.
    pub name: String,
    /// URI to JSON metadata.
    pub uri: String,
}

// implement from ConfigLine mpl_candy_machine_core::ConfigLine
impl From<ConfigLine> for mpl_candy_machine_core::ConfigLine {
    fn from(config_line: ConfigLine) -> Self {
        Self {
            name: config_line.name,
            uri: config_line.uri,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CandyGuardData {
    pub default: GuardSet,
    pub groups: Option<Vec<Group>>,
}

impl From<CandyGuardData> for mpl_candy_guard::state::CandyGuardData {
    fn from(candy_guard_data: CandyGuardData) -> Self {
        Self {
            default: candy_guard_data.default.into(),
            groups: candy_guard_data
                .groups
                .map(|groups| groups.into_iter().map(|group| group.into()).collect()),
        }
    }
}

// A group represent a specific set of guards. When groups are used, transactions
/// must specify which group should be used during validation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub label: String,
    pub guards: GuardSet,
}

impl From<Group> for mpl_candy_guard::state::Group {
    fn from(group: Group) -> Self {
        Self {
            label: group.label,
            guards: group.guards.into(),
        }
    }
}

/// The set of guards available.
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub token2022_payment: Option<Token2022Payment>,
}

impl From<GuardSet> for mpl_candy_guard::state::GuardSet {
    fn from(guard_set: GuardSet) -> Self {
        Self {
            bot_tax: guard_set.bot_tax.map(|bot_tax| bot_tax.into()),
            sol_payment: guard_set.sol_payment.map(|sol_payment| sol_payment.into()),
            token_payment: guard_set
                .token_payment
                .map(|token_payment| token_payment.into()),
            start_date: guard_set.start_date.map(|start_date| start_date.into()),
            third_party_signer: guard_set
                .third_party_signer
                .map(|third_party_signer| third_party_signer.into()),
            token_gate: guard_set.token_gate.map(|token_gate| token_gate.into()),
            gatekeeper: guard_set.gatekeeper.map(|gatekeeper| gatekeeper.into()),
            end_date: guard_set.end_date.map(|end_date| end_date.into()),
            allow_list: guard_set.allow_list.map(|allow_list| allow_list.into()),
            mint_limit: guard_set.mint_limit.map(|mint_limit| mint_limit.into()),
            nft_payment: guard_set.nft_payment.map(|nft_payment| nft_payment.into()),
            redeemed_amount: guard_set
                .redeemed_amount
                .map(|redeemed_amount| redeemed_amount.into()),
            address_gate: guard_set
                .address_gate
                .map(|address_gate| address_gate.into()),
            nft_gate: guard_set.nft_gate.map(|nft_gate| nft_gate.into()),
            nft_burn: guard_set.nft_burn.map(|nft_burn| nft_burn.into()),
            token_burn: guard_set.token_burn.map(|token_burn| token_burn.into()),
            freeze_sol_payment: guard_set
                .freeze_sol_payment
                .map(|freeze_sol_payment| freeze_sol_payment.into()),
            freeze_token_payment: guard_set
                .freeze_token_payment
                .map(|freeze_token_payment| freeze_token_payment.into()),
            program_gate: guard_set
                .program_gate
                .map(|program_gate| program_gate.into()),
            allocation: guard_set.allocation.map(|allocation| allocation.into()),
            token2022_payment: guard_set
                .token2022_payment
                .map(|token2022_payment| token2022_payment.into()),
        }
    }
}

/// Guard that charges an amount in a specified spl-token as payment for the mint.
///
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Address of the ATA to receive the tokens.
///   2. `[]` Mint account.
///   3. `[]` SPL Token-2022 program account.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Token2022Payment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}

impl From<Token2022Payment> for mpl_candy_guard::guards::Token2022Payment {
    fn from(token2022_payment: Token2022Payment) -> Self {
        Self {
            amount: token2022_payment.amount,
            mint: token2022_payment.mint,
            destination_ata: token2022_payment.destination_ata,
        }
    }
}

/// Guard that restricts access to a specific address.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddressGate {
    pub address: Pubkey,
}

impl From<AddressGate> for mpl_candy_guard::guards::AddressGate {
    fn from(address_gate: AddressGate) -> Self {
        Self {
            address: address_gate.address,
        }
    }
}

/// Gaurd to specify the maximum number of mints in a guard set.
///
/// List of accounts required:
///
///   0. `[writable]` Mint tracker PDA. The PDA is derived
///                   using the seed `["allocation", allocation id,
///                   candy guard pubkey, candy machine pubkey]`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Allocation {
    /// Unique identifier of the allocation.
    pub id: u8,
    /// The size of the allocation.
    pub size: u32,
}

impl From<Allocation> for mpl_candy_guard::guards::Allocation {
    fn from(allocation: Allocation) -> Self {
        Self {
            id: allocation.id,
            limit: allocation.size,
        }
    }
}

/// Guard that uses a merkle tree to specify the addresses allowed to mint.
///
/// List of accounts required:
///
///   0. `[]` Pda created by the merkle proof instruction (seeds `["allow_list", merke tree root,
///           payer key, candy guard pubkey, candy machine pubkey]`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AllowList {
    /// Merkle root of the addresses allowed to mint.
    pub merkle_root: [u8; 32],
}

impl From<AllowList> for mpl_candy_guard::guards::AllowList {
    fn from(allow_list: AllowList) -> Self {
        Self {
            merkle_root: allow_list.merkle_root,
        }
    }
}

/// Guard is used to:
/// * charge a penalty for invalid transactions
/// * validate that the mint transaction is the last transaction
/// * verify that only authorized programs have instructions
///
/// The `bot_tax` is applied to any error that occurs during the
/// validation of the guards.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BotTax {
    pub lamports: u64,
    pub last_instruction: bool,
}

impl From<BotTax> for mpl_candy_guard::guards::BotTax {
    fn from(bot_tax: BotTax) -> Self {
        Self {
            lamports: bot_tax.lamports,
            last_instruction: bot_tax.last_instruction,
        }
    }
}

/// Guard that sets a specific date for the mint to stop.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EndDate {
    pub date: i64,
}

impl From<EndDate> for mpl_candy_guard::guards::EndDate {
    fn from(end_date: EndDate) -> Self {
        Self {
            date: end_date.date,
        }
    }
}

/// Guard that charges an amount in SOL (lamports) for the mint with a freeze period.
///
/// List of accounts required:
///
///   0. `[writable]` Freeze PDA to receive the funds (seeds `["freeze_escrow",
///           destination pubkey, candy guard pubkey, candy machine pubkey]`).
///   1. `[]` Associate token account of the NFT (seeds `[payer pubkey, token
///           program pubkey, nft mint pubkey]`).
///   2. `[optional]` Authorization rule set for the minted pNFT.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FreezeSolPayment {
    pub lamports: u64,
    pub destination: Pubkey,
}

impl From<FreezeSolPayment> for mpl_candy_guard::guards::FreezeSolPayment {
    fn from(freeze_sol_payment: FreezeSolPayment) -> Self {
        Self {
            lamports: freeze_sol_payment.lamports,
            destination: freeze_sol_payment.destination,
        }
    }
}

/// Guard that charges an amount in a specified spl-token as payment for the mint with a freeze period.
///
/// List of accounts required:
///
///   0. `[writable]` Freeze PDA to receive the funds (seeds `["freeze_escrow",
///           destination_ata pubkey, candy guard pubkey, candy machine pubkey]`).
///   1. `[]` Associate token account of the NFT (seeds `[payer pubkey, token
///           program pubkey, nft mint pubkey]`).
///   2. `[writable]` Token account holding the required amount.
///   3. `[writable]` Associate token account of the Freeze PDA (seeds `[freeze PDA
///                   pubkey, token program pubkey, nft mint pubkey]`).
///   4. `[optional]` Authorization rule set for the minted pNFT.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FreezeTokenPayment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}

impl From<FreezeTokenPayment> for mpl_candy_guard::guards::FreezeTokenPayment {
    fn from(freeze_token_payment: FreezeTokenPayment) -> Self {
        Self {
            amount: freeze_token_payment.amount,
            mint: freeze_token_payment.mint,
            destination_ata: freeze_token_payment.destination_ata,
        }
    }
}

/// Guard that validates if the payer of the transaction has a token from a specified
/// gateway network — in most cases, a token after completing a captcha challenge.
///
/// List of accounts required:
///
///   0. `[writeable]` Gatekeeper token account.
///   1. `[]` Gatekeeper program account.
///   2. `[]` Gatekeeper expire account.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Gatekeeper {
    /// The network for the gateway token required
    pub gatekeeper_network: Pubkey,
    /// Whether or not the token should expire after minting.
    /// The gatekeeper network must support this if true.
    pub expire_on_use: bool,
}

impl From<Gatekeeper> for mpl_candy_guard::guards::Gatekeeper {
    fn from(gatekeeper: Gatekeeper) -> Self {
        Self {
            gatekeeper_network: gatekeeper.gatekeeper_network,
            expire_on_use: gatekeeper.expire_on_use,
        }
    }
}

/// Gaurd to set a limit of mints per wallet.
///
/// List of accounts required:
///
///   0. `[writable]` Mint counter PDA. The PDA is derived
///                   using the seed `["mint_limit", mint guard id, payer key,
///                   candy guard pubkey, candy machine pubkey]`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MintLimit {
    /// Unique identifier of the mint limit.
    pub id: u8,
    /// Limit of mints per individual address.
    pub limit: u16,
}

impl From<MintLimit> for mpl_candy_guard::guards::MintLimit {
    fn from(mint_limit: MintLimit) -> Self {
        Self {
            id: mint_limit.id,
            limit: mint_limit.limit,
        }
    }
}

/// Guard that requires another NFT (token) from a specific collection to be burned.
///
/// List of accounts required:
///
///   0. `[writeable]` Token account of the NFT.
///   1. `[writeable]` Metadata account of the NFT.
///   2. `[writeable]` Master Edition account of the NFT.
///   3. `[writeable]` Mint account of the NFT.
///   4. `[writeable]` Collection metadata account of the NFT.
///   5. `[writeable]` Token Record of the NFT (pNFT).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftBurn {
    pub required_collection: Pubkey,
}

impl From<NftBurn> for mpl_candy_guard::guards::NftBurn {
    fn from(nft_burn: NftBurn) -> Self {
        Self {
            required_collection: nft_burn.required_collection,
        }
    }
}

/// Guard that restricts the transaction to holders of a specified collection.
///
/// List of accounts required:
///
///   0. `[]` Token account of the NFT.
///   1. `[]` Metadata account of the NFT.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftGate {
    pub required_collection: Pubkey,
}

impl From<NftGate> for mpl_candy_guard::guards::NftGate {
    fn from(nft_gate: NftGate) -> Self {
        Self {
            required_collection: nft_gate.required_collection,
        }
    }
}

/// Guard that charges another NFT (token) from a specific collection as payment
/// for the mint.
///
/// List of accounts required:
///
///   0. `[writeable]` Token account of the NFT.
///   1. `[writeable]` Metadata account of the NFT.
///   2. `[]` Mint account of the NFT.
///   3. `[]` Account to receive the NFT.
///   4. `[writeable]` Destination PDA key (seeds [destination pubkey, token program id, nft mint pubkey]).
///   5. `[]` spl-associate-token program ID.
///   6. `[]` Master edition (pNFT)
///   7. `[writable]` Owner token record (pNFT)
///   8. `[writable]` Destination token record (pNFT)
///   9. `[]` Token Authorization Rules program (pNFT)
///   10. `[]` Token Authorization Rules account (pNFT)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftPayment {
    pub required_collection: Pubkey,
    pub destination: Pubkey,
}

impl From<NftPayment> for mpl_candy_guard::guards::NftPayment {
    fn from(nft_payment: NftPayment) -> Self {
        Self {
            required_collection: nft_payment.required_collection,
            destination: nft_payment.destination,
        }
    }
}

/// Guard that restricts the programs that can be in a mint transaction. The guard allows the
/// necessary programs for the mint and any other program specified in the configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgramGate {
    pub additional: Vec<Pubkey>,
}

impl From<ProgramGate> for mpl_candy_guard::guards::ProgramGate {
    fn from(program_gate: ProgramGate) -> Self {
        Self {
            additional: program_gate.additional,
        }
    }
}

/// Guard that stop the mint once the specified amount of items
/// redeenmed is reached.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RedeemedAmount {
    pub maximum: u64,
}

impl From<RedeemedAmount> for mpl_candy_guard::guards::RedeemedAmount {
    fn from(redeemed_amount: RedeemedAmount) -> Self {
        Self {
            maximum: redeemed_amount.maximum,
        }
    }
}

/// Guard that charges an amount in SOL (lamports) for the mint.
///
/// List of accounts required:
///
///   0. `[]` Account to receive the funds.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SolPayment {
    pub lamports: u64,
    pub destination: Pubkey,
}

// convert SolPayment to mpl_candy_machine_core::SolPayment
impl From<SolPayment> for mpl_candy_guard::guards::SolPayment {
    fn from(sol_payment: SolPayment) -> Self {
        Self {
            destination: sol_payment.destination,
            lamports: sol_payment.lamports,
        }
    }
}

/// Guard that sets a specific start date for the mint.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartDate {
    pub date: i64,
}

impl From<StartDate> for mpl_candy_guard::guards::StartDate {
    fn from(start_date: StartDate) -> Self {
        Self {
            date: start_date.date,
        }
    }
}

/// Guard that requires a specified signer to validate the transaction.
///
/// List of accounts required:
///
///   0. `[signer]` Signer of the transaction.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThirdPartySigner {
    pub signer_key: Pubkey,
}

impl From<ThirdPartySigner> for mpl_candy_guard::guards::ThirdPartySigner {
    fn from(third_party_signer: ThirdPartySigner) -> Self {
        Self {
            signer_key: third_party_signer.signer_key,
        }
    }
}

/// Guard that requires addresses that hold an amount of a specified spl-token
/// and burns them.
///
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Token mint account.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenBurn {
    pub amount: u64,
    pub mint: Pubkey,
}

impl From<TokenBurn> for mpl_candy_guard::guards::TokenBurn {
    fn from(token_burn: TokenBurn) -> Self {
        Self {
            amount: token_burn.amount,
            mint: token_burn.mint,
        }
    }
}

/// Guard that restricts access to addresses that hold the specified spl-token.
///
/// List of accounts required:
///
///   0. `[]` Token account holding the required amount.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenGate {
    pub amount: u64,
    pub mint: Pubkey,
}

impl From<TokenGate> for mpl_candy_guard::guards::TokenGate {
    fn from(token_gate: TokenGate) -> Self {
        Self {
            amount: token_gate.amount,
            mint: token_gate.mint,
        }
    }
}

/// Guard that charges an amount in a specified spl-token as payment for the mint.
///
/// List of accounts required:
///
///   0. `[writable]` Token account holding the required amount.
///   1. `[writable]` Address of the ATA to receive the tokens.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenPayment {
    pub amount: u64,
    pub mint: Pubkey,
    pub destination_ata: Pubkey,
}

impl From<TokenPayment> for mpl_candy_guard::guards::TokenPayment {
    fn from(token_payment: TokenPayment) -> Self {
        Self {
            amount: token_payment.amount,
            mint: token_payment.mint,
            destination_ata: token_payment.destination_ata,
        }
    }
}
