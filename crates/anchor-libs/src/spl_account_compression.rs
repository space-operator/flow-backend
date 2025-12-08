use accounts::*;
use anchor_lang::prelude::*;
use events::*;
use types::*;
#[doc = "Program ID of program `spl_account_compression`."]
pub static ID: Pubkey = __ID;
#[doc = r" Const version of `ID`"]
pub const ID_CONST: Pubkey = __ID_CONST;
#[doc = r" The name is intentionally prefixed with `__` in order to reduce to possibility of name"]
#[doc = r" clashes with the crate's `ID`."]
static __ID: Pubkey = Pubkey::new_from_array([
    9u8, 42u8, 19u8, 238u8, 149u8, 196u8, 28u8, 186u8, 8u8, 166u8, 127u8, 90u8, 198u8, 126u8,
    141u8, 247u8, 225u8, 218u8, 17u8, 98u8, 94u8, 29u8, 100u8, 19u8, 127u8, 143u8, 79u8, 35u8,
    131u8, 3u8, 127u8, 20u8,
]);
const __ID_CONST: Pubkey = Pubkey::new_from_array([
    9u8, 42u8, 19u8, 238u8, 149u8, 196u8, 28u8, 186u8, 8u8, 166u8, 127u8, 90u8, 198u8, 126u8,
    141u8, 247u8, 225u8, 218u8, 17u8, 98u8, 94u8, 29u8, 100u8, 19u8, 127u8, 143u8, 79u8, 35u8,
    131u8, 3u8, 127u8, 20u8,
]);
#[doc = r" Program definition."]
pub mod program {
    use super::*;
    #[doc = r" Program type"]
    #[derive(Clone)]
    pub struct SplAccountCompression;

    impl anchor_lang::Id for SplAccountCompression {
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
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct ApplicationDataEventV1 {
        pub application_data: Vec<u8>,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct ChangeLogEventV1 {
        pub id: Pubkey,
        pub path: Vec<PathNode>,
        pub seq: u64,
        pub index: u32,
    }
    #[doc = " Initialization parameters for an SPL ConcurrentMerkleTree."]
    #[doc = ""]
    #[doc = " Only the following permutations are valid:"]
    #[doc = ""]
    #[doc = " | max_depth | max_buffer_size       |"]
    #[doc = " | --------- | --------------------- |"]
    #[doc = " | 14        | (64, 256, 1024, 2048) |"]
    #[doc = " | 20        | (64, 256, 1024, 2048) |"]
    #[doc = " | 24        | (64, 256, 512, 1024, 2048) |"]
    #[doc = " | 26        | (64, 256, 512, 1024, 2048) |"]
    #[doc = " | 30        | (512, 1024, 2048) |"]
    #[doc = ""]
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct ConcurrentMerkleTreeHeader {
        pub account_type: CompressionAccountType,
        pub header: ConcurrentMerkleTreeHeaderData,
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct ConcurrentMerkleTreeHeaderDataV1 {
        pub max_buffer_size: u32,
        pub max_depth: u32,
        pub authority: Pubkey,
        pub creation_slot: u64,
        pub padding: [u8; 6],
    }
    #[derive(Debug, Default, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub struct PathNode {
        pub node: [u8; 32],
        pub index: u32,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub enum ApplicationDataEvent {
        V1(ApplicationDataEventV1),
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub enum ChangeLogEvent {
        V1(ChangeLogEventV1),
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
    pub enum AccountCompressionEvent {
        ChangeLog(ChangeLogEvent),
        ApplicationData(ApplicationDataEvent),
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum CompressionAccountType {
        Uninitialized,
        ConcurrentMerkleTree,
    }
    #[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
    pub enum ConcurrentMerkleTreeHeaderData {
        V1(ConcurrentMerkleTreeHeaderDataV1),
    }
}
#[doc = r" Cross program invocation (CPI) helpers."]
pub mod cpi {
    use super::*;
    pub fn init_empty_merkle_tree<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::InitEmptyMerkleTree<'info>,
        >,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::InitEmptyMerkleTree {
                max_depth,
                max_buffer_size,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::InitEmptyMerkleTree::DISCRIMINATOR);
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
    pub fn replace_leaf<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::ReplaceLeaf<'info>>,
        root: [u8; 32],
        previous_leaf: [u8; 32],
        new_leaf: [u8; 32],
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::ReplaceLeaf {
                root,
                previous_leaf,
                new_leaf,
                index,
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::ReplaceLeaf::DISCRIMINATOR);
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
    pub fn transfer_authority<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<
            'a,
            'b,
            'c,
            'info,
            accounts::TransferAuthority<'info>,
        >,
        new_authority: Pubkey,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::TransferAuthority { new_authority };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::TransferAuthority::DISCRIMINATOR);
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
    pub fn verify_leaf<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::VerifyLeaf<'info>>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::VerifyLeaf { root, leaf, index };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::VerifyLeaf::DISCRIMINATOR);
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
    pub fn append<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::Append<'info>>,
        leaf: [u8; 32],
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::Append { leaf };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::Append::DISCRIMINATOR);
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
    pub fn insert_or_append<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::InsertOrAppend<'info>>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::InsertOrAppend { root, leaf, index };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::InsertOrAppend::DISCRIMINATOR);
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
    pub fn close_empty_tree<'a, 'b, 'c, 'info>(
        ctx: anchor_lang::context::CpiContext<'a, 'b, 'c, 'info, accounts::CloseEmptyTree<'info>>,
    ) -> anchor_lang::Result<()> {
        let ix = {
            let ix = internal::args::CloseEmptyTree;
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::CloseEmptyTree::DISCRIMINATOR);
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
        pub use super::internal::__cpi_client_accounts_append::*;
        pub use super::internal::__cpi_client_accounts_close_empty_tree::*;
        pub use super::internal::__cpi_client_accounts_init_empty_merkle_tree::*;
        pub use super::internal::__cpi_client_accounts_insert_or_append::*;
        pub use super::internal::__cpi_client_accounts_replace_leaf::*;
        pub use super::internal::__cpi_client_accounts_transfer_authority::*;
        pub use super::internal::__cpi_client_accounts_verify_leaf::*;
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
        pub use super::internal::__client_accounts_append::*;
        pub use super::internal::__client_accounts_close_empty_tree::*;
        pub use super::internal::__client_accounts_init_empty_merkle_tree::*;
        pub use super::internal::__client_accounts_insert_or_append::*;
        pub use super::internal::__client_accounts_replace_leaf::*;
        pub use super::internal::__client_accounts_transfer_authority::*;
        pub use super::internal::__client_accounts_verify_leaf::*;
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
        pub struct InitEmptyMerkleTree {
            pub max_depth: u32,
            pub max_buffer_size: u32,
        }
        impl anchor_lang::Discriminator for InitEmptyMerkleTree {
            const DISCRIMINATOR: &'static [u8] =
                &[191u8, 11u8, 119u8, 7u8, 180u8, 107u8, 220u8, 110u8];
        }
        impl anchor_lang::InstructionData for InitEmptyMerkleTree {}

        impl anchor_lang::Owner for InitEmptyMerkleTree {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct ReplaceLeaf {
            pub root: [u8; 32],
            pub previous_leaf: [u8; 32],
            pub new_leaf: [u8; 32],
            pub index: u32,
        }
        impl anchor_lang::Discriminator for ReplaceLeaf {
            const DISCRIMINATOR: &'static [u8] =
                &[204u8, 165u8, 76u8, 100u8, 73u8, 147u8, 0u8, 128u8];
        }
        impl anchor_lang::InstructionData for ReplaceLeaf {}

        impl anchor_lang::Owner for ReplaceLeaf {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct TransferAuthority {
            pub new_authority: Pubkey,
        }
        impl anchor_lang::Discriminator for TransferAuthority {
            const DISCRIMINATOR: &'static [u8] =
                &[48u8, 169u8, 76u8, 72u8, 229u8, 180u8, 55u8, 161u8];
        }
        impl anchor_lang::InstructionData for TransferAuthority {}

        impl anchor_lang::Owner for TransferAuthority {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct VerifyLeaf {
            pub root: [u8; 32],
            pub leaf: [u8; 32],
            pub index: u32,
        }
        impl anchor_lang::Discriminator for VerifyLeaf {
            const DISCRIMINATOR: &'static [u8] =
                &[124u8, 220u8, 22u8, 223u8, 104u8, 10u8, 250u8, 224u8];
        }
        impl anchor_lang::InstructionData for VerifyLeaf {}

        impl anchor_lang::Owner for VerifyLeaf {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct Append {
            pub leaf: [u8; 32],
        }
        impl anchor_lang::Discriminator for Append {
            const DISCRIMINATOR: &'static [u8] =
                &[149u8, 120u8, 18u8, 222u8, 236u8, 225u8, 88u8, 203u8];
        }
        impl anchor_lang::InstructionData for Append {}

        impl anchor_lang::Owner for Append {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct InsertOrAppend {
            pub root: [u8; 32],
            pub leaf: [u8; 32],
            pub index: u32,
        }
        impl anchor_lang::Discriminator for InsertOrAppend {
            const DISCRIMINATOR: &'static [u8] =
                &[6u8, 42u8, 50u8, 190u8, 51u8, 109u8, 178u8, 168u8];
        }
        impl anchor_lang::InstructionData for InsertOrAppend {}

        impl anchor_lang::Owner for InsertOrAppend {
            fn owner() -> Pubkey {
                super::__ID
            }
        }
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize, AnchorDeserialize)]
        pub struct CloseEmptyTree;

        impl anchor_lang::Discriminator for CloseEmptyTree {
            const DISCRIMINATOR: &'static [u8] =
                &[50u8, 14u8, 219u8, 107u8, 78u8, 103u8, 16u8, 103u8];
        }
        impl anchor_lang::InstructionData for CloseEmptyTree {}

        impl anchor_lang::Owner for CloseEmptyTree {
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
    pub(crate) mod __cpi_client_accounts_init_empty_merkle_tree {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`InitEmptyMerkleTree`]."]
        pub struct InitEmptyMerkleTree<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub noop: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitEmptyMerkleTree<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.noop),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InitEmptyMerkleTree<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.noop));
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
    pub(crate) mod __cpi_client_accounts_replace_leaf {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`ReplaceLeaf`]."]
        pub struct ReplaceLeaf<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub noop: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for ReplaceLeaf<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.noop),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for ReplaceLeaf<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.noop));
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
    pub(crate) mod __cpi_client_accounts_transfer_authority {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`TransferAuthority`]."]
        pub struct TransferAuthority<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for TransferAuthority<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
        impl<'info> anchor_lang::ToAccountInfos<'info> for TransferAuthority<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
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
    pub(crate) mod __cpi_client_accounts_verify_leaf {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`VerifyLeaf`]."]
        pub struct VerifyLeaf<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for VerifyLeaf<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.merkle_tree),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for VerifyLeaf<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
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
    pub(crate) mod __cpi_client_accounts_append {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`Append`]."]
        pub struct Append<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub noop: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for Append<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.noop),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for Append<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.noop));
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
    pub(crate) mod __cpi_client_accounts_insert_or_append {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`InsertOrAppend`]."]
        pub struct InsertOrAppend<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub noop: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InsertOrAppend<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
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
                        anchor_lang::Key::key(&self.noop),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for InsertOrAppend<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.noop));
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
    pub(crate) mod __cpi_client_accounts_close_empty_tree {
        use super::*;
        #[doc = " Generated CPI struct of the accounts for [`CloseEmptyTree`]."]
        pub struct CloseEmptyTree<'info> {
            pub merkle_tree: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub recipient: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CloseEmptyTree<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.merkle_tree),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.recipient),
                    false,
                ));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for CloseEmptyTree<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = vec![];
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.merkle_tree,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.recipient,
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
    pub(crate) mod __client_accounts_init_empty_merkle_tree {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`InitEmptyMerkleTree`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct InitEmptyMerkleTree {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
            pub noop: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for InitEmptyMerkleTree {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
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
                        self.noop, false,
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
    pub(crate) mod __client_accounts_replace_leaf {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`ReplaceLeaf`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct ReplaceLeaf {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
            pub noop: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for ReplaceLeaf {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
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
                        self.noop, false,
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
    pub(crate) mod __client_accounts_transfer_authority {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`TransferAuthority`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct TransferAuthority {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for TransferAuthority {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
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
    pub(crate) mod __client_accounts_verify_leaf {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`VerifyLeaf`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct VerifyLeaf {
            pub merkle_tree: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for VerifyLeaf {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.merkle_tree,
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
    pub(crate) mod __client_accounts_append {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`Append`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct Append {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
            pub noop: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for Append {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
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
                        self.noop, false,
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
    pub(crate) mod __client_accounts_insert_or_append {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`InsertOrAppend`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct InsertOrAppend {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
            pub noop: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for InsertOrAppend {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
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
                        self.noop, false,
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
    pub(crate) mod __client_accounts_close_empty_tree {
        use super::*;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`CloseEmptyTree`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct CloseEmptyTree {
            pub merkle_tree: Pubkey,
            pub authority: Pubkey,
            pub recipient: Pubkey,
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CloseEmptyTree {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = vec![];
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.merkle_tree,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.recipient,
                    false,
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
    pub enum Account {}

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
