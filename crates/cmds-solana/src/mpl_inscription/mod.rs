use borsh::{BorshDeserialize, BorshSerialize};
use flow_lib::solana::Pubkey;
use solana_program::pubkey;

pub mod create_shard;
pub mod initialize;

pub const INSCRIPTION_PROGRAM_ID: Pubkey = pubkey!("1NSCRfGeyo7wPUazGbaPBUsTM49e1k2aXewHGARfzSo");

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq, PartialOrd, Hash)]
pub enum Key {
    Uninitialized,
    InscriptionMetadataAccount,
    MintInscriptionMetadataAccount,
    InscriptionShardAccount,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct InscriptionShard {
    pub key: Key,
    pub bump: u8,
    pub shard_number: u8,
    pub count: u64,
}

impl<'a> TryFrom<&solana_program::account_info::AccountInfo<'a>> for InscriptionShard {
    type Error = std::io::Error;

    fn try_from(
        account_info: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<Self, Self::Error> {
        let mut data: &[u8] = &(*account_info.data).borrow();
        Self::deserialize(&mut data)
    }
}
