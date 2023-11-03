use crate::prelude::*;
use metaboss_utils::commands::snapshot::{get_mint_accounts, SnapshotMintsArgs};

#[derive(Debug)]
pub struct SnapshotMints;

impl SnapshotMints {
    pub async fn snapshot_mints(
        client: &RpcClient,
        args: SnapshotMintsArgs,
    ) -> crate::Result<Vec<String>> {
        let mut mint_addresses = get_mint_accounts(
            client,
            &args.creator,
            args.position,
            args.update_authority,
            args.allow_unverified,
            args.v2,
        )
        .await
        .map_err(|_| crate::Error::FailedToFetchMintSnapshot)?;

        mint_addresses.sort_unstable();

        Ok(mint_addresses)
    }
}

const SNAPSHOT_MINTS: &str = "snapshot_mints";

// Inputs
const VALUE: &str = "value";
const POSITION: &str = "position";
const VALUE_TYPE: &str = "value_type";
const ALLOW_UNVERIFIED: &str = "allow_unverified";
const V2: &str = "v2";

// Outputs
const MINTS: &str = "mints";

// Value types
const CREATOR: &str = "creator";
const UPDATE_AUTHORITY: &str = "update_authority";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub value: String,
    pub position: u64,
    pub value_type: String,
    pub allow_unverified: bool,
    pub v2: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub mints: Vec<String>,
}

#[async_trait]
impl CommandTrait for SnapshotMints {
    fn name(&self) -> Name {
        SNAPSHOT_MINTS.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: VALUE.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: POSITION.into(),
                type_bounds: [ValueType::U64].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: VALUE_TYPE.into(),
                type_bounds: [ValueType::String].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: ALLOW_UNVERIFIED.into(),
                type_bounds: [ValueType::Bool].to_vec(),
                required: true,
                passthrough: false,
            },
            CmdInput {
                name: V2.into(),
                type_bounds: [ValueType::Bool].to_vec(),
                required: true,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: MINTS.into(),
            r#type: ValueType::Json,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input: Input = value::from_map(inputs)?;

        let mut args = SnapshotMintsArgs {
            creator: None,
            position: input.position as usize,
            update_authority: None,
            v2: input.v2,
            allow_unverified: input.allow_unverified,
            output: String::new(),
        };

        match input.value_type.as_str() {
            CREATOR => {
                args.creator.replace(input.value);
            }
            UPDATE_AUTHORITY => {
                args.update_authority.replace(input.value);
            }
            _ => {
                return Err(crate::Error::ErrorSnapshottingMints(
                    "an invalid value type was provided!".to_string(),
                )
                .into())
            }
        }
        let mints = Self::snapshot_mints(&ctx.solana_client, args)
            .await
            .map_err(crate::Error::custom)?;

        Ok(value::to_map(&Output { mints })?)
    }
}

inventory::submit!(CommandDescription::new(SNAPSHOT_MINTS, |_| Ok(Box::new(
    SnapshotMints
))));
