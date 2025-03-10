use anchor_lang::prelude:: * ;
use accounts:: * ;
use events:: * ;
use types:: * ;
#[doc = "Program ID of program `spl_noop`."]
pub static ID:Pubkey = __ID;
#[doc = r" Const version of `ID`"]
pub const ID_CONST:Pubkey = __ID_CONST;
#[doc = r" The name is intentionally prefixed with `__` in order to reduce to possibility of name"]
#[doc = r" clashes with the crate's `ID`."]
static __ID:Pubkey = Pubkey::new_from_array([11u8,188u8,15u8,192u8,187u8,71u8,202u8,47u8,116u8,196u8,17u8,46u8,148u8,171u8,19u8,207u8,163u8,198u8,52u8,229u8,220u8,23u8,234u8,203u8,3u8,205u8,26u8,35u8,205u8,126u8,120u8,124u8,]);
const __ID_CONST:Pubkey = Pubkey::new_from_array([11u8,188u8,15u8,192u8,187u8,71u8,202u8,47u8,116u8,196u8,17u8,46u8,148u8,171u8,19u8,207u8,163u8,198u8,52u8,229u8,220u8,23u8,234u8,203u8,3u8,205u8,26u8,35u8,205u8,126u8,120u8,124u8,]);
#[doc = r" Program definition."]
pub mod program {
    use super:: * ;
    #[doc = r" Program type"]
    #[derive(Clone)]
    pub struct SplNoop;
    
    impl anchor_lang::Id for SplNoop {
        fn id() -> Pubkey {
            super::__ID
        }
    
        }

    }#[doc = r" Program constants."]
pub mod constants{}
#[doc = r" Program account type definitions."]
pub mod accounts {
    use super:: * ;
}#[doc = r" Program event type definitions."]
pub mod events {
    use super:: * ;
}#[doc = r" Program type definitions."]
#[doc = r""]
#[doc = r" Note that account and event type definitions are not included in this module, as they"]
#[doc = r" have their own dedicated modules."]
pub mod types {
    use super:: * ;
}#[doc = r" Cross program invocation (CPI) helpers."]
pub mod cpi {
    use super:: * ;
    pub fn noop_instruction<'a,'b,'c,'info>(ctx:anchor_lang::context::CpiContext<'a,'b,'c,'info,accounts::NoopInstruction> ,data:Vec<u8>) -> anchor_lang::Result<()>{
        let ix = {
            let ix = internal::args::NoopInstruction {
                data
            };
            let mut data = Vec::with_capacity(256);
            data.extend_from_slice(internal::args::NoopInstruction::DISCRIMINATOR);
            AnchorSerialize::serialize(&ix, &mut data).map_err(|_|anchor_lang::error::ErrorCode::InstructionDidNotSerialize)? ;
            let accounts = ctx.to_account_metas(None);
            anchor_lang::solana_program::instruction::Instruction {
                program_id:ctx.program.key(),accounts,data,
            }
        };
        let mut acc_infos = ctx.to_account_infos();
        anchor_lang::solana_program::program::invoke_signed(&ix, &acc_infos,ctx.signer_seeds,).map_or_else(|e|Err(Into::into(e)), |_|{
            Ok(())
        })
    }
    pub struct Return<T>{
        phantom:std::marker::PhantomData<T>
    }
    impl <T:AnchorDeserialize>Return<T>{
        pub fn get(&self) -> T {
            let(_key,data) = anchor_lang::solana_program::program::get_return_data().unwrap();
            T::try_from_slice(&data).unwrap()
        }
    
        }
    pub mod accounts {
        pub use super::internal::__cpi_client_accounts_noop_instruction:: * ;
    }
}#[doc = r" Off-chain client helpers."]
pub mod client {
    use super:: * ;
    #[doc = r" Client args."]
    pub mod args {
        pub use super::internal::args:: * ;
    }pub mod accounts {
        pub use super::internal::__client_accounts_noop_instruction:: * ;
    }
}#[doc(hidden)]
mod internal {
    use super:: * ;
    #[doc = r" An Anchor generated module containing the program's set of instructions, where each"]
    #[doc = r" method handler in the `#[program]` mod is associated with a struct defining the input"]
    #[doc = r" arguments to the method. These should be used directly, when one wants to serialize"]
    #[doc = r" Anchor instruction data, for example, when specifying instructions instructions on a"]
    #[doc = r" client."]
    pub mod args {
        use super:: * ;
        #[doc = r" Instruction argument"]
        #[derive(AnchorSerialize,AnchorDeserialize)]
        pub struct NoopInstruction {
            pub data:Vec<u8>
        }
        impl anchor_lang::Discriminator for NoopInstruction {
            const DISCRIMINATOR: &'static[u8] =  &[112u8,96u8,44u8,14u8,37u8,186u8,6u8,189u8];
        }
        impl anchor_lang::InstructionData for NoopInstruction{}
        
        impl anchor_lang::Owner for NoopInstruction {
            fn owner() -> Pubkey {
                super::__ID
            }
        
            }
    
        }#[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a CPI struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is an"]
    #[doc = r" AccountInfo."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" [`cpi::accounts`] module (also generated), which re-exports this."]
    pub(crate)mod __cpi_client_accounts_noop_instruction {
        use super:: * ;
        #[doc = " Generated CPI struct of the accounts for [`NoopInstruction`]."]
        pub struct NoopInstruction{}
        
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for NoopInstruction {
            fn to_account_metas(&self,is_signer:Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta>{
                let mut account_metas = vec![];
                account_metas
            }
        
            }
        #[automatically_derived]
        impl <'info>anchor_lang::ToAccountInfos<'info>for NoopInstruction {
            fn to_account_infos(&self) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>{
                let mut account_infos = vec![];
                account_infos
            }
        
            }
    
        }#[doc = r" An internal, Anchor generated module. This is used (as an"]
    #[doc = r" implementation detail), to generate a struct for a given"]
    #[doc = r" `#[derive(Accounts)]` implementation, where each field is a Pubkey,"]
    #[doc = r" instead of an `AccountInfo`. This is useful for clients that want"]
    #[doc = r" to generate a list of accounts, without explicitly knowing the"]
    #[doc = r" order all the fields should be in."]
    #[doc = r""]
    #[doc = r" To access the struct in this module, one should use the sibling"]
    #[doc = r" `accounts` module (also generated), which re-exports this."]
    pub(crate)mod __client_accounts_noop_instruction {
        use super:: * ;
        use anchor_lang::prelude::borsh;
        #[doc = " Generated client accounts for [`NoopInstruction`]."]
        #[derive(anchor_lang::AnchorSerialize)]
        pub struct NoopInstruction{}
        
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for NoopInstruction {
            fn to_account_metas(&self,is_signer:Option<bool>) -> Vec<anchor_lang::solana_program::instruction::AccountMeta>{
                let mut account_metas = vec![];
                account_metas
            }
        
            }
    
        }
}#[doc = r" Program utilities."]
pub mod utils {
    use super:: * ;
    #[doc = r" An enum that includes all accounts of the declared program as a tuple variant."]
    #[doc = r""]
    #[doc = r" See [`Self::try_from_bytes`] to create an instance from bytes."]
    pub enum Account{}
    
    impl Account {
        #[doc = r" Try to create an account based on the given bytes."]
        #[doc = r""]
        #[doc = r" This method returns an error if the discriminator of the given bytes don't match"]
        #[doc = r" with any of the existing accounts, or if the deserialization fails."]
        pub fn try_from_bytes(bytes: &[u8]) -> Result<Self>{
            Self::try_from(bytes)
        }
    
        }
    impl TryFrom< &[u8]>for Account {
        type Error = anchor_lang::error::Error;
        fn try_from(value: &[u8]) -> Result<Self>{
            Err(ProgramError::InvalidArgument.into())
        }
    
        }
    #[doc = r" An enum that includes all events of the declared program as a tuple variant."]
    #[doc = r""]
    #[doc = r" See [`Self::try_from_bytes`] to create an instance from bytes."]
    pub enum Event{}
    
    impl Event {
        #[doc = r" Try to create an event based on the given bytes."]
        #[doc = r""]
        #[doc = r" This method returns an error if the discriminator of the given bytes don't match"]
        #[doc = r" with any of the existing events, or if the deserialization fails."]
        pub fn try_from_bytes(bytes: &[u8]) -> Result<Self>{
            Self::try_from(bytes)
        }
    
        }
    impl TryFrom< &[u8]>for Event {
        type Error = anchor_lang::error::Error;
        fn try_from(value: &[u8]) -> Result<Self>{
            Err(ProgramError::InvalidArgument.into())
        }
    
        }

    }
