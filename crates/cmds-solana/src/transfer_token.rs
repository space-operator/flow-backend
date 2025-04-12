use crate::get_decimals;
use crate::{prelude::*, utils::ui_amount_to_amount};
use solana_commitment_config::CommitmentConfig;
use solana_program::instruction::Instruction;
use solana_program::program_pack::Pack;
use solana_program::system_program;
use spl_associated_token_account::instruction;
use spl_token::instruction::transfer_checked;
use tracing::info;

const SOLANA_TRANSFER_TOKEN: &str = "transfer_token";

const DEFINITION: &str = flow_lib::node_definition!("transfer_token.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(SOLANA_TRANSFER_TOKEN)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(SOLANA_TRANSFER_TOKEN, |_| build()));

#[allow(clippy::too_many_arguments)]
async fn command_transfer_token(
    client: &RpcClient,
    fee_payer: &Pubkey,
    token_mint: Pubkey,
    ui_amount: Decimal,
    decimals: Option<u8>,
    recipient: Pubkey,
    sender: Option<Pubkey>,
    sender_owner: Pubkey,
    allow_unfunded_recipient: bool,
    fund_recipient: bool,
    memo: String,
) -> crate::Result<(Vec<Instruction>, Pubkey)> {
    let sender_token_acc = if let Some(sender) = sender {
        sender
    } else {
        spl_associated_token_account::get_associated_token_address(&sender_owner, &token_mint)
    };

    let decimals = if let Some(d) = decimals {
        d
    } else {
        get_decimals(client, token_mint).await?
    };

    let commitment = CommitmentConfig::confirmed();

    let transfer_balance = {
        // TODO error handling
        let sender_token_amount = client
            .get_token_account_balance_with_commitment(&sender_token_acc, commitment)
            .await?
            .value;

        info!("sender_token_amount: {:?}", sender_token_amount);

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

    let recipient_is_token_account = {
        let recipient_account_info = client
            .get_account_with_commitment(&recipient, commitment)
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

    info!(
        "recipient_is_token_account: {:?}",
        recipient_is_token_account
    );

    let mut instructions = vec![];
    if !recipient_is_token_account {
        recipient_token_account =
            spl_associated_token_account::get_associated_token_address(&recipient, &token_mint);

        let needs_funding = {
            match client
                .get_account_with_commitment(&recipient_token_account, commitment)
                .await?
                .value
            {
                Some(recipient_token_account_data) => match recipient_token_account_data.owner {
                    x if x == system_program::ID => true,
                    y if y == spl_token::ID => false,
                    _ => {
                        return Err(crate::Error::UnsupportedRecipientAddress(
                            recipient.to_string(),
                        ));
                    }
                },
                _ => true,
            }
        };

        if needs_funding {
            if fund_recipient {
                instructions.push(instruction::create_associated_token_account(
                    fee_payer,
                    &recipient,
                    &token_mint,
                    &spl_token::ID,
                ));
            } else {
                // TODO: discuss the logic of this error
                return Err(crate::Error::AssociatedTokenAccountDoesntExist);
            }
        }
    }

    instructions.push(transfer_checked(
        &spl_token::ID,
        &sender_token_acc,
        &token_mint,
        &recipient_token_account,
        &sender_owner,
        &[&sender_owner, fee_payer],
        transfer_balance,
        decimals,
    )?);

    instructions.push(spl_memo::build_memo(memo.as_bytes(), &[fee_payer]));

    Ok((instructions, recipient_token_account))
}

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

async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let (instructions, recipient_token_account) = command_transfer_token(
        &ctx.solana_client(),
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

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.sender_owner].into(),
        instructions,
    };

    let instructions = input.submit.then_some(ins).unwrap_or_default();

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
