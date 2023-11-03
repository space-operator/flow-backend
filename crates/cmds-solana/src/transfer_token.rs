use crate::{prelude::*, utils::ui_amount_to_amount};
use solana_program::system_program;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_pack::Pack;
use spl_associated_token_account::instruction;
use spl_token::instruction::transfer_checked;

const SOLANA_TRANSFER_TOKEN: &str = "transfer_token";

const DEFINITION: &str = include_str!("../../../node-definitions/solana/transfer_token.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_TRANSFER_TOKEN)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(SOLANA_TRANSFER_TOKEN, |_| build()));

async fn get_decimals(client: &RpcClient, token_account: Pubkey) -> crate::Result<u8> {
    let source_account = client
        .get_token_account(&token_account)
        .await
        .map_err(|_| crate::Error::NotTokenAccount(token_account))?
        .ok_or(crate::Error::NotTokenAccount(token_account))?;
    Ok(source_account.token_amount.decimals)
}

// https://spl.solana.com/associated-token-account
// https://github.com/solana-labs/solana-program-library/blob/master/token/cli/src/main.rs#L555
#[allow(clippy::too_many_arguments)]
async fn command_transfer_token(
    client: &RpcClient,
    fee_payer: &Pubkey,
    token: Pubkey,
    ui_amount: Decimal,
    decimals: Option<u8>,
    recipient: Pubkey,
    sender: Option<Pubkey>,
    sender_owner: Pubkey,
    allow_unfunded_recipient: bool,
    fund_recipient: bool,
    memo: String,
) -> crate::Result<(u64, Vec<Instruction>, Pubkey)> {
    let sender = if let Some(sender) = sender {
        sender
    } else {
        spl_associated_token_account::get_associated_token_address(&sender_owner, &token)
    };

    let decimals = if let Some(d) = decimals {
        d
    } else {
        get_decimals(client, sender).await?
    };
    let transfer_balance = {
        // TODO error handling
        let sender_token_amount = client.get_token_account_balance(&sender).await?;

        // TODO error handling
        let sender_balance = sender_token_amount
            .amount
            .parse::<u64>()
            .map_err(crate::Error::custom)?;

        let transfer_balance = ui_amount_to_amount(ui_amount, decimals)?;
        if transfer_balance > sender_balance {
            // TODO: discuss if this error appropriate for token semantically?
            return Err(crate::Error::InsufficientSolanaBalance {
                needed: transfer_balance,
                balance: sender_balance,
            });
        }
        transfer_balance
    };

    let mut recipient_token_account = recipient;
    let mut minimum_balance_for_rent_exemption = 0;

    let recipient_is_token_account = {
        let recipient_account_info = client
            .get_account_with_commitment(&recipient, client.commitment())
            .await?
            .value
            .map(|account| {
                account.owner == spl_token::id()
                    && account.data.len() == spl_token::state::Account::LEN
            });

        if recipient_account_info.is_none() && !allow_unfunded_recipient {
            return Err(crate::Error::RecipientAddressNotFunded);
        }
        recipient_account_info.unwrap_or(false)
    };

    let mut instructions = vec![];
    if !recipient_is_token_account {
        recipient_token_account =
            spl_associated_token_account::get_associated_token_address(&recipient, &token);

        let needs_funding = {
            if let Some(recipient_token_account_data) = client
                .get_account_with_commitment(&recipient_token_account, client.commitment())
                .await?
                .value
            {
                match recipient_token_account_data.owner {
                    x if x == system_program::id() => true,
                    y if y == spl_token::id() => false,
                    _ => {
                        return Err(crate::Error::UnsupportedRecipientAddress(
                            recipient.to_string(),
                        ))
                    }
                }
            } else {
                true
            }
        };

        if needs_funding {
            if fund_recipient {
                minimum_balance_for_rent_exemption += client
                    .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
                    .await?;
                instructions.push(instruction::create_associated_token_account(
                    fee_payer,
                    &recipient,
                    &token,
                    &spl_associated_token_account::ID,
                ));
            } else {
                // TODO: discuss the logic of this error
                return Err(crate::Error::AssociatedTokenAccountDoesntExist);
            }
        }
    }

    instructions.push(transfer_checked(
        &spl_token::id(),
        &sender,
        &token,
        &recipient_token_account,
        &sender_owner,
        &[&sender_owner, fee_payer],
        transfer_balance,
        decimals,
    )?);

    instructions.push(spl_memo::build_memo(memo.as_bytes(), &[fee_payer]));

    Ok((
        minimum_balance_for_rent_exemption,
        instructions,
        recipient_token_account,
    ))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
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
    #[serde(with = "value::keypair")]
    pub sender_owner: Keypair,
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

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (minimum_balance_for_rent_exemption, instructions, recipient_token_account) =
        command_transfer_token(
            &ctx.solana_client,
            &input.fee_payer.pubkey(),
            input.mint_account,
            input.amount,
            input.decimals,
            input.recipient,
            input.sender_token_account,
            input.sender_owner.pubkey(),
            input.allow_unfunded,
            input.fund_recipient,
            input.memo,
        )
        .await?;

    let instructions = if input.submit {
        Instructions {
            fee_payer: input.fee_payer.pubkey(),
            signers: [
                input.fee_payer.clone_keypair(),
                input.sender_owner.clone_keypair(),
            ]
            .into(),
            minimum_balance_for_rent_exemption,
            instructions,
        }
    } else {
        <_>::default()
    };

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
