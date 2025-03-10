use accounts::*;
use anchor_lang::prelude::*;
use events::*;
use types::*;
#[doc = "Program ID of program `bubblegum`."]
pub static ID: Pubkey = __ID;
#[doc = r" Const version of `ID`"]
pub const ID_CONST: Pubkey = __ID_CONST;
#[doc = r" The name is intentionally prefixed with `__` in order to reduce to possibility of name"]
#[doc = r" clashes with the crate's `ID`."]
static __ID: Pubkey = Pubkey::new_from_array([
    152u8, 139u8, 128u8, 235u8, 121u8, 53u8, 40u8, 105u8, 178u8, 36u8, 116u8, 95u8, 89u8, 221u8,
    191u8, 138u8, 38u8, 88u8, 202u8, 19u8, 220u8, 104u8, 129u8, 33u8, 38u8, 53u8, 28u8, 174u8, 7u8,
    193u8, 165u8, 165u8,
]);
const __ID_CONST: Pubkey = Pubkey::new_from_array([
    152u8, 139u8, 128u8, 235u8, 121u8, 53u8, 40u8, 105u8, 178u8, 36u8, 116u8, 95u8, 89u8, 221u8,
    191u8, 138u8, 38u8, 88u8, 202u8, 19u8, 220u8, 104u8, 129u8, 33u8, 38u8, 53u8, 28u8, 174u8, 7u8,
    193u8, 165u8, 165u8,
]);
#[doc = r" Program definition."]
pub mod program {
    use super::*;
    #[doc = r" Program type"]
    #[derive(Clone)]
    pub struct Bubblegum;

    impl anchor_lang::Id for Bubblegum {
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
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct TreeConfig {
        pub tree_creator: Pubkey,
        pub tree_delegate: Pubkey,
        pub total_mint_capacity: u64,
        pub num_minted: u64,
        pub is_public: bool,
        pub is_decompressible: DecompressibleState,
    }
    impl anchor_lang::AccountSerialize for TreeConfig {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
            if writer.write_all(TreeConfig::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    impl anchor_lang::AccountDeserialize for TreeConfig {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < TreeConfig::DISCRIMINATOR.len() {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..TreeConfig::DISCRIMINATOR.len()];
            if TreeConfig::DISCRIMINATOR != given_disc {
                return Err(anchor_lang::error!(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                )
                .with_account_name(stringify!(TreeConfig)));
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[TreeConfig::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    impl anchor_lang::Discriminator for TreeConfig {
        const DISCRIMINATOR: &'static [u8] = &[122u8, 245u8, 175u8, 248u8, 171u8, 34u8, 0u8, 207u8];
    }
    impl anchor_lang::Owner for TreeConfig {
        fn owner() -> Pubkey {
            super::__ID
        }
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Voucher {
        pub leaf_schema: LeafSchema,
        pub index: u32,
        pub merkle_tree: Pubkey,
    }
    impl anchor_lang::AccountSerialize for Voucher {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
            if writer.write_all(Voucher::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    impl anchor_lang::AccountDeserialize for Voucher {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < Voucher::DISCRIMINATOR.len() {
                return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
            }
            let given_disc = &buf[..Voucher::DISCRIMINATOR.len()];
            if Voucher::DISCRIMINATOR != given_disc {
                return Err(anchor_lang::error!(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                )
                .with_account_name(stringify!(Voucher)));
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[Voucher::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
        }
    }
    impl anchor_lang::Discriminator for Voucher {
        const DISCRIMINATOR: &'static [u8] =
            &[191u8, 204u8, 149u8, 234u8, 213u8, 165u8, 13u8, 65u8];
    }
    impl anchor_lang::Owner for Voucher {
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
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Creator {
        pub address: Pubkey,
        pub verified: bool,
        pub share: u8,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Uses {
        pub use_method: UseMethod,
        pub remaining: u64,
        pub total: u64,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct Collection {
        pub verified: bool,
        pub key: Pubkey,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct MetadataArgs {
        pub name: String,
        pub symbol: String,
        pub uri: String,
        pub seller_fee_basis_points: u16,
        pub primary_sale_happened: bool,
        pub is_mutable: bool,
        pub edition_nonce: Option<u8>,
        pub token_standard: Option<TokenStandard>,
        pub collection: Option<Collection>,
        pub uses: Option<Uses>,
        pub token_program_version: TokenProgramVersion,
        pub creators: Vec<Creator>,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct UpdateArgs {
        pub name: Option<String>,
        pub symbol: Option<String>,
        pub uri: Option<String>,
        pub creators: Option<Vec<Creator>>,
        pub seller_fee_basis_points: Option<u16>,
        pub primary_sale_happened: Option<bool>,
        pub is_mutable: Option<bool>,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum Version {
        V1,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum LeafSchema {
        V1 {
            id: Pubkey,
            owner: Pubkey,
            delegate: Pubkey,
            nonce: u64,
            data_hash: [u8; 32],
            creator_hash: [u8; 32],
        },
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum TokenProgramVersion {
        Original,
        Token2022,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum TokenStandard {
        NonFungible,
        FungibleAsset,
        Fungible,
        NonFungibleEdition,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum UseMethod {
        Burn,
        Multiple,
        Single,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum BubblegumEventType {
        Uninitialized,
        LeafSchemaEvent,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum DecompressibleState {
        Enabled,
        Disabled,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum InstructionName {
        Unknown,
        MintV1,
        Redeem,
        CancelRedeem,
        Transfer,
        Delegate,
        DecompressV1,
        Compress,
        Burn,
        CreateTree,
        VerifyCreator,
        UnverifyCreator,
        VerifyCollection,
        UnverifyCollection,
        SetAndVerifyCollection,
        MintToCollectionV1,
        SetDecompressibleState,
        UpdateMetadata,
    }
}
#[doc = r" Cross program invocation (CPI) helpers."]
pub mod cpi {
    use super::*;
    pub fn burn<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Burn<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Burn {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Burn::DISCRIMINATOR);
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
    pub fn cancel_redeem<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::CancelRedeem<'info>>,
        root: [u8; 32],
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::CancelRedeem { root };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::CancelRedeem::DISCRIMINATOR);
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
    pub fn compress<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Compress<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Compress;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Compress::DISCRIMINATOR);
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
    pub fn create_tree<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::CreateTree<'info>>,
        max_depth: u32,
        max_buffer_size: u32,
        public: Option<bool>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::CreateTree {
                max_depth,
                max_buffer_size,
                public,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::CreateTree::DISCRIMINATOR);
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
    pub fn decompress_v1<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::DecompressV1<'info>>,
        metadata: MetadataArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::DecompressV1 { metadata };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::DecompressV1::DISCRIMINATOR);
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
    pub fn delegate<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Delegate<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Delegate {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Delegate::DISCRIMINATOR);
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
    pub fn mint_to_collection_v1<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::MintToCollectionV1<'info>,
        >,
        metadata_args: MetadataArgs,
    ) -> anchor_lang::Result<Return<LeafSchema>> {
        let ix = {
            let ix = internal::args::MintToCollectionV1 { metadata_args };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::MintToCollectionV1::DISCRIMINATOR);
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
            .map_or_else(
                |e| Err(Into::into(e)),
                |_| {
                    Ok(Return::<LeafSchema> {
                        phantom: std::marker::PhantomData,
                    })
                },
            )
    }
    pub fn mint_v1<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::MintV1<'info>>,
        message: MetadataArgs,
    ) -> anchor_lang::Result<Return<LeafSchema>> {
        let ix = {
            let ix = internal::args::MintV1 { message };
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
            .map_or_else(
                |e| Err(Into::into(e)),
                |_| {
                    Ok(Return::<LeafSchema> {
                        phantom: std::marker::PhantomData,
                    })
                },
            )
    }
    pub fn redeem<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Redeem<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Redeem {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Redeem::DISCRIMINATOR);
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
    pub fn set_and_verify_collection<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::SetAndVerifyCollection<'info>,
        >,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        message: MetadataArgs,
        collection: Pubkey,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetAndVerifyCollection {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
                message,
                collection,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetAndVerifyCollection::DISCRIMINATOR);
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
    pub fn set_decompressable_state<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::SetDecompressableState<'info>,
        >,
        decompressable_state: DecompressibleState,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetDecompressableState {
                decompressable_state,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetDecompressableState::DISCRIMINATOR);
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
    pub fn set_decompressible_state<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::SetDecompressibleState<'info>,
        >,
        decompressable_state: DecompressibleState,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetDecompressibleState {
                decompressable_state,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetDecompressibleState::DISCRIMINATOR);
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
    pub fn set_tree_delegate<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::SetTreeDelegate<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::SetTreeDelegate;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::SetTreeDelegate::DISCRIMINATOR);
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
    pub fn transfer<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Transfer<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Transfer {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Transfer::DISCRIMINATOR);
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
    pub fn unverify_collection<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::UnverifyCollection<'info>,
        >,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        message: MetadataArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::UnverifyCollection {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
                message,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::UnverifyCollection::DISCRIMINATOR);
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
    pub fn unverify_creator<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::UnverifyCreator<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        message: MetadataArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::UnverifyCreator {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
                message,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::UnverifyCreator::DISCRIMINATOR);
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
    pub fn verify_collection<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::VerifyCollection<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        message: MetadataArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::VerifyCollection {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
                message,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::VerifyCollection::DISCRIMINATOR);
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
    pub fn verify_creator<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::VerifyCreator<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
        message: MetadataArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::VerifyCreator {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
                message,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::VerifyCreator::DISCRIMINATOR);
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
    pub fn update_metadata<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::UpdateMetadata<'info>>,
        root: [u8; 32],
        nonce: u64,
        index: u32,
        current_metadata: MetadataArgs,
        update_args: UpdateArgs,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::UpdateMetadata {
                root,
                nonce,
                index,
                current_metadata,
                update_args,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::UpdateMetadata::DISCRIMINATOR);
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
        pub use super::internal::__cpi_client_accounts_burn::*;
        pub use super::internal::__cpi_client_accounts_cancel_redeem::*;
        pub use super::internal::__cpi_client_accounts_compress::*;
        pub use super::internal::__cpi_client_accounts_create_tree::*;
        pub use super::internal::__cpi_client_accounts_decompress_v1::*;
        pub use super::internal::__cpi_client_accounts_delegate::*;
        pub use super::internal::__cpi_client_accounts_mint_to_collection_v1::*;
        pub use super::internal::__cpi_client_accounts_mint_v1::*;
        pub use super::internal::__cpi_client_accounts_redeem::*;
        pub use super::internal::__cpi_client_accounts_set_and_verify_collection::*;
        pub use super::internal::__cpi_client_accounts_set_decompressable_state::*;
        pub use super::internal::__cpi_client_accounts_set_decompressible_state::*;
        pub use super::internal::__cpi_client_accounts_set_tree_delegate::*;
        pub use super::internal::__cpi_client_accounts_transfer::*;
        pub use super::internal::__cpi_client_accounts_unverify_collection::*;
        pub use super::internal::__cpi_client_accounts_unverify_creator::*;
        pub use super::internal::__cpi_client_accounts_update_metadata::*;
        pub use super::internal::__cpi_client_accounts_verify_collection::*;
        pub use super::internal::__cpi_client_accounts_verify_creator::*;
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
        pub use super::internal::__client_accounts_burn::*;
        pub use super::internal::__client_accounts_cancel_redeem::*;
        pub use super::internal::__client_accounts_compress::*;
        pub use super::internal::__client_accounts_create_tree::*;
        pub use super::internal::__client_accounts_decompress_v1::*;
        pub use super::internal::__client_accounts_delegate::*;
        pub use super::internal::__client_accounts_mint_to_collection_v1::*;
        pub use super::internal::__client_accounts_mint_v1::*;
        pub use super::internal::__client_accounts_redeem::*;
        pub use super::internal::__client_accounts_set_and_verify_collection::*;
        pub use super::internal::__client_accounts_set_decompressable_state::*;
        pub use super::internal::__client_accounts_set_decompressible_state::*;
        pub use super::internal::__client_accounts_set_tree_delegate::*;
        pub use super::internal::__client_accounts_transfer::*;
        pub use super::internal::__client_accounts_unverify_collection::*;
        pub use super::internal::__client_accounts_unverify_creator::*;
        pub use super::internal::__client_accounts_update_metadata::*;
        pub use super::internal::__client_accounts_verify_collection::*;
        pub use super::internal::__client_accounts_verify_creator::*;
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
        pub struct Burn {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
        }
        impl anchor_lang::Discriminator for Burn {
            const DISCRIMINATOR: &'static [u8] =
                &[116u8, 110u8, 29u8, 56u8, 107u8, 219u8, 42u8, 93u8];
        }
        impl anchor_lang::InstructionData for Burn {}

        impl anchor_lang::Owner for Burn {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct CancelRedeem {
            pub root: [u8; 32],
        }
        impl anchor_lang::Discriminator for CancelRedeem {
            const DISCRIMINATOR: &'static [u8] =
                &[111u8, 76u8, 232u8, 50u8, 39u8, 175u8, 48u8, 242u8];
        }
        impl anchor_lang::InstructionData for CancelRedeem {}

        impl anchor_lang::Owner for CancelRedeem {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Compress;

        impl anchor_lang::Discriminator for Compress {
            const DISCRIMINATOR: &'static [u8] =
                &[82u8, 193u8, 176u8, 117u8, 176u8, 21u8, 115u8, 253u8];
        }
        impl anchor_lang::InstructionData for Compress {}

        impl anchor_lang::Owner for Compress {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct CreateTree {
            pub max_depth: u32,
            pub max_buffer_size: u32,
            pub public: Option<bool>,
        }
        impl anchor_lang::Discriminator for CreateTree {
            const DISCRIMINATOR: &'static [u8] =
                &[165u8, 83u8, 136u8, 142u8, 89u8, 202u8, 47u8, 220u8];
        }
        impl anchor_lang::InstructionData for CreateTree {}

        impl anchor_lang::Owner for CreateTree {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct DecompressV1 {
            pub metadata: MetadataArgs,
        }
        impl anchor_lang::Discriminator for DecompressV1 {
            const DISCRIMINATOR: &'static [u8] =
                &[54u8, 85u8, 76u8, 70u8, 228u8, 250u8, 164u8, 81u8];
        }
        impl anchor_lang::InstructionData for DecompressV1 {}

        impl anchor_lang::Owner for DecompressV1 {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Delegate {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
        }
        impl anchor_lang::Discriminator for Delegate {
            const DISCRIMINATOR: &'static [u8] =
                &[90u8, 147u8, 75u8, 178u8, 85u8, 88u8, 4u8, 137u8];
        }
        impl anchor_lang::InstructionData for Delegate {}

        impl anchor_lang::Owner for Delegate {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct MintToCollectionV1 {
            pub metadata_args: MetadataArgs,
        }
        impl anchor_lang::Discriminator for MintToCollectionV1 {
            const DISCRIMINATOR: &'static [u8] =
                &[153u8, 18u8, 178u8, 47u8, 197u8, 158u8, 86u8, 15u8];
        }
        impl anchor_lang::InstructionData for MintToCollectionV1 {}

        impl anchor_lang::Owner for MintToCollectionV1 {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct MintV1 {
            pub message: MetadataArgs,
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
        pub struct Redeem {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
        }
        impl anchor_lang::Discriminator for Redeem {
            const DISCRIMINATOR: &'static [u8] =
                &[184u8, 12u8, 86u8, 149u8, 70u8, 196u8, 97u8, 225u8];
        }
        impl anchor_lang::InstructionData for Redeem {}

        impl anchor_lang::Owner for Redeem {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetAndVerifyCollection {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub message: MetadataArgs,
            pub collection: Pubkey,
        }
        impl anchor_lang::Discriminator for SetAndVerifyCollection {
            const DISCRIMINATOR: &'static [u8] =
                &[235u8, 242u8, 121u8, 216u8, 158u8, 234u8, 180u8, 234u8];
        }
        impl anchor_lang::InstructionData for SetAndVerifyCollection {}

        impl anchor_lang::Owner for SetAndVerifyCollection {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetDecompressableState {
            pub decompressable_state: DecompressibleState,
        }
        impl anchor_lang::Discriminator for SetDecompressableState {
            const DISCRIMINATOR: &'static [u8] =
                &[18u8, 135u8, 238u8, 168u8, 246u8, 195u8, 61u8, 115u8];
        }
        impl anchor_lang::InstructionData for SetDecompressableState {}

        impl anchor_lang::Owner for SetDecompressableState {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetDecompressibleState {
            pub decompressable_state: DecompressibleState,
        }
        impl anchor_lang::Discriminator for SetDecompressibleState {
            const DISCRIMINATOR: &'static [u8] =
                &[82u8, 104u8, 152u8, 6u8, 149u8, 111u8, 100u8, 13u8];
        }
        impl anchor_lang::InstructionData for SetDecompressibleState {}

        impl anchor_lang::Owner for SetDecompressibleState {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct SetTreeDelegate;

        impl anchor_lang::Discriminator for SetTreeDelegate {
            const DISCRIMINATOR: &'static [u8] =
                &[253u8, 118u8, 66u8, 37u8, 190u8, 49u8, 154u8, 102u8];
        }
        impl anchor_lang::InstructionData for SetTreeDelegate {}

        impl anchor_lang::Owner for SetTreeDelegate {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Transfer {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
        }
        impl anchor_lang::Discriminator for Transfer {
            const DISCRIMINATOR: &'static [u8] =
                &[163u8, 52u8, 200u8, 231u8, 140u8, 3u8, 69u8, 186u8];
        }
        impl anchor_lang::InstructionData for Transfer {}

        impl anchor_lang::Owner for Transfer {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct UnverifyCollection {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub message: MetadataArgs,
        }
        impl anchor_lang::Discriminator for UnverifyCollection {
            const DISCRIMINATOR: &'static [u8] =
                &[250u8, 251u8, 42u8, 106u8, 41u8, 137u8, 186u8, 168u8];
        }
        impl anchor_lang::InstructionData for UnverifyCollection {}

        impl anchor_lang::Owner for UnverifyCollection {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct UnverifyCreator {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub message: MetadataArgs,
        }
        impl anchor_lang::Discriminator for UnverifyCreator {
            const DISCRIMINATOR: &'static [u8] =
                &[107u8, 178u8, 57u8, 39u8, 105u8, 115u8, 112u8, 152u8];
        }
        impl anchor_lang::InstructionData for UnverifyCreator {}

        impl anchor_lang::Owner for UnverifyCreator {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct VerifyCollection {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub message: MetadataArgs,
        }
        impl anchor_lang::Discriminator for VerifyCollection {
            const DISCRIMINATOR: &'static [u8] =
                &[56u8, 113u8, 101u8, 253u8, 79u8, 55u8, 122u8, 169u8];
        }
        impl anchor_lang::InstructionData for VerifyCollection {}

        impl anchor_lang::Owner for VerifyCollection {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct VerifyCreator {
            pub root: [u8; 32],
            pub data_hash: [u8; 32],
            pub creator_hash: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub message: MetadataArgs,
        }
        impl anchor_lang::Discriminator for VerifyCreator {
            const DISCRIMINATOR: &'static [u8] = &[52u8, 17u8, 96u8, 132u8, 71u8, 4u8, 85u8, 194u8];
        }
        impl anchor_lang::InstructionData for VerifyCreator {}

        impl anchor_lang::Owner for VerifyCreator {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct UpdateMetadata {
            pub root: [u8; 32],
            pub nonce: u64,
            pub index: u32,
            pub current_metadata: MetadataArgs,
            pub update_args: UpdateArgs,
        }
        impl anchor_lang::Discriminator for UpdateMetadata {
            const DISCRIMINATOR: &'static [u8] =
                &[170u8, 182u8, 43u8, 239u8, 97u8, 78u8, 225u8, 186u8];
        }
        impl anchor_lang::InstructionData for UpdateMetadata {}

        impl anchor_lang::Owner for UpdateMetadata {
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
    pub(crate) mod __cpi_client_accounts_burn {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Burn`]."]
        pub struct Burn<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Burn<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Burn<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_cancel_redeem {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`CancelRedeem`]."]
        pub struct CancelRedeem<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub voucher: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CancelRedeem<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.leaf_owner),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.voucher),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for CancelRedeem<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.voucher));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_compress {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Compress`]."]
        pub struct Compress<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub master_edition: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Compress<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.merkle_tree),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.token_account),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.mint),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.metadata),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.master_edition),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Compress<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.mint));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_program,
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
    pub(crate) mod __cpi_client_accounts_create_tree {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`CreateTree`]."]
        pub struct CreateTree<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CreateTree<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_creator),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for CreateTree<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_creator,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_decompress_v1 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`DecompressV1`]."]
        pub struct DecompressV1<'info> {
            pub voucher: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub mint_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub master_edition: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub sysvar_rent: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub associated_token_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for DecompressV1<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.voucher),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.leaf_owner),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.token_account),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.mint),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.mint_authority),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.metadata),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.master_edition),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.sysvar_rent),
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
                        anchor_lang::Key::key(&self.associated_token_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for DecompressV1<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.voucher));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.mint));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.mint_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.master_edition,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.sysvar_rent,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_metadata_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.token_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.associated_token_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
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
    pub(crate) mod __cpi_client_accounts_delegate {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Delegate`]."]
        pub struct Delegate<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub previous_leaf_delegate:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Delegate<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.previous_leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Delegate<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.previous_leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_mint_to_collection_v1 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`MintToCollectionV1`]."]
        pub struct MintToCollectionV1<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record_pda:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub edition_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub bubblegum_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for MintToCollectionV1<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.tree_delegate),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority_record_pda),
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
                        anchor_lang::Key::key(&self.edition_account),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.bubblegum_signer),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for MintToCollectionV1<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.edition_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.bubblegum_signer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_mint_v1 {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`MintV1`]."]
        pub struct MintV1<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for MintV1<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.tree_delegate),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for MintV1<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_redeem {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Redeem`]."]
        pub struct Redeem<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub voucher: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Redeem<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.leaf_owner),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.voucher),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Redeem<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.voucher));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_set_and_verify_collection {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetAndVerifyCollection`]."]
        pub struct SetAndVerifyCollection<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record_pda:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub edition_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub bubblegum_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetAndVerifyCollection<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.tree_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority_record_pda),
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
                        anchor_lang::Key::key(&self.edition_account),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.bubblegum_signer),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetAndVerifyCollection<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.edition_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.bubblegum_signer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_set_decompressable_state {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetDecompressableState`]."]
        pub struct SetDecompressableState<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetDecompressableState<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_creator),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetDecompressableState<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_creator,
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
    pub(crate) mod __cpi_client_accounts_set_decompressible_state {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetDecompressibleState`]."]
        pub struct SetDecompressibleState<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetDecompressibleState<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_creator),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetDecompressibleState<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_creator,
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
    pub(crate) mod __cpi_client_accounts_set_tree_delegate {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`SetTreeDelegate`]."]
        pub struct SetTreeDelegate<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for SetTreeDelegate<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.tree_authority),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_creator),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_tree_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.merkle_tree),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for SetTreeDelegate<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_creator,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
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
    pub(crate) mod __cpi_client_accounts_transfer {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Transfer`]."]
        pub struct Transfer<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub new_leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Transfer<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.new_leaf_owner),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for Transfer<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.new_leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_unverify_collection {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`UnverifyCollection`]."]
        pub struct UnverifyCollection<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record_pda:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub edition_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub bubblegum_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UnverifyCollection<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.tree_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority_record_pda),
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
                        anchor_lang::Key::key(&self.edition_account),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.bubblegum_signer),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for UnverifyCollection<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.edition_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.bubblegum_signer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_unverify_creator {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`UnverifyCreator`]."]
        pub struct UnverifyCreator<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UnverifyCreator<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.creator),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for UnverifyCreator<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.creator));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_verify_collection {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`VerifyCollection`]."]
        pub struct VerifyCollection<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub tree_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_authority_record_pda:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_metadata: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub edition_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub bubblegum_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for VerifyCollection<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.tree_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.collection_authority_record_pda),
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
                        anchor_lang::Key::key(&self.edition_account),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.bubblegum_signer),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for VerifyCollection<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.edition_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.bubblegum_signer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_verify_creator {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`VerifyCreator`]."]
        pub struct VerifyCreator<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub creator: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for VerifyCreator<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.creator),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for VerifyCreator<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.creator));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    pub(crate) mod __cpi_client_accounts_update_metadata {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`UpdateMetadata`]."]
        pub struct UpdateMetadata<'info> {
            pub tree_authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub collection_mint:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_metadata:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub collection_authority_record_pda:
                Option<anchor_lang::solana_program::account_info::AccountInfo<'info>>,
            pub leaf_owner: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub leaf_delegate: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub log_wrapper: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub compression_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub token_metadata_program:
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UpdateMetadata<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.tree_authority),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                if let Some(collection_mint) = &self.collection_mint {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(collection_mint),
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
                if let Some(collection_metadata) = &self.collection_metadata {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(collection_metadata),
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
                if let Some(collection_authority_record_pda) = &self.collection_authority_record_pda
                {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(collection_authority_record_pda),
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
                        anchor_lang::Key::key(&self.leaf_owner),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.leaf_delegate),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.payer),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.log_wrapper),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.compression_program),
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
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateMetadata<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.tree_authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_mint,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_metadata,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.collection_authority_record_pda,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_owner,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.leaf_delegate,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.log_wrapper,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.compression_program,
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
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate) mod __client_accounts_burn {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Burn`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Burn {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Burn {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_cancel_redeem {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`CancelRedeem`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct CancelRedeem {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub merkle_tree: Pubkey,
            pub voucher: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CancelRedeem {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.leaf_owner,
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.voucher,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_compress {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Compress`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Compress {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub token_account: Pubkey,
            pub mint: Pubkey,
            pub metadata: Pubkey,
            pub master_edition: Pubkey,
            pub payer: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Compress {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.merkle_tree,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.token_account,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.mint, false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.metadata,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.master_edition,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_create_tree {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`CreateTree`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct CreateTree {
            pub tree_authority: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_creator: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CreateTree {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_creator,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_decompress_v1 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`DecompressV1`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct DecompressV1 {
            pub voucher: Pubkey,
            pub leaf_owner: Pubkey,
            pub token_account: Pubkey,
            pub mint: Pubkey,
            pub mint_authority: Pubkey,
            pub metadata: Pubkey,
            pub master_edition: Pubkey,
            pub system_program: Pubkey,
            pub sysvar_rent: Pubkey,
            pub token_metadata_program: Pubkey,
            pub token_program: Pubkey,
            pub associated_token_program: Pubkey,
            pub log_wrapper: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for DecompressV1 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.voucher,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.leaf_owner,
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.token_account,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.mint, false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.mint_authority,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.metadata,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.master_edition,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.sysvar_rent,
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
                        self.associated_token_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
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
    pub(crate) mod __client_accounts_delegate {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Delegate`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Delegate {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub previous_leaf_delegate: Pubkey,
            pub new_leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Delegate {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.previous_leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_mint_to_collection_v1 {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`MintToCollectionV1`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct MintToCollectionV1 {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_delegate: Pubkey,
            pub collection_authority: Pubkey,
            pub collection_authority_record_pda: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub edition_account: Pubkey,
            pub bubblegum_signer: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for MintToCollectionV1 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_delegate,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority_record_pda,
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
                        self.edition_account,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.bubblegum_signer,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_delegate: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for MintV1 {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_delegate,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_redeem {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Redeem`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Redeem {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub voucher: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Redeem {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.leaf_owner,
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.voucher,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_set_and_verify_collection {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetAndVerifyCollection`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetAndVerifyCollection {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_delegate: Pubkey,
            pub collection_authority: Pubkey,
            pub collection_authority_record_pda: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub edition_account: Pubkey,
            pub bubblegum_signer: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetAndVerifyCollection {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority_record_pda,
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
                        self.edition_account,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.bubblegum_signer,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_set_decompressable_state {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetDecompressableState`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetDecompressableState {
            pub tree_authority: Pubkey,
            pub tree_creator: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetDecompressableState {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_creator,
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
    pub(crate) mod __client_accounts_set_decompressible_state {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetDecompressibleState`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetDecompressibleState {
            pub tree_authority: Pubkey,
            pub tree_creator: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetDecompressibleState {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_creator,
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
    pub(crate) mod __client_accounts_set_tree_delegate {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`SetTreeDelegate`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct SetTreeDelegate {
            pub tree_authority: Pubkey,
            pub tree_creator: Pubkey,
            pub new_tree_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for SetTreeDelegate {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.tree_authority,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_creator,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_tree_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.merkle_tree,
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
    pub(crate) mod __client_accounts_transfer {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Transfer`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Transfer {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub new_leaf_owner: Pubkey,
            pub merkle_tree: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Transfer {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.new_leaf_owner,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_unverify_collection {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`UnverifyCollection`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct UnverifyCollection {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_delegate: Pubkey,
            pub collection_authority: Pubkey,
            pub collection_authority_record_pda: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub edition_account: Pubkey,
            pub bubblegum_signer: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for UnverifyCollection {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority_record_pda,
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
                        self.edition_account,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.bubblegum_signer,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_unverify_creator {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`UnverifyCreator`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct UnverifyCreator {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub creator: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for UnverifyCreator {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.creator,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_verify_collection {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`VerifyCollection`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct VerifyCollection {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub tree_delegate: Pubkey,
            pub collection_authority: Pubkey,
            pub collection_authority_record_pda: Pubkey,
            pub collection_mint: Pubkey,
            pub collection_metadata: Pubkey,
            pub edition_account: Pubkey,
            pub bubblegum_signer: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for VerifyCollection {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.collection_authority_record_pda,
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
                        self.edition_account,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.bubblegum_signer,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_verify_creator {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`VerifyCreator`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct VerifyCreator {
            pub tree_authority: Pubkey,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub merkle_tree: Pubkey,
            pub payer: Pubkey,
            pub creator: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for VerifyCreator {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.creator,
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
    pub(crate) mod __client_accounts_update_metadata {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`UpdateMetadata`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct UpdateMetadata {
            pub tree_authority: Pubkey,
            pub authority: Pubkey,
            pub collection_mint: Option<Pubkey>,
            pub collection_metadata: Option<Pubkey>,
            pub collection_authority_record_pda: Option<Pubkey>,
            pub leaf_owner: Pubkey,
            pub leaf_delegate: Pubkey,
            pub payer: Pubkey,
            pub merkle_tree: Pubkey,
            pub log_wrapper: Pubkey,
            pub compression_program: Pubkey,
            pub token_metadata_program: Pubkey,
            pub system_program: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for UpdateMetadata {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.tree_authority,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                if let Some(collection_mint) = &self.collection_mint {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *collection_mint,
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
                if let Some(collection_metadata) = &self.collection_metadata {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *collection_metadata,
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
                if let Some(collection_authority_record_pda) = &self.collection_authority_record_pda
                {
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            *collection_authority_record_pda,
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
                        self.leaf_owner,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.leaf_delegate,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.payer, true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.log_wrapper,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.compression_program,
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
                        self.system_program,
                        false,
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
        TreeConfig(TreeConfig),
        Voucher(Voucher),
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
            if value.starts_with(TreeConfig::DISCRIMINATOR) {
                return TreeConfig::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::TreeConfig)
                    .map_err(Into::into);
            }
            if value.starts_with(Voucher::DISCRIMINATOR) {
                return Voucher::try_deserialize_unchecked(&mut &value[..])
                    .map(Self::Voucher)
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
