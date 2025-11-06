#![allow(warnings)]

pub mod remainder_vec;
mod generated;
mod hooked;
mod indexable_asset;

pub use generated::programs::MPL_CORE_ID as ID;
pub use generated::*;
pub use hooked::*;
pub use indexable_asset::*;

impl Copy for generated::types::Key {}

pub trait TryToVec {
    fn try_to_vec(&self) -> Result<Vec<u8>, borsh::io::Error>;
}

impl<T> TryToVec for T
where
    T: borsh::BorshSerialize,
{
    fn try_to_vec(&self) -> Result<Vec<u8>, borsh::io::Error> {
        borsh::to_vec(self)
    }
}
