use crate::prelude::*;
use anchor_lang_26::InstructionData;
use payments::state::Payment as ClockworkPayment;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub struct UpdatePayment;

// update disbursement amount
fn update_payment(payment_pubkey: Pubkey, payer: Pubkey, amount: u64) -> Instruction {
    // create instruction
    Instruction {
        program_id: payments::ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(payment_pubkey, false),
        ],
        data: payments::instruction::UpdatePayment {
            amount: Some(amount),
        }
        .data(),
    }
}

impl UpdatePayment {
    #[allow(clippy::too_many_arguments)]
    async fn command_update_payment(
        &self,
        rpc_client: &RpcClient,
        payer: Pubkey,
        payment: Pubkey,
        amount: u64,
    ) -> crate::Result<(u64, Vec<Instruction>)> {
        // FIXME min rent
        let minimum_balance_for_rent_exemption = rpc_client
            .get_minimum_balance_for_rent_exemption(80)
            .await?;

        let instructions = vec![update_payment(payment, payer, amount)];

        Ok((minimum_balance_for_rent_exemption, instructions))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub token_mint: Pubkey,
    #[serde(with = "value::pubkey")]
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    signature: Signature,
}

// Command Name
const UPDATE_PAYMENT: &str = "update_payment";

// Inputs
const PAYER: &str = "payer";
const TOKEN_MINT: &str = "token_mint";
const RECIPIENT: &str = "recipient";
const AMOUNT: &str = "amount";

// Outputs
const SIGNATURE: &str = "signature";

// TODO
// convert schedule
// /home/amir/.cargo/registry/src/github.com-1ecc6299db9ec823/clockwork-cron-1.4.0/src/schedule.rs

#[async_trait]
impl CommandTrait for UpdatePayment {
    fn name(&self) -> Name {
        UPDATE_PAYMENT.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: PAYER.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: true,
            },
            CmdInput {
                name: TOKEN_MINT.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: RECIPIENT.into(),
                type_bounds: [ValueType::Keypair, ValueType::String, ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: AMOUNT.into(),
                type_bounds: [ValueType::U64].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: SIGNATURE.into(),
            r#type: ValueType::String,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input {
            payer,
            token_mint,
            recipient,
            amount,
        } = value::from_map(inputs.clone())?;

        // Derive PDAs
        let payment = ClockworkPayment::pubkey(payer.pubkey(), token_mint, recipient);

        // Create Instructions
        let (minimum_balance_for_rent_exemption, instructions) = self
            .command_update_payment(&ctx.solana_client, payer.pubkey(), payment, amount)
            .await?;

        let (mut transaction, recent_blockhash) = execute(
            &ctx.solana_client,
            &payer.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )
        .await?;

        try_sign_wallet(&ctx, &mut transaction, &[&payer], recent_blockhash).await?;

        let signature = submit_transaction(&ctx.solana_client, transaction).await?;

        Ok(value::to_map(&Output { signature })?)
    }
}

inventory::submit!(CommandDescription::new(UPDATE_PAYMENT, |_| {
    Ok(Box::new(UpdatePayment))
}));
