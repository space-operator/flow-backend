use crate::prelude::*;
use metaboss_utils::commands::collections::{migrate_collection, MigrateArgs};

#[derive(Clone, Debug)]
pub struct MigrateCollection;

const MIGRATE_COLLECTION: &str = "migrate_collection";

// Inputs
const KEYPAIR: &str = "keypair";
const MINT_ADDRESS: &str = "mint_address";
const CANDY_MACHINE_ID: &str = "candy_machine_id";
const MINT_LIST: &str = "mint_list";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub keypair: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_address: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub candy_machine_id: Option<Pubkey>,
    pub mint_list: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {}

#[async_trait]
impl CommandTrait for MigrateCollection {
    fn name(&self) -> Name {
        MIGRATE_COLLECTION.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: KEYPAIR.into(),
                type_bounds: [ValueType::Keypair].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: MINT_ADDRESS.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: CANDY_MACHINE_ID.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: MINT_LIST.into(),
                type_bounds: [ValueType::Json].to_vec(),
                required: false,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [].to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input: Input = value::from_map(inputs)?;

        let args = MigrateArgs {
            client: &ctx.solana_client,
            keypair: input.keypair,
            mint_address: input.mint_address.to_string(),
            candy_machine_id: input.candy_machine_id.map(|a| a.to_string()),
            mint_list: input.mint_list,
            retries: 8,
            batch_size: 10,
        };

        migrate_collection(&args)
            .await
            .map_err(crate::Error::custom)?;

        Ok(value::to_map(&Output {})?)
    }
}

inventory::submit!(CommandDescription::new(MIGRATE_COLLECTION, |_| Ok(
    Box::new(MigrateCollection)
)));
