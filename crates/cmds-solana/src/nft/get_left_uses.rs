use crate::prelude::*;
use mpl_token_metadata::state::Metadata;
use solana_program::pubkey::Pubkey;

const NAME: &str = "get_left_uses";

const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/get_left_uses.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub left_uses: Option<u64>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let account_data = ctx
        .solana_client()
        .get_account_data(&metadata_account)
        .await?;

    let mut account_data_ptr = account_data.as_slice();

    let metadata = <Metadata as borsh::BorshDeserialize>::deserialize(&mut account_data_ptr)?;

    let left_uses = metadata.uses.map(|v| v.remaining);

    Ok(Output { left_uses })
}
