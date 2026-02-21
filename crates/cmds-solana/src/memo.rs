use crate::prelude::*;

const NAME: &str = "memo";

const DEFINITION: &str = flow_lib::node_definition!("memo.json");

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    memo: String,
    memo_signers: Option<Vec<Wallet>>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut memo_signers = input.memo_signers.unwrap_or_default();

    // Validate memo fits in a single transaction.
    // overhead = 168 (fixed) + 97 per memo_signer
    // compact-u16 encoding of data length: 1 byte if < 128, 2 bytes otherwise
    let overhead = 168 + 97 * memo_signers.len();
    let compact_len = if input.memo.len() < 128 { 1 } else { 2 };
    let max_memo = 1232_usize
        .saturating_sub(overhead)
        .saturating_sub(compact_len);
    if input.memo.len() > max_memo {
        return Err(CommandError::msg(format!(
            "memo too large: {} bytes, max is {} bytes with {} signer(s)",
            input.memo.len(),
            max_memo,
            memo_signers.len(),
        )));
    }

    let memo_signers_pubkey = memo_signers.iter().map(|s| s.pubkey()).collect::<Vec<_>>();
    let instruction = spl_memo_interface::instruction::build_memo(
        &spl_memo_interface::v3::ID,
        input.memo.as_bytes(),
        &memo_signers_pubkey.iter().collect::<Vec<_>>(),
    );

    memo_signers.insert(0, input.fee_payer);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: memo_signers.last().unwrap().pubkey(),
        signers: memo_signers,
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
