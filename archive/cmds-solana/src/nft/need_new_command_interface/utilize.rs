use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Utilize {
    pub mint_account: Option<NodeId>,
    pub use_authority: Option<NodeId>, // keypair
    pub fee_payer: Option<NodeId>,     // keypair
    pub account: Option<Option<NodeId>>,
    pub owner: Option<NodeId>,
    pub burner: Option<NodeId>,
    pub number_of_uses: Option<u64>,
}

impl Utilize {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
            },
        };

        let use_authority = match self.use_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("use_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("use_authority".to_string())),
            },
        };

        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let account = match self.account {
            Some(s) => match s {
                Some(account) => Some(ctx.get_pubkey_by_id(account).await?),
                None => None,
            },
            None => match inputs.remove("account") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(account) => Some(ctx.get_pubkey_by_id(account).await?),
                    None => None,
                },
                Some(Value::Keypair(k)) => Some(Keypair::from(k).pubkey()),
                Some(Value::Pubkey(p)) => Some(p.into()),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("account".to_string())),
            },
        };

        let owner = match self.owner {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("owner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("owner".to_string())),
            },
        };

        let burner = match self.burner {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("burner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("burner".to_string())),
            },
        };

        let number_of_uses = match self.number_of_uses {
            Some(s) => s,
            None => match inputs.remove("number_of_uses") {
                Some(Value::U64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("number_of_uses".to_string())),
            },
        };

        let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&mint_account);

        let account = account.unwrap_or_else(|| {
            spl_associated_token_account::get_associated_token_address(&owner, &mint_account)
        });

        let (minimum_balance_for_rent_exemption, instructions) = command_utilize(
            metadata_account,
            account,
            mint_account,
            use_authority.pubkey(),
            owner,
            Some(burner),
            number_of_uses,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&use_authority, &fee_payer];

        let res = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        );

        let signature = res?;

        let outputs = hashmap! {
            "signature".to_owned()=>Value::Success(signature),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "mint_account".to_owned()=> Value::Pubkey(mint_account.into()),
            "use_authority".to_owned() => Value::Keypair(use_authority.into()),
            "owner".to_owned() => Value::Pubkey(owner.into()),
            "account".to_owned() => Value::Pubkey(account.into()),
            "burner".to_owned() => Value::Pubkey(burner.into()),
        };

        Ok(outputs)
    }
}

pub fn command_utilize(
    metadata_pubkey: Pubkey,
    token_account: Pubkey,
    mint: Pubkey,
    use_authority: Pubkey,
    owner: Pubkey,
    burner: Option<Pubkey>,
    number_of_uses: u64,
) -> CommandResult {
    let instructions = vec![mpl_token_metadata::instruction::utilize(
        mpl_token_metadata::id(),
        metadata_pubkey,
        token_account,
        mint,
        None,
        use_authority,
        owner,
        burner,
        number_of_uses,
    )];

    Ok((0, instructions))
}
