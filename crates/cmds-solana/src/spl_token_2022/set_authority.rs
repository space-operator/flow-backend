use crate::prelude::*;

const NAME: &str = "set_authority_2022";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/spl_token_2022/set_authority.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub owned_pubkey: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub new_authority: Option<Pubkey>,
    pub authority_type: AuthorityType,
    pub owner: Wallet,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ix = spl_token_2022_interface::instruction::set_authority(
        &spl_token_2022_interface::ID,
        &input.owned_pubkey,
        input.new_authority.as_ref(),
        input.authority_type.into(),
        &input.owner.pubkey(),
        &[],
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.owner].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AuthorityType {
    MintTokens,
    FreezeAccount,
    AccountOwner,
    CloseAccount,
}

impl From<AuthorityType> for spl_token_2022_interface::instruction::AuthorityType {
    fn from(value: AuthorityType) -> Self {
        match value {
            AuthorityType::MintTokens => Self::MintTokens,
            AuthorityType::FreezeAccount => Self::FreezeAccount,
            AuthorityType::AccountOwner => Self::AccountOwner,
            AuthorityType::CloseAccount => Self::CloseAccount,
        }
    }
}
