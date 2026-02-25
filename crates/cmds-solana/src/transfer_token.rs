use crate::get_decimals;
use crate::{prelude::*, utils::ui_amount_to_amount};
use solana_program::program_pack::Pack;
use solana_sdk_ids::system_program;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_associated_token_account_interface::instruction;
use spl_token_interface::instruction::transfer_checked;
use tracing::info;

const SOLANA_TRANSFER_TOKEN: &str = "transfer_token";

const DEFINITION: &str = flow_lib::node_definition!("system_program/transfer_token.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_TRANSFER_TOKEN)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(SOLANA_TRANSFER_TOKEN, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(default)]
    pub memo: String,
    #[serde(with = "value::decimal")]
    pub amount: Decimal,
    pub decimals: Option<u8>,
    #[serde(with = "value::pubkey")]
    pub recipient: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub sender_token_account: Option<Pubkey>,
    pub sender_owner: Wallet,
    #[serde(default = "value::default::bool_true")]
    pub allow_unfunded: bool,
    #[serde(default = "value::default::bool_true")]
    pub fund_recipient: bool,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let client = ctx.solana_client();
    let fee_payer = input.fee_payer.pubkey();
    let sender_owner = input.sender_owner.pubkey();
    let commitment = client.commitment();

    let sender_token_acc = input
        .sender_token_account
        .unwrap_or_else(|| get_associated_token_address(&sender_owner, &input.mint_account));

    let decimals = match input.decimals {
        Some(d) => d,
        None => get_decimals(client, input.mint_account).await?,
    };

    let sender_token_amount = client
        .get_token_account_balance_with_commitment(&sender_token_acc, commitment)
        .await?
        .value;
    info!("sender_token_amount: {:?}", sender_token_amount);

    let sender_balance = sender_token_amount
        .amount
        .parse::<u64>()
        .map_err(crate::Error::custom)?;

    let transfer_balance = ui_amount_to_amount(input.amount, decimals)?;
    if transfer_balance > sender_balance {
        return Err(crate::Error::InsufficientSolanaBalance {
            needed: transfer_balance,
            balance: sender_balance,
        }
        .into());
    }

    let recipient_is_token_account = client
        .get_account_with_commitment(&input.recipient, commitment)
        .await?
        .value
        .map(|account| {
            spl_token_interface::check_id(&account.owner)
                && account.data.len() == spl_token_interface::state::Account::LEN
        });

    if recipient_is_token_account.is_none() && !input.allow_unfunded {
        return Err(crate::Error::RecipientAddressNotFunded.into());
    }
    let recipient_is_token_account = recipient_is_token_account.unwrap_or(false);
    info!("recipient_is_token_account: {:?}", recipient_is_token_account);

    let mut instructions = vec![];

    let recipient_token_account = if recipient_is_token_account {
        input.recipient
    } else {
        let ata = get_associated_token_address(&input.recipient, &input.mint_account);

        let needs_funding = match client
            .get_account_with_commitment(&ata, commitment)
            .await?
            .value
        {
            Some(account) if account.owner == system_program::ID => true,
            Some(account) if account.owner == spl_token_interface::ID => false,
            Some(_) => {
                return Err(crate::Error::UnsupportedRecipientAddress(
                    input.recipient.to_string(),
                )
                .into());
            }
            None => true,
        };

        if needs_funding {
            if !input.fund_recipient {
                return Err(crate::Error::AssociatedTokenAccountDoesntExist.into());
            }
            instructions.push(instruction::create_associated_token_account(
                &fee_payer,
                &input.recipient,
                &input.mint_account,
                &spl_token_interface::ID,
            ));
        }

        ata
    };

    instructions.push(transfer_checked(
        &spl_token_interface::ID,
        &sender_token_acc,
        &input.mint_account,
        &recipient_token_account,
        &sender_owner,
        &[],
        transfer_balance,
        decimals,
    )?);

    if !input.memo.is_empty() {
        instructions.push(spl_memo_interface::instruction::build_memo(
            &spl_memo_interface::v3::ID,
            input.memo.as_bytes(),
            &[&fee_payer],
        ));
    }

    let ins = Instructions {
        lookup_tables: None,
        fee_payer,
        signers: [input.fee_payer, input.sender_owner].into(),
        instructions,
    };

    let instructions = if input.submit { ins } else { Default::default() };

    let signature = ctx
        .execute(
            instructions,
            value::map! {
                "recipient_token_account" => recipient_token_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
