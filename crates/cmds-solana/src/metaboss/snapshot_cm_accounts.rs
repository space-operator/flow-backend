use crate::prelude::*;
use metaboss_utils::commands::snapshot::{snapshot_cm_accounts, CandyMachineAccount};

#[derive(Clone, Debug)]
pub struct SnapshotCMAccounts;

impl SnapshotCMAccounts {
    pub async fn snapshot_cm_accounts(
        client: &RpcClient,
        update_authority: &str,
    ) -> crate::Result<Vec<CandyMachineAccount>> {
        let accounts = snapshot_cm_accounts(client, update_authority)
            .await
            .map_err(|_| crate::Error::FailedToFetchMintSnapshot)?;

        Ok(accounts.candy_machine_accounts)
    }
}

const SNAPSHOT_CM_ACCOUNTS: &str = "snapshot_cm_accounts";

// Inputs
const UPDATE_AUTHORITY: &str = "update_authority";

// Outputs
const ACCOUNTS: &str = "accounts";

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub update_authority: Pubkey,
}

#[derive(Serialize, Debug)]
pub struct Output {
    pub accounts: Vec<CandyMachineAccount>,
}

#[async_trait]
impl CommandTrait for SnapshotCMAccounts {
    fn name(&self) -> Name {
        SNAPSHOT_CM_ACCOUNTS.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [CmdInput {
            name: UPDATE_AUTHORITY.into(),
            type_bounds: [ValueType::Pubkey].to_vec(),
            required: true,
            passthrough: false,
        }]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: ACCOUNTS.into(),
            r#type: ValueType::Json,
        }]
        .to_vec()
    }

    async fn run(&self, ctx: Context, inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let input: Input = value::from_map(inputs)?;

        let holders =
            Self::snapshot_cm_accounts(&ctx.solana_client, &input.update_authority.to_string())
                .await
                .map_err(crate::Error::custom)?;

        Ok(value::to_map(&Output { accounts: holders })?)
    }
}

inventory::submit!(CommandDescription::new(SNAPSHOT_CM_ACCOUNTS, |_| Ok(
    Box::new(SnapshotCMAccounts)
)));
