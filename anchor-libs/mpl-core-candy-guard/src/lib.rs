#![allow(warnings)]

use accounts::*;
use anchor_lang::prelude::*;
use events::*;
use mpl_candy_guard_derive::GuardSet;
use types::*;
#[doc = "Program ID of program `candy_guard`."]
pub static ID: Pubkey = __ID;
#[doc = r" Const version of `ID`"]
pub const ID_CONST: Pubkey = __ID_CONST;
#[doc = r" The name is intentionally prefixed with `__` in order to reduce to possibility of name"]
#[doc = r" clashes with the crate's `ID`."]
static __ID: Pubkey = Pubkey::new_from_array([
    168u8, 154u8, 164u8, 238u8, 27u8, 230u8, 250u8, 239u8, 97u8, 29u8, 219u8, 190u8, 234u8, 165u8,
    221u8, 143u8, 242u8, 5u8, 237u8, 51u8, 151u8, 28u8, 41u8, 64u8, 47u8, 215u8, 140u8, 161u8, 3u8,
    166u8, 166u8, 9u8,
]);
const __ID_CONST: Pubkey = Pubkey::new_from_array([
    168u8, 154u8, 164u8, 238u8, 27u8, 230u8, 250u8, 239u8, 97u8, 29u8, 219u8, 190u8, 234u8, 165u8,
    221u8, 143u8, 242u8, 5u8, 237u8, 51u8, 151u8, 28u8, 41u8, 64u8, 47u8, 215u8, 140u8, 161u8, 3u8,
    166u8, 166u8, 9u8,
]);
#[doc = r" Program definition."]
pub mod program {
    use super::*;
    #[doc = r" Program type"]
    #[derive(Clone)]
    pub struct CandyGuard;

    impl anchor_lang::Id for CandyGuard {
        fn id() -> Pubkey {
            super::__ID
        }
    }
}
#[doc = r" Program constants."]
pub mod constants {}
#[doc = r" Program account type definitions."]
pub mod accounts {
    use super::*;
    #[doc = " PDA to store the frozen funds."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct FreezeEscrow {
        pub candy_guard: Pubkey,
        pub candy_machine: Pubkey,
        pub frozen_count: u64,
        pub first_mint_time: Option<i64>,
        pub freeze_period: i64,
        pub destination: Pubkey,
        pub authority: Pubkey,
    }
    impl anchor_lang::AccountSerialize for FreezeEscrow {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
            if writer.write_all(FreezeEscrow::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    impl anchor_lang::AccountDeserialize for FreezeEscrow {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < FreezeEscrow::DISCRIMINATOR.len() {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..FreezeEscrow::DISCRIMINATOR.len()];
            if FreezeEscrow::DISCRIMINATOR != given_disc {
                return Err(anchor_lang::error!(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                )
                .with_account_name(stringify!(FreezeEscrow)));
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[FreezeEscrow::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    impl anchor_lang::Discriminator for FreezeEscrow {
        const DISCRIMINATOR: &'static [u8] = &[227u8, 186u8, 40u8, 152u8, 7u8, 174u8, 131u8, 184u8];
    }
    impl anchor_lang::Owner for FreezeEscrow {
        fn owner() -> Pubkey {
            super::__ID
        }
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct CandyGuard {
        pub base: Pubkey,
        pub bump: u8,
        pub authority: Pubkey,
    }
    impl anchor_lang::AccountSerialize for CandyGuard {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
            if writer.write_all(CandyGuard::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    impl anchor_lang::AccountDeserialize for CandyGuard {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < CandyGuard::DISCRIMINATOR.len() {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..CandyGuard::DISCRIMINATOR.len()];
            if CandyGuard::DISCRIMINATOR != given_disc {
                return Err(anchor_lang::error!(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                )
                .with_account_name(stringify!(CandyGuard)));
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[CandyGuard::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    impl anchor_lang::Discriminator for CandyGuard {
        const DISCRIMINATOR: &'static [u8] =
            &[44u8, 207u8, 199u8, 184u8, 112u8, 103u8, 34u8, 181u8];
    }
    impl anchor_lang::Owner for CandyGuard {
        fn owner() -> Pubkey {
            super::__ID
        }
    }
}
#[doc = r" Program event type definitions."]
pub mod events {
    use super::*;
}
#[doc = r" Program type definitions."]
#[doc = r""]
#[doc = r" Note that account and event type definitions are not included in this module, as they"]
#[doc = r" have their own dedicated modules."]
pub mod types {
    use super::*;
    #[doc = " Guard that restricts access to a specific address."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AddressGate {
        pub address: Pubkey,
    }
    #[doc = " Gaurd to specify the maximum number of mints in a guard set."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Allocation tracker PDA. The PDA is derived"]
    #[doc = " using the seed `[\"allocation\", allocation id,"]
    #[doc = " candy guard pubkey, candy machine pubkey]`."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Allocation {
        pub id: u8,
        pub limit: u32,
    }
    #[doc = " PDA to track the number of mints."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AllocationTracker {
        pub count: u32,
    }
    #[doc = " Guard that uses a merkle tree to specify the addresses allowed to mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Pda created by the merkle proof instruction (seeds `[\"allow_list\", merke tree root,"]
    #[doc = " payer key, candy guard pubkey, candy machine pubkey]`)."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AllowList {
        pub merkle_root: [u8; 32],
    }
    #[doc = " PDA to track whether an address has been validated or not."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AllowListProof {
        pub timestamp: i64,
    }
    #[doc = " Guard that requires a specific number of Assets from a specific collection to be burned."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Address of the collection."]
    #[doc = " x. `[writeable]` Address of the Asset(s)."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetBurnMulti {
        pub required_collection: Pubkey,
        pub num: u8,
    }
    #[doc = " Guard that requires another Core Asset from a specific collection to be burned."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Address of the asset."]
    #[doc = " 1. `[writeable]` Address of the required collection."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetBurn {
        pub required_collection: Pubkey,
    }
    #[doc = " Guard that restricts the transaction to holders of a specified collection."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Account of the Asset."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetGate {
        pub required_collection: Pubkey,
    }
    #[doc = " Gaurd to set a limit of mints per wallet based on holding a specific Core Asset."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Mint counter PDA. The PDA is derived"]
    #[doc = " using the seed `[\"asset_mint_limit\", asset mint guard id, mint key,"]
    #[doc = " candy guard pubkey, candy machine pubkey]`."]
    #[doc = " 1. `[]` Address the Core Asset."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetMintLimit {
        pub id: u8,
        pub limit: u16,
        pub required_collection: Pubkey,
    }
    #[doc = " PDA to track the number of mints for an individual address."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetMintCounter {
        pub count: u16,
    }
    #[doc = " Guard that requires a specific number of Assets from a specific collection to be used as payment."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Address of the collection."]
    #[doc = " 1. `[writeable]` Address of the destination."]
    #[doc = " x. `[writeable]` Address of the Asset(s)."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetPaymentMulti {
        pub required_collection: Pubkey,
        pub destination: Pubkey,
        pub num: u8,
    }
    #[doc = " Guard that charges another Core Asset from a specific collection as payment"]
    #[doc = " for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Asset address."]
    #[doc = " 1. `[]` Collection address."]
    #[doc = " 2. `[]` Destination address."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct AssetPayment {
        pub required_collection: Pubkey,
        pub destination: Pubkey,
    }
    #[doc = " Guard is used to:"]
    #[doc = " * charge a penalty for invalid transactions"]
    #[doc = " * validate that the mint transaction is the last transaction"]
    #[doc = " * verify that only authorized programs have instructions"]
    #[doc = ""]
    #[doc = " The `bot_tax` is applied to any error that occurs during the"]
    #[doc = " validation of the guards."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct BotTax {
        pub lamports: u64,
        pub last_instruction: bool,
    }
    #[doc = " Guard that adds an edition plugin to the asset."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Edition {
        pub edition_start_offset: u32,
    }
    #[doc = " Guard that sets a specific date for the mint to stop."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct EndDate {
        pub date: i64,
    }
    #[doc = " Guard that charges an amount in SOL (lamports) for the mint with a freeze period."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Freeze PDA to receive the funds (seeds `[\"freeze_escrow\","]
    #[doc = " destination pubkey, candy guard pubkey, candy machine pubkey]`)."]
    #[doc = " 1. `[]` Associate token account of the NFT (seeds `[payer pubkey, token"]
    #[doc = " program pubkey, nft mint pubkey]`)."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct FreezeSolPayment {
        pub lamports: u64,
        pub destination: Pubkey,
    }
    #[doc = " Guard that charges an amount in a specified spl-token as payment for the mint with a freeze period."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Freeze PDA to receive the funds (seeds `[\"freeze_escrow\","]
    #[doc = " destination_ata pubkey, candy guard pubkey, candy machine pubkey]`)."]
    #[doc = " 1. `[writable]` Token account holding the required amount."]
    #[doc = " 2. `[writable]` Associate token account of the Freeze PDA (seeds `[freeze PDA"]
    #[doc = " pubkey, token program pubkey, nft mint pubkey]`)."]
    #[doc = " 3. `[]` SPL Token program."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct FreezeTokenPayment {
        pub amount: u64,
        pub mint: Pubkey,
        pub destination_ata: Pubkey,
    }
    #[doc = " Guard that validates if the payer of the transaction has a token from a specified"]
    #[doc = " gateway network — in most cases, a token after completing a captcha challenge."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Gatekeeper token account."]
    #[doc = " 1. `[]` Gatekeeper program account."]
    #[doc = " 2. `[]` Gatekeeper expire account."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Gatekeeper {
        pub gatekeeper_network: Pubkey,
        pub expire_on_use: bool,
    }
    #[doc = " Gaurd to set a limit of mints per wallet."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Mint counter PDA. The PDA is derived"]
    #[doc = " using the seed `[\"mint_limit\", mint guard id, payer key,"]
    #[doc = " candy guard pubkey, candy machine pubkey]`."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct MintLimit {
        pub id: u8,
        pub limit: u16,
    }
    #[doc = " PDA to track the number of mints for an individual address."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct MintCounter {
        pub count: u16,
    }
    #[doc = " Guard that requires another NFT (token) from a specific collection to be burned."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Token account of the NFT."]
    #[doc = " 1. `[writeable]` Metadata account of the NFT."]
    #[doc = " 2. `[writeable]` Master Edition account of the NFT."]
    #[doc = " 3. `[writeable]` Mint account of the NFT."]
    #[doc = " 4. `[writeable]` Collection metadata account of the NFT."]
    #[doc = " 5. `[]` SPL token program."]
    #[doc = " 6. `[]` Token Metadata program."]
    #[doc = " 7. `[writeable]` Token Record of the NFT (pNFT)."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct NftBurn {
        pub required_collection: Pubkey,
    }
    #[doc = " Guard that restricts the transaction to holders of a specified collection."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Token account of the NFT."]
    #[doc = " 1. `[]` Metadata account of the NFT."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct NftGate {
        pub required_collection: Pubkey,
    }
    #[doc = " Gaurd to set a limit of mints per wallet based on holding a specific NFT."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Mint counter PDA. The PDA is derived"]
    #[doc = " using the seed `[\"nft_mint_limit\", nft mint guard id, mint key,"]
    #[doc = " candy guard pubkey, candy machine pubkey]`."]
    #[doc = " 1. `[]` Token account of the NFT."]
    #[doc = " 2. `[]` Metadata account of the NFT."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct NftMintLimit {
        pub id: u8,
        pub limit: u16,
        pub required_collection: Pubkey,
    }
    #[doc = " PDA to track the number of mints for an individual address."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct NftMintCounter {
        pub count: u16,
    }
    #[doc = " Guard that charges another NFT (token) from a specific collection as payment"]
    #[doc = " for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writeable]` Token account of the NFT."]
    #[doc = " 1. `[writeable]` Metadata account of the NFT."]
    #[doc = " 2. `[]` Mint account of the NFT."]
    #[doc = " 3. `[]` Account to receive the NFT."]
    #[doc = " 4. `[writeable]` Destination PDA key (seeds [destination pubkey, token program id, nft mint pubkey])."]
    #[doc = " 5. `[]` spl-associate-token program ID."]
    #[doc = " 6. `[]` SPL token program."]
    #[doc = " 7. `[]` Token Metadata program."]
    #[doc = " 8. `[]` Master edition (pNFT)"]
    #[doc = " 9. `[writable]` Owner token record (pNFT)"]
    #[doc = " 10. `[writable]` Destination token record (pNFT)"]
    #[doc = " 11. `[]` Token Authorization Rules program (pNFT)"]
    #[doc = " 12. `[]` Token Authorization Rules account (pNFT)"]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct NftPayment {
        pub required_collection: Pubkey,
        pub destination: Pubkey,
    }
    #[doc = " Guard that restricts the programs that can be in a mint transaction. The guard allows the"]
    #[doc = " necessary programs for the mint and any other program specified in the configuration."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct ProgramGate {
        pub additional: Vec<Pubkey>,
    }
    #[doc = " Guard that stop the mint once the specified amount of items"]
    #[doc = " redeenmed is reached."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct RedeemedAmount {
        pub maximum: u64,
    }
    #[doc = " Guard that charges an amount in SOL (lamports) for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Account to receive the fees."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct SolFixedFee {
        pub lamports: u64,
        pub destination: Pubkey,
    }
    #[doc = " Guard that charges an amount in SOL (lamports) for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Account to receive the funds."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct SolPayment {
        pub lamports: u64,
        pub destination: Pubkey,
    }
    #[doc = " Guard that sets a specific start date for the mint."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct StartDate {
        pub date: i64,
    }
    #[doc = " Guard that requires a specified signer to validate the transaction."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[signer]` Signer of the transaction."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct ThirdPartySigner {
        pub signer_key: Pubkey,
    }
    #[doc = " Guard that charges an amount in a specified spl-token as payment for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Token account holding the required amount."]
    #[doc = " 1. `[writable]` Address of the ATA to receive the tokens."]
    #[doc = " 2. `[]` Mint account."]
    #[doc = " 3. `[]` SPL Token-2022 program account."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Token2022Payment {
        pub amount: u64,
        pub mint: Pubkey,
        pub destination_ata: Pubkey,
    }
    #[doc = " Guard that requires addresses that hold an amount of a specified spl-token"]
    #[doc = " and burns them."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Token account holding the required amount."]
    #[doc = " 1. `[writable]` Token mint account."]
    #[doc = " 2. `[]` SPL token program."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct TokenBurn {
        pub amount: u64,
        pub mint: Pubkey,
    }
    #[doc = " Guard that restricts access to addresses that hold the specified spl-token."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[]` Token account holding the required amount."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct TokenGate {
        pub amount: u64,
        pub mint: Pubkey,
    }
    #[doc = " Guard that charges an amount in a specified spl-token as payment for the mint."]
    #[doc = ""]
    #[doc = " List of accounts required:"]
    #[doc = ""]
    #[doc = " 0. `[writable]` Token account holding the required amount."]
    #[doc = " 1. `[writable]` Address of the ATA to receive the tokens."]
    #[doc = " 2. `[]` SPL token program."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct TokenPayment {
        pub amount: u64,
        pub mint: Pubkey,
        pub destination_ata: Pubkey,
    }
    #[doc = " Guard that sets a specific start date for the mint."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct VanityMint {
        pub regex: String,
    }
    #[doc = " Arguments for a route transaction."]
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct RouteArgs {
        pub guard: GuardType,
        pub data: Vec<u8>,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct CandyGuardData {
        pub default: GuardSet,
        pub groups: Option<Vec<Group>>,
    }
    #[doc = " A group represent a specific set of guards. When groups are used, transactions"]
    #[doc = " must specify which group should be used during validation."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct Group {
        pub label: String,
        pub guards: GuardSet,
    }
    #[doc = " The set of guards available."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, GuardSet)]
    pub struct GuardSet {
        pub bot_tax: Option<BotTax>,
        pub sol_payment: Option<SolPayment>,
        pub token_payment: Option<TokenPayment>,
        pub start_date: Option<StartDate>,
        pub third_party_signer: Option<ThirdPartySigner>,
        pub token_gate: Option<TokenGate>,
        pub gatekeeper: Option<Gatekeeper>,
        pub end_date: Option<EndDate>,
        pub allow_list: Option<AllowList>,
        pub mint_limit: Option<MintLimit>,
        pub nft_payment: Option<NftPayment>,
        pub redeemed_amount: Option<RedeemedAmount>,
        pub address_gate: Option<AddressGate>,
        pub nft_gate: Option<NftGate>,
        pub nft_burn: Option<NftBurn>,
        pub token_burn: Option<TokenBurn>,
        pub freeze_sol_payment: Option<FreezeSolPayment>,
        pub freeze_token_payment: Option<FreezeTokenPayment>,
        pub program_gate: Option<ProgramGate>,
        pub allocation: Option<Allocation>,
        pub token2022_payment: Option<Token2022Payment>,
        pub sol_fixed_fee: Option<SolFixedFee>,
        pub nft_mint_limit: Option<NftMintLimit>,
        pub edition: Option<Edition>,
        pub asset_payment: Option<AssetPayment>,
        pub asset_burn: Option<AssetBurn>,
        pub asset_mint_limit: Option<AssetMintLimit>,
        pub asset_burn_multi: Option<AssetBurnMulti>,
        pub asset_payment_multi: Option<AssetPaymentMulti>,
        pub asset_gate: Option<AssetGate>,
        pub vanity_mint: Option<VanityMint>,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum FreezeInstruction {
        Initialize,
        Thaw,
        UnlockFunds,
    }
    #[doc = " Available guard types."]
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
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
}
#[doc = r" Cross program invocation (CPI) helpers."]
pub mod cpi {
    use super::*;
    pub fn initialize<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Initialize<'info>>,
        data: Vec<u8>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Initialize { data };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Initialize::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn mint_v1<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::MintV1<'info>>,
        mint_args: Vec<u8>,
        label: Option<String>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::MintV1 { mint_args, label };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::MintV1::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn route<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Route<'info>>,
        args: RouteArgs,
        label: Option<String>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Route { args, label };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Route::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn set_authority<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetAuthority<'info>>,
        new_authority: Pubkey,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetAuthority { new_authority };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetAuthority::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn unwrap<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Unwrap<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Unwrap;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Unwrap::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn update<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Update<'info>>,
        data: Vec<u8>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Update { data };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Update::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn withdraw<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Withdraw<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Withdraw;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Withdraw::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn wrap<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Wrap<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Wrap;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Wrap::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program.key(),
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub struct Return<T> {
        phantom: std::marker::PhantomData<T>,
    }
    impl<T: AnchorDeserialize> Return<T> {
        pub fn get(&self) -> T {
            let (_key, data) = anchor_lang::solana_program::program::get_return_data().unwrap();
            T::try_from_slice(&data).unwrap()
        }
    }
    pub mod accounts {
        pub use super::internal::__cpi_client_accounts_initialize::*;
        pub use super::internal::__cpi_client_accounts_mint_v1::*;
        pub use super::internal::__cpi_client_accounts_route::*;
        pub use super::internal::__cpi_client_accounts_set_authority::*;
        pub use super::internal::__cpi_client_accounts_unwrap::*;
        pub use super::internal::__cpi_client_accounts_update::*;
        pub use super::internal::__cpi_client_accounts_withdraw::*;
        pub use super::internal::__cpi_client_accounts_wrap::*;
    }
}
#[doc = r" Off-chain client helpers."]
pub mod client {
    use super::*;
    #[doc = r" Client args."]
    pub mod args {
        pub use super::internal::args::*;
    }
    pub mod accounts {
        pub use super::internal::__client_accounts_initialize::*;
        pub use super::internal::__client_accounts_mint_v1::*;
        pub use super::internal::__client_accounts_route::*;
        pub use super::internal::__client_accounts_set_authority::*;
        pub use super::internal::__client_accounts_unwrap::*;
        pub use super::internal::__client_accounts_update::*;
        pub use super::internal::__client_accounts_withdraw::*;
        pub use super::internal::__client_accounts_wrap::*;
    }
}
#[doc(hidden)]
mod internal {
    use super::*;
    #[doc = r" An Anchor generated module containing the program's set of instructions, where each"]
    #[doc = r" method handler in the `#[program]` mod is associated with a struct defining the input"]
    #[doc = r" arguments to the method. These should be used directly, when one wants to serialize"]
    #[doc = r" Anchor instruction data, for example, when specifying instructions instructions on a"]
    #[doc = r" client."]
    pub mod args {
        use super::*;
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Initialize {
            pub data: Vec<u8>,
        }
        impl anchor_lang::Discriminator for Initialize {
            const DISCRIMINATOR: &'static [u8] =
                &[175u8, 175u8, 109u8, 31u8, 13u8, 152u8, 155u8, 237u8];
        }
        impl anchor_lang::InstructionData for Initialize {}

        impl anchor_lang::Owner for Initialize {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct MintV1 {
            pub mint_args: Vec<u8>,
            pub label: Option<String>,
        }
        impl anchor_lang::Discriminator for MintV1 {
            const DISCRIMINATOR: &'static [u8] =
                &[145u8, 98u8, 192u8, 118u8, 184u8, 147u8, 118u8, 104u8];
        }
        impl anchor_lang::InstructionData for MintV1 {}

        impl anchor_lang::Owner for MintV1 {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Route {
            pub args: RouteArgs,
            pub label: Option<String>,
        }
        impl anchor_lang::Discriminator for Route {
            const DISCRIMINATOR: &'static [u8] =
                &[229u8, 23u8, 203u8, 151u8, 122u8, 227u8, 173u8, 42u8];
        }
        impl anchor_lang::InstructionData for Route {}

        impl anchor_lang::Owner for Route {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetAuthority {
            pub new_authority: Pubkey,
        }
        impl anchor_lang::Discriminator for SetAuthority {
            const DISCRIMINATOR: &'static [u8] =
                &[133u8, 250u8, 37u8, 21u8, 110u8, 163u8, 26u8, 121u8];
        }
        impl anchor_lang::InstructionData for SetAuthority {}

        impl anchor_lang::Owner for SetAuthority {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Unwrap;

        impl anchor_lang::Discriminator for Unwrap {
            const DISCRIMINATOR: &'static [u8] =
                &[126u8, 175u8, 198u8, 14u8, 212u8, 69u8, 50u8, 44u8];
        }
        impl anchor_lang::InstructionData for Unwrap {}

        impl anchor_lang::Owner for Unwrap {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Update {
            pub data: Vec<u8>,
        }
        impl anchor_lang::Discriminator for Update {
            const DISCRIMINATOR: &'static [u8] =
                &[219u8, 200u8, 88u8, 176u8, 158u8, 63u8, 253u8, 127u8];
        }
        impl anchor_lang::InstructionData for Update {}

        impl anchor_lang::Owner for Update {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Withdraw;

        impl anchor_lang::Discriminator for Withdraw {
            const DISCRIMINATOR: &'static [u8] =
                &[183u8, 18u8, 70u8, 156u8, 148u8, 109u8, 161u8, 34u8];
        }
        impl anchor_lang::InstructionData for Withdraw {}

        impl anchor_lang::Owner for Withdraw {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Wrap;

        impl anchor_lang::Discriminator for Wrap {
            const DISCRIMINATOR: &'static [u8] =
                &[178u8, 40u8, 10u8, 189u8, 228u8, 129u8, 186u8, 140u8];
        }
        impl anchor_lang::InstructionData for Wrap {}

        impl anchor_lang::Owner for Wrap {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_initialize {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Initialize`]."]
        pub struct Initialize<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub base: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Initialize<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_guard),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.base),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Initialize<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.base));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_mint_v1 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`MintV1`]."]
        pub struct MintV1<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_authority_pda:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub minter: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub owner: Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub asset: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mpl_core_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_instructions: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub recent_slothashes: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for MintV1<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_guard),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_machine_program),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine_authority_pda),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.minter),
                    true,
                ));
                if let Some(owner) = &self.owner {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(owner),
                            false,
                        ),
                    );
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.asset),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.mpl_core_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.sysvar_instructions),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.recent_slothashes),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for MintV1<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.minter));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.owner));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.asset));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.mpl_core_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.sysvar_instructions,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.recent_slothashes,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_route {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Route`]."]
        pub struct Route<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Route<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_guard),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Route<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_set_authority {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetAuthority`]."]
        pub struct SetAuthority<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetAuthority<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_guard),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetAuthority<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_unwrap {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Unwrap`]."]
        pub struct Unwrap<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Unwrap<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_guard),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_machine_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_machine_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Unwrap<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_program,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_update {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Update`]."]
        pub struct Update<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Update<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_guard),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Update<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_withdraw {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Withdraw`]."]
        pub struct Withdraw<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Withdraw<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_guard),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority),
                    true,
                ));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Withdraw<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_wrap {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Wrap`]."]
        pub struct Wrap<'info> {
            pub candy_guard: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub candy_machine_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Wrap<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_guard),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_machine_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.candy_machine_authority),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Wrap<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_guard,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine_authority,
                ));
                account_infos
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_initialize {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Initialize`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Initialize {
            pub candy_guard: Pubkey,
            pub base: Pubkey,
            pub authority: Pubkey,
            pub payer: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Initialize {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_guard,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.base, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_mint_v1 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`MintV1`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct MintV1 {
            pub candy_guard: Pubkey,
            pub candy_machine_program: Pubkey,
            pub candy_machine: Pubkey,
            pub candy_machine_authority_pda: Pubkey,
            pub payer: Pubkey,
            pub minter: Pubkey,
            pub owner: Option<Pubkey>,
            pub asset: Pubkey,
            pub collection: Pubkey,
            pub mpl_core_program: Pubkey,
            pub system_program: Pubkey,
            pub sysvar_instructions: Pubkey,
            pub recent_slothashes: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for MintV1 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_guard,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_machine_program,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine_authority_pda,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.minter,
                    true,
                ));
                if let Some(owner) = &self.owner {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *owner, false,
                        ),
                    );
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.asset, false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.mpl_core_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.sysvar_instructions,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.recent_slothashes,
                        false,
                    ),
                );
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_route {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Route`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Route {
            pub candy_guard: Pubkey,
            pub candy_machine: Pubkey,
            pub payer: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Route {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_guard,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_set_authority {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetAuthority`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetAuthority {
            pub candy_guard: Pubkey,
            pub authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetAuthority {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_guard,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_unwrap {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Unwrap`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Unwrap {
            pub candy_guard: Pubkey,
            pub authority: Pubkey,
            pub candy_machine: Pubkey,
            pub candy_machine_authority: Pubkey,
            pub candy_machine_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Unwrap {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_guard,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_machine_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_machine_program,
                        false,
                    ),
                );
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_update {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Update`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Update {
            pub candy_guard: Pubkey,
            pub authority: Pubkey,
            pub payer: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Update {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_guard,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_withdraw {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Withdraw`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Withdraw {
            pub candy_guard: Pubkey,
            pub authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Withdraw {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_guard,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority,
                    true,
                ));
                account_metas
            }
        }
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_wrap {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Wrap`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Wrap {
            pub candy_guard: Pubkey,
            pub authority: Pubkey,
            pub candy_machine: Pubkey,
            pub candy_machine_program: Pubkey,
            pub candy_machine_authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Wrap {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_guard,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_machine_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.candy_machine_authority,
                        true,
                    ),
                );
                account_metas
            }
        }
    }
}
#[doc = r" Program utilities."]
pub mod utils {
    use super::*;
    #[doc = r" An enum that includes all accounts of the declared program as a tuple variant."]
    #[doc = r""]
    #[doc = r" See [`Self::try_from_bytes`] to create an instance from bytes."]
    pub enum Account {
        FreezeEscrow(FreezeEscrow),
        CandyGuard(CandyGuard),
    }
    impl Account {
        #[doc = r" Try to create an account based on the given bytes."]
        #[doc = r""]
        #[doc = r" This method returns an error if the discriminator of the given bytes don't match"]
        #[doc = r" with any of the existing accounts, or if the deserialization fails."]
        pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
            Self::try_from(bytes)
        }
    }
    impl TryFrom<&[u8]> for Account {
        type Error = anchor_lang::error::Error;
        fn try_from(value: &[u8]) -> Result<Self> {
            if value.starts_with(FreezeEscrow::DISCRIMINATOR) {
                return FreezeEscrow::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::FreezeEscrow)
                    .map_err(Into::into);
            }
            if value.starts_with(CandyGuard::DISCRIMINATOR) {
                return CandyGuard::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::CandyGuard)
                    .map_err(Into::into);
            }
            Err(ProgramError::InvalidArgument.into())
        }
    }
    #[doc = r" An enum that includes all events of the declared program as a tuple variant."]
    #[doc = r""]
    #[doc = r" See [`Self::try_from_bytes`] to create an instance from bytes."]
    pub enum Event {}

    impl Event {
        #[doc = r" Try to create an event based on the given bytes."]
        #[doc = r""]
        #[doc = r" This method returns an error if the discriminator of the given bytes don't match"]
        #[doc = r" with any of the existing events, or if the deserialization fails."]
        pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
            Self::try_from(bytes)
        }
    }
    impl TryFrom<&[u8]> for Event {
        type Error = anchor_lang::error::Error;
        fn try_from(value: &[u8]) -> Result<Self> {
            Err(ProgramError::InvalidArgument.into())
        }
    }
}

// Empty value used for string padding.
const NULL_STRING: &str = "\0";

/// Return a padded string up to the specified length. If the specified
/// string `value` is longer than the allowed `length`, return an error.
pub fn fixed_length_string(value: String, length: usize) -> Result<String> {
    if length < value.len() {
        // the value is larger than the allowed length
        return err!(crate::errors::CandyGuardError::ExceededLength);
    }

    let padding = NULL_STRING.repeat(length - value.len());
    Ok(value + &padding)
}

/// Maximum group label size.
pub const MAX_LABEL_SIZE: usize = 6;

impl CandyGuardData {
    /// Serialize the candy guard data into the specified data array.
    pub fn save(&self, data: &mut [u8]) -> Result<()> {
        let mut cursor = 0;

        // saves the 'default' guard set
        let _ = self.default.to_data(data)?;
        cursor += self.default.size();

        // stores the number of 'groups' guard set
        let group_counter = if let Some(groups) = &self.groups {
            groups.len() as u32
        } else {
            0
        };
        data[cursor..cursor + 4].copy_from_slice(&u32::to_le_bytes(group_counter));
        cursor += 4;

        // saves each individual 'groups' guard set
        if let Some(groups) = &self.groups {
            for group in groups {
                // label
                let label = fixed_length_string(group.label.to_string(), MAX_LABEL_SIZE)?;
                data[cursor..cursor + MAX_LABEL_SIZE].copy_from_slice(label.as_bytes());
                cursor += MAX_LABEL_SIZE;
                // guard set
                let _ = group.guards.to_data(&mut data[cursor..])?;
                cursor += group.guards.size();
            }
        }

        Ok(())
    }
}

pub mod errors {
    use anchor_lang::error_code;

    #[error_code]
    pub enum CandyGuardError {
        #[msg("Could not save guard to account")]
        InvalidAccountSize,
        #[msg("Could not deserialize guard")]
        DeserializationError,
        #[msg("Public key mismatch")]
        PublicKeyMismatch,
        #[msg("Exceeded account increase limit")]
        DataIncrementLimitExceeded,
        #[msg("Account does not have correct owner")]
        IncorrectOwner,
        #[msg("Account is not initialized")]
        Uninitialized,
        #[msg("Missing expected remaining account")]
        MissingRemainingAccount,
        #[msg("Numerical overflow error")]
        NumericalOverflowError,
        #[msg("Missing required group label")]
        RequiredGroupLabelNotFound,
        #[msg("Group not found")]
        GroupNotFound,
        #[msg("Value exceeded maximum length")]
        ExceededLength,
        #[msg("Candy machine is empty")]
        CandyMachineEmpty,
        #[msg("No instruction was found")]
        InstructionNotFound,
        #[msg("Collection public key mismatch")]
        CollectionKeyMismatch,
        #[msg("Missing collection accounts")]
        MissingCollectionAccounts,
        #[msg("Collection update authority public key mismatch")]
        CollectionUpdateAuthorityKeyMismatch,
        #[msg("Mint must be the last instructions of the transaction")]
        MintNotLastTransaction,
        #[msg("Mint is not live")]
        MintNotLive,
        #[msg("Not enough SOL to pay for the mint")]
        NotEnoughSOL,
        #[msg("Token burn failed")]
        TokenBurnFailed,
        #[msg("Not enough tokens on the account")]
        NotEnoughTokens,
        #[msg("Token transfer failed")]
        TokenTransferFailed,
        #[msg("A signature was required but not found")]
        MissingRequiredSignature,
        #[msg("Gateway token is not valid")]
        GatewayTokenInvalid,
        #[msg("Current time is after the set end date")]
        AfterEndDate,
        #[msg("Current time is not within the allowed mint time")]
        InvalidMintTime,
        #[msg("Address not found on the allowed list")]
        AddressNotFoundInAllowedList,
        #[msg("Missing allowed list proof")]
        MissingAllowedListProof,
        #[msg("Allow list guard is not enabled")]
        AllowedListNotEnabled,
        #[msg("The maximum number of allowed mints was reached")]
        AllowedMintLimitReached,
        #[msg("Invalid NFT collection")]
        InvalidNftCollection,
        #[msg("Missing NFT on the account")]
        MissingNft,
        #[msg("Current redemeed items is at the set maximum amount")]
        MaximumRedeemedAmount,
        #[msg("Address not authorized")]
        AddressNotAuthorized,
        #[msg("Missing freeze instruction data")]
        MissingFreezeInstruction,
        #[msg("Freeze guard must be enabled")]
        FreezeGuardNotEnabled,
        #[msg("Freeze must be initialized")]
        FreezeNotInitialized,
        #[msg("Missing freeze period")]
        MissingFreezePeriod,
        #[msg("The freeze escrow account already exists")]
        FreezeEscrowAlreadyExists,
        #[msg("Maximum freeze period exceeded")]
        ExceededMaximumFreezePeriod,
        #[msg("Thaw is not enabled")]
        ThawNotEnabled,
        #[msg("Unlock is not enabled (not all NFTs are thawed)")]
        UnlockNotEnabled,
        #[msg("Duplicated group label")]
        DuplicatedGroupLabel,
        #[msg("Duplicated mint limit id")]
        DuplicatedMintLimitId,
        #[msg("An unauthorized program was found in the transaction")]
        UnauthorizedProgramFound,
        #[msg("Exceeded the maximum number of programs in the additional list")]
        ExceededProgramListSize,
        #[msg("Allocation PDA not initialized")]
        AllocationNotInitialized,
        #[msg("Allocation limit was reached")]
        AllocationLimitReached,
        #[msg("Allocation guard must be enabled")]
        AllocationGuardNotEnabled,
        #[msg("Candy machine has an invalid mint authority")]
        InvalidMintAuthority,
        #[msg("Instruction could not be created")]
        InstructionBuilderFailed,
        #[msg("Invalid account version")]
        InvalidAccountVersion,
    }
}
