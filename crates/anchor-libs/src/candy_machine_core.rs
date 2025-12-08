use accounts::*;
use anchor_lang::prelude::*;
use events::*;
use types::*;
#[doc = "Program ID of program `candy_machine_core`."]
pub static ID: Pubkey = __ID;
#[doc = r" Const version of `ID`"]
pub const ID_CONST: Pubkey = __ID_CONST;
#[doc = r" The name is intentionally prefixed with `__` in order to reduce to possibility of name"]
#[doc = r" clashes with the crate's `ID`."]
static __ID: Pubkey = Pubkey::new_from_array([
    175u8, 33u8, 127u8, 194u8, 214u8, 71u8, 225u8, 38u8, 220u8, 199u8, 29u8, 50u8, 234u8, 196u8,
    84u8, 239u8, 202u8, 73u8, 240u8, 29u8, 87u8, 162u8, 79u8, 17u8, 150u8, 153u8, 82u8, 222u8,
    172u8, 228u8, 173u8, 146u8,
]);
const __ID_CONST: Pubkey = Pubkey::new_from_array([
    175u8, 33u8, 127u8, 194u8, 214u8, 71u8, 225u8, 38u8, 220u8, 199u8, 29u8, 50u8, 234u8, 196u8,
    84u8, 239u8, 202u8, 73u8, 240u8, 29u8, 87u8, 162u8, 79u8, 17u8, 150u8, 153u8, 82u8, 222u8,
    172u8, 228u8, 173u8, 146u8,
]);
#[doc = r" Program definition."]
pub mod program {
    use super::*;
    #[doc = r" Program type"]
    #[derive(Clone)]
    pub struct CandyMachineCore;

    impl anchor_lang::Id for CandyMachineCore {
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
    #[doc = " Candy machine state and config data."]
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct CandyMachine {
        pub version: AccountVersion,
        pub token_standard: u8,
        pub features: [u8; 6],
        pub authority: Pubkey,
        pub mint_authority: Pubkey,
        pub collection_mint: Pubkey,
        pub items_redeemed: u64,
        pub data: CandyMachineData,
    }
    impl anchor_lang::AccountSerialize for CandyMachine {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
            if writer.write_all(CandyMachine::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    impl anchor_lang::AccountDeserialize for CandyMachine {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < CandyMachine::DISCRIMINATOR.len() {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..CandyMachine::DISCRIMINATOR.len()];
            if CandyMachine::DISCRIMINATOR != given_disc {
                return Err(anchor_lang::error!(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                )
                .with_account_name(stringify!(CandyMachine)));
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[CandyMachine::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    impl anchor_lang::Discriminator for CandyMachine {
        const DISCRIMINATOR: &'static [u8] =
            &[51u8, 173u8, 177u8, 113u8, 25u8, 241u8, 109u8, 189u8];
    }
    impl anchor_lang::Owner for CandyMachine {
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
    #[doc = " Candy machine configuration data."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct CandyMachineData {
        pub items_available: u64,
        pub symbol: String,
        pub seller_fee_basis_points: u16,
        pub max_supply: u64,
        pub is_mutable: bool,
        pub creators: Vec<Creator>,
        pub config_line_settings: Option<ConfigLineSettings>,
        pub hidden_settings: Option<HiddenSettings>,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Creator {
        pub address: Pubkey,
        pub verified: bool,
        pub percentage_share: u8,
    }
    #[doc = " Hidden settings for large mints used with off-chain data."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct HiddenSettings {
        pub name: String,
        pub uri: String,
        pub hash: [u8; 32],
    }
    #[doc = " Config line settings to allocate space for individual name + URI."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct ConfigLineSettings {
        pub prefix_name: String,
        pub name_length: u32,
        pub prefix_uri: String,
        pub uri_length: u32,
        pub is_sequential: bool,
    }
    #[doc = " Config line struct for storing asset (NFT) data pre-mint."]
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct ConfigLine {
        pub name: String,
        pub uri: String,
    }
    #[doc = " Account versioning."]
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum AccountVersion {
        V1,
        V2,
    }
}
#[doc = r" Cross program invocation (CPI) helpers."]
pub mod cpi {
    use super::*;
    pub fn add_config_lines<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::AddConfigLines<'info>>,
        index: u32,
        config_lines: Vec<ConfigLine>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::AddConfigLines {
                index,
                config_lines,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::AddConfigLines::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn initialize<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Initialize<'info>>,
        data: CandyMachineData,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Initialize { data };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Initialize::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn initialize_v2<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::InitializeV2<'info>>,
        data: CandyMachineData,
        token_standard: u8,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::InitializeV2 {
                data,
                token_standard,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::InitializeV2::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn mint<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Mint<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Mint;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Mint::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn mint_v2<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::MintV2<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::MintV2;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::MintV2::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
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
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn set_collection<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetCollection<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetCollection;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetCollection::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn set_collection_v2<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetCollectionV2<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetCollectionV2;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetCollectionV2::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn set_mint_authority<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetMintAuthority<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetMintAuthority;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetMintAuthority::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
                accounts,
                data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos, ctx.signer_seeds)
            .map_or_else(|e| Err(Into::into(e)), |_| Ok(()))
    }
    pub fn set_token_standard<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetTokenStandard<'info>>,
        token_standard: u8,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetTokenStandard { token_standard };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetTokenStandard::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
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
        data: CandyMachineData,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Update { data };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Update::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id: ctx.program_id,
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
                program_id: ctx.program_id,
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
        pub use super::internal::__cpi_client_accounts_add_config_lines::*;
        pub use super::internal::__cpi_client_accounts_initialize::*;
        pub use super::internal::__cpi_client_accounts_initialize_v2::*;
        pub use super::internal::__cpi_client_accounts_mint::*;
        pub use super::internal::__cpi_client_accounts_mint_v2::*;
        pub use super::internal::__cpi_client_accounts_set_authority::*;
        pub use super::internal::__cpi_client_accounts_set_collection::*;
        pub use super::internal::__cpi_client_accounts_set_collection_v2::*;
        pub use super::internal::__cpi_client_accounts_set_mint_authority::*;
        pub use super::internal::__cpi_client_accounts_set_token_standard::*;
        pub use super::internal::__cpi_client_accounts_update::*;
        pub use super::internal::__cpi_client_accounts_withdraw::*;
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
        pub use super::internal::__client_accounts_add_config_lines::*;
        pub use super::internal::__client_accounts_initialize::*;
        pub use super::internal::__client_accounts_initialize_v2::*;
        pub use super::internal::__client_accounts_mint::*;
        pub use super::internal::__client_accounts_mint_v2::*;
        pub use super::internal::__client_accounts_set_authority::*;
        pub use super::internal::__client_accounts_set_collection::*;
        pub use super::internal::__client_accounts_set_collection_v2::*;
        pub use super::internal::__client_accounts_set_mint_authority::*;
        pub use super::internal::__client_accounts_set_token_standard::*;
        pub use super::internal::__client_accounts_update::*;
        pub use super::internal::__client_accounts_withdraw::*;
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
        pub struct AddConfigLines {
            pub index: u32,
            pub config_lines: Vec<ConfigLine>,
        }
        impl anchor_lang::Discriminator for AddConfigLines {
            const DISCRIMINATOR: &'static [u8] =
                &[223u8, 50u8, 224u8, 227u8, 151u8, 8u8, 115u8, 106u8];
        }
        impl anchor_lang::InstructionData for AddConfigLines {}

        impl anchor_lang::Owner for AddConfigLines {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Initialize {
            pub data: CandyMachineData,
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
        pub struct InitializeV2 {
            pub data: CandyMachineData,
            pub token_standard: u8,
        }
        impl anchor_lang::Discriminator for InitializeV2 {
            const DISCRIMINATOR: &'static [u8] =
                &[67u8, 153u8, 175u8, 39u8, 218u8, 16u8, 38u8, 32u8];
        }
        impl anchor_lang::InstructionData for InitializeV2 {}

        impl anchor_lang::Owner for InitializeV2 {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Mint;

        impl anchor_lang::Discriminator for Mint {
            const DISCRIMINATOR: &'static [u8] =
                &[51u8, 57u8, 225u8, 47u8, 182u8, 146u8, 137u8, 166u8];
        }
        impl anchor_lang::InstructionData for Mint {}

        impl anchor_lang::Owner for Mint {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct MintV2;

        impl anchor_lang::Discriminator for MintV2 {
            const DISCRIMINATOR: &'static [u8] =
                &[120u8, 121u8, 23u8, 146u8, 173u8, 110u8, 199u8, 205u8];
        }
        impl anchor_lang::InstructionData for MintV2 {}

        impl anchor_lang::Owner for MintV2 {
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
        pub struct SetCollection;

        impl anchor_lang::Discriminator for SetCollection {
            const DISCRIMINATOR: &'static [u8] =
                &[192u8, 254u8, 206u8, 76u8, 168u8, 182u8, 59u8, 223u8];
        }
        impl anchor_lang::InstructionData for SetCollection {}

        impl anchor_lang::Owner for SetCollection {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetCollectionV2;

        impl anchor_lang::Discriminator for SetCollectionV2 {
            const DISCRIMINATOR: &'static [u8] =
                &[229u8, 35u8, 61u8, 91u8, 15u8, 14u8, 99u8, 160u8];
        }
        impl anchor_lang::InstructionData for SetCollectionV2 {}

        impl anchor_lang::Owner for SetCollectionV2 {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetMintAuthority;

        impl anchor_lang::Discriminator for SetMintAuthority {
            const DISCRIMINATOR: &'static [u8] =
                &[67u8, 127u8, 155u8, 187u8, 100u8, 174u8, 103u8, 121u8];
        }
        impl anchor_lang::InstructionData for SetMintAuthority {}

        impl anchor_lang::Owner for SetMintAuthority {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetTokenStandard {
            pub token_standard: u8,
        }
        impl anchor_lang::Discriminator for SetTokenStandard {
            const DISCRIMINATOR: &'static [u8] =
                &[147u8, 212u8, 106u8, 195u8, 30u8, 170u8, 209u8, 128u8];
        }
        impl anchor_lang::InstructionData for SetTokenStandard {}

        impl anchor_lang::Owner for SetTokenStandard {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Update {
            pub data: CandyMachineData,
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
    }
    #[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate) mod __cpi_client_accounts_add_config_lines {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`AddConfigLines`]."]
        pub struct AddConfigLines<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for AddConfigLines<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for AddConfigLines<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
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
    pub(crate) mod __cpi_client_accounts_initialize {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Initialize`]."]
        pub struct Initialize<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
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
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        false,
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
                        anchor_lang::Key::key(&self.collection_metadata),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_update_authority),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_authority_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
                        false,
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Initialize<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
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
    pub(crate) mod __cpi_client_accounts_initialize_v2 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`InitializeV2`]."]
        pub struct InitializeV2<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub rule_set: Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_delegate_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_instructions: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authorization_rules_program:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub authorization_rules:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitializeV2<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
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
                if let Some(rule_set) = &self.rule_set {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(rule_set),
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
                    anchor_lang::Key::key(&self.collection_metadata),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_update_authority),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_delegate_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules_program),
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules),
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
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeV2<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.rule_set,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_delegate_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.sysvar_instructions,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules,
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
    pub(crate) mod __cpi_client_accounts_mint {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Mint`]."]
        pub struct Mint<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_master_edition: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub recent_slothashes: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Mint<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.mint_authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_mint),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.nft_mint_authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_metadata),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_master_edition),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority_record),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_metadata),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_update_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_program),
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
                        anchor_lang::Key::key(&self.recent_slothashes),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Mint<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.mint_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_mint_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
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
    pub(crate) mod __cpi_client_accounts_mint_v2 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`MintV2`]."]
        pub struct MintV2<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub nft_master_edition: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token: Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub token_record: Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_delegate_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub spl_token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub spl_ata_program:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_instructions: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub recent_slothashes: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authorization_rules_program:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub authorization_rules:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for MintV2<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.mint_authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.nft_owner),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_mint),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.nft_mint_authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_metadata),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.nft_master_edition),
                    false,
                ));
                if let Some(token) = &self.token {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(token),
                        false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                if let Some(token_record) = &self.token_record {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(token_record),
                        false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_delegate_record),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_metadata),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_update_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.spl_token_program),
                        false,
                    ),
                );
                if let Some(spl_ata_program) = &self.spl_ata_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(spl_ata_program),
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules_program),
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules),
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
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for MintV2<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.mint_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_mint_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.nft_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.token));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_delegate_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.spl_token_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.spl_ata_program,
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
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules,
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
    pub(crate) mod __cpi_client_accounts_set_authority {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetAuthority`]."]
        pub struct SetAuthority<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
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
                    anchor_lang::Key::key(&self.candy_machine),
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
                    &self.candy_machine,
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
    pub(crate) mod __cpi_client_accounts_set_collection {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetCollection`]."]
        pub struct SetCollection<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_metadata:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_authority_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetCollection<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_metadata),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_authority_record),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.new_collection_update_authority),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_metadata),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_mint),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.new_collection_authority_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
                        false,
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetCollection<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_authority_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
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
    pub(crate) mod __cpi_client_accounts_set_collection_v2 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetCollectionV2`]."]
        pub struct SetCollectionV2<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_delegate_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_metadata:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_master_edition:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_collection_delegate_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_instructions: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authorization_rules_program:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub authorization_rules:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetCollectionV2<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_update_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_metadata),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_delegate_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_update_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_mint),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.new_collection_metadata),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_collection_master_edition),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.new_collection_delegate_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules_program),
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules),
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
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetCollectionV2<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_delegate_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_collection_delegate_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.sysvar_instructions,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules,
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
    pub(crate) mod __cpi_client_accounts_set_mint_authority {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetMintAuthority`]."]
        pub struct SetMintAuthority<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetMintAuthority<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
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
                        anchor_lang::Key::key(&self.mint_authority),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetMintAuthority<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.mint_authority,
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
    pub(crate) mod __cpi_client_accounts_set_token_standard {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetTokenStandard`]."]
        pub struct SetTokenStandard<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority_pda: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub rule_set: Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_delegate_record:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_update_authority:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_instructions: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authorization_rules_program:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub authorization_rules:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetTokenStandard<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.authority_pda),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                if let Some(rule_set) = &self.rule_set {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(rule_set),
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
                    anchor_lang::Key::key(&self.collection_delegate_record),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_mint),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.collection_metadata),
                    false,
                ));
                if let Some(collection_authority_record) = &self.collection_authority_record {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(collection_authority_record),
                        false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_update_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.token_metadata_program),
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules_program),
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(authorization_rules),
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
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetTokenStandard<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.rule_set,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_delegate_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_update_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.sysvar_instructions,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authorization_rules,
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
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Update<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.candy_machine),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Update<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.candy_machine,
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
    pub(crate) mod __cpi_client_accounts_withdraw {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Withdraw`]."]
        pub struct Withdraw<'info> {
            pub candy_machine: anchor_lang::solana_program::account_info::AccountInfo<'info>,
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
                    anchor_lang::Key::key(&self.candy_machine),
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
                    &self.candy_machine,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
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
    pub(crate) mod __client_accounts_add_config_lines {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`AddConfigLines`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct AddConfigLines {
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for AddConfigLines {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
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
    pub(crate) mod __client_accounts_initialize {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Initialize`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Initialize {
            pub candy_machine: Pubkey,
            pub authority_pda: Pubkey,
            pub authority: Pubkey,
            pub payer: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_master_edition: Pubkey,
            pub collection_update_authority: Pubkey,
            pub collection_authority_record: Pubkey,
            pub token_metadata_program: Pubkey,
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
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_metadata,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_update_authority,
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_authority_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
                        false,
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
    pub(crate) mod __client_accounts_initialize_v2 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`InitializeV2`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct InitializeV2 {
            pub candy_machine: Pubkey,
            pub authority_pda: Pubkey,
            pub authority: Pubkey,
            pub payer: Pubkey,
            pub rule_set: Option<Pubkey>,
            pub collection_metadata: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_master_edition: Pubkey,
            pub collection_update_authority: Pubkey,
            pub collection_delegate_record: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
            pub sysvar_instructions: Pubkey,
            pub authorization_rules_program: Option<Pubkey>,
            pub authorization_rules: Option<Pubkey>,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for InitializeV2 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                if let Some(rule_set) = &self.rule_set {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *rule_set, false,
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
                    self.collection_metadata,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_update_authority,
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_delegate_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules_program,
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules,
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
    pub(crate) mod __client_accounts_mint {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Mint`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Mint {
            pub candy_machine: Pubkey,
            pub authority_pda: Pubkey,
            pub mint_authority: Pubkey,
            pub payer: Pubkey,
            pub nft_mint: Pubkey,
            pub nft_mint_authority: Pubkey,
            pub nft_metadata: Pubkey,
            pub nft_master_edition: Pubkey,
            pub collection_authority_record: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_master_edition: Pubkey,
            pub collection_update_authority: Pubkey,
            pub token_metadata_program: Pubkey,
            pub token_program: Pubkey,
            pub system_program: Pubkey,
            pub recent_slothashes: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Mint {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.mint_authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_mint,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.nft_mint_authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_metadata,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_master_edition,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority_record,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_metadata,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_update_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_program,
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
    pub(crate) mod __client_accounts_mint_v2 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`MintV2`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct MintV2 {
            pub candy_machine: Pubkey,
            pub authority_pda: Pubkey,
            pub mint_authority: Pubkey,
            pub payer: Pubkey,
            pub nft_owner: Pubkey,
            pub nft_mint: Pubkey,
            pub nft_mint_authority: Pubkey,
            pub nft_metadata: Pubkey,
            pub nft_master_edition: Pubkey,
            pub token: Option<Pubkey>,
            pub token_record: Option<Pubkey>,
            pub collection_delegate_record: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_master_edition: Pubkey,
            pub collection_update_authority: Pubkey,
            pub token_metadata_program: Pubkey,
            pub spl_token_program: Pubkey,
            pub spl_ata_program: Option<Pubkey>,
            pub system_program: Pubkey,
            pub sysvar_instructions: Pubkey,
            pub recent_slothashes: Pubkey,
            pub authorization_rules_program: Option<Pubkey>,
            pub authorization_rules: Option<Pubkey>,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for MintV2 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.mint_authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.nft_owner,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_mint,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.nft_mint_authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_metadata,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.nft_master_edition,
                    false,
                ));
                if let Some(token) = &self.token {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        *token, false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                if let Some(token_record) = &self.token_record {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        *token_record,
                        false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_delegate_record,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_metadata,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_update_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.spl_token_program,
                        false,
                    ),
                );
                if let Some(spl_ata_program) = &self.spl_ata_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *spl_ata_program,
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules_program,
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules,
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
            pub candy_machine: Pubkey,
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
                    self.candy_machine,
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
    pub(crate) mod __client_accounts_set_collection {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetCollection`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetCollection {
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
            pub authority_pda: Pubkey,
            pub payer: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_authority_record: Pubkey,
            pub new_collection_update_authority: Pubkey,
            pub new_collection_metadata: Pubkey,
            pub new_collection_mint: Pubkey,
            pub new_collection_master_edition: Pubkey,
            pub new_collection_authority_record: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetCollection {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_metadata,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_authority_record,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.new_collection_update_authority,
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_metadata,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_mint,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.new_collection_authority_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
                        false,
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
    pub(crate) mod __client_accounts_set_collection_v2 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetCollectionV2`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetCollectionV2 {
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
            pub authority_pda: Pubkey,
            pub payer: Pubkey,
            pub collection_update_authority: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_delegate_record: Pubkey,
            pub new_collection_update_authority: Pubkey,
            pub new_collection_mint: Pubkey,
            pub new_collection_metadata: Pubkey,
            pub new_collection_master_edition: Pubkey,
            pub new_collection_delegate_record: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
            pub sysvar_instructions: Pubkey,
            pub authorization_rules_program: Option<Pubkey>,
            pub authorization_rules: Option<Pubkey>,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetCollectionV2 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_update_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_metadata,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_delegate_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_update_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_mint,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.new_collection_metadata,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_collection_master_edition,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.new_collection_delegate_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules_program,
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules,
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
    pub(crate) mod __client_accounts_set_mint_authority {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetMintAuthority`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetMintAuthority {
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
            pub mint_authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetMintAuthority {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
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
                        self.mint_authority,
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
    pub(crate) mod __client_accounts_set_token_standard {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetTokenStandard`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetTokenStandard {
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
            pub authority_pda: Pubkey,
            pub payer: Pubkey,
            pub rule_set: Option<Pubkey>,
            pub collection_delegate_record: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub collection_authority_record: Option<Pubkey>,
            pub collection_update_authority: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
            pub sysvar_instructions: Pubkey,
            pub authorization_rules_program: Option<Pubkey>,
            pub authorization_rules: Option<Pubkey>,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetTokenStandard {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.authority_pda,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                if let Some(rule_set) = &self.rule_set {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *rule_set, false,
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
                    self.collection_delegate_record,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_mint,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.collection_metadata,
                    false,
                ));
                if let Some(collection_authority_record) = &self.collection_authority_record {
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        *collection_authority_record,
                        false,
                    ));
                } else {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            super::__ID,
                            false,
                        ),
                    );
                }
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_update_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.token_metadata_program,
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
                if let Some(authorization_rules_program) = &self.authorization_rules_program {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules_program,
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
                if let Some(authorization_rules) = &self.authorization_rules {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *authorization_rules,
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
            pub candy_machine: Pubkey,
            pub authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Update {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.candy_machine,
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
    pub(crate) mod __client_accounts_withdraw {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Withdraw`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Withdraw {
            pub candy_machine: Pubkey,
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
                    self.candy_machine,
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
}
#[doc = r" Program utilities."]
pub mod utils {
    use super::*;
    #[doc = r" An enum that includes all accounts of the declared program as a tuple variant."]
    #[doc = r""]
    #[doc = r" See [`Self::try_from_bytes`] to create an instance from bytes."]
    pub enum Account {
        CandyMachine(CandyMachine),
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
            if value.starts_with(CandyMachine::DISCRIMINATOR) {
                return CandyMachine::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::CandyMachine)
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
