use mpl_core::{
    instructions::{CreateV1Builder, CreateV2Builder},
    types::{Plugin, PluginAuthorityPair},
};
use tracing::info;

use crate::prelude::*;

// Command Name
const NAME: &str = "create_core_v2";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_create_asset.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    pub asset: Keypair,
    #[serde(with = "value::keypair::opt")]
    pub authority: Option<Keypair>,
    pub name: String,
    pub uri: String,
    pub plugins: Vec<PluginAuthorityPair>,
    #[serde(with = "value::keypair::opt")]
    pub verified_creator: Option<Keypair>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut additional_signers: Vec<Keypair> = Vec::new();
    let mut creators: Vec<Pubkey> = Vec::new();

    let plugins: Vec<PluginAuthorityPair> = input
        .plugins
        .iter()
        .map(|plugin_authority_pair| {
            let plugin = match &plugin_authority_pair.plugin {
                Plugin::Royalties(royalties) => Plugin::Royalties(royalties.clone()),
                Plugin::FreezeDelegate(freeze_delegate) => {
                    Plugin::FreezeDelegate(freeze_delegate.clone())
                }
                Plugin::BurnDelegate(burn_delegate) => Plugin::BurnDelegate(burn_delegate.clone()),
                Plugin::TransferDelegate(transfer_delegate) => {
                    Plugin::TransferDelegate(transfer_delegate.clone())
                }
                Plugin::UpdateDelegate(update_delegate) => {
                    Plugin::UpdateDelegate(update_delegate.clone())
                }
                Plugin::PermanentFreezeDelegate(permanent_freeze_delegate) => {
                    Plugin::PermanentFreezeDelegate(permanent_freeze_delegate.clone())
                }
                Plugin::Attributes(attributes) => Plugin::Attributes(attributes.clone()),
                Plugin::PermanentTransferDelegate(permanent_transfer_delegate) => {
                    Plugin::PermanentTransferDelegate(permanent_transfer_delegate.clone())
                }
                Plugin::PermanentBurnDelegate(permanent_burn_delegate) => {
                    Plugin::PermanentBurnDelegate(permanent_burn_delegate.clone())
                }
                Plugin::Edition(edition) => Plugin::Edition(edition.clone()),
                Plugin::MasterEdition(master_edition) => {
                    Plugin::MasterEdition(master_edition.clone())
                }
                Plugin::AddBlocker(add_blocker) => Plugin::AddBlocker(add_blocker.clone()),
                Plugin::ImmutableMetadata(immutable_metadata) => {
                    Plugin::ImmutableMetadata(immutable_metadata.clone())
                }
                Plugin::VerifiedCreators(verified_creators) => {
                    for signature in &verified_creators.signatures {
                        if signature.verified {
                            info!("verified creator: {}", signature.address);
                            creators.push(signature.address);
                        }
                    }
                    Plugin::VerifiedCreators(verified_creators.clone())
                }
                Plugin::Autograph(autograph) => Plugin::Autograph(autograph.clone()),
            };
            PluginAuthorityPair {
                plugin,
                authority: plugin_authority_pair.authority.clone(),
            }
        })
        .collect();

    let mut builder = CreateV1Builder::new();

    let builder = builder
        .asset(input.asset.pubkey())
        .payer(input.fee_payer.pubkey())
        .name(input.name)
        .uri(input.uri);

    let builder = if let Some(ref authority) = input.authority {
        builder.authority(Some(authority.pubkey()))
    } else {
        builder.authority(None)
    };

    let builder = if let Some(collection) = input.collection {
        builder.collection(Some(collection))
    } else {
        builder
    };

    let builder = if !plugins.is_empty() {
        builder.plugins(plugins)
    } else {
        builder
    };

    let ins = builder.instruction();

    let mut signers = vec![input.fee_payer.clone_keypair(), input.asset.clone_keypair()];

    if let Some(authority) = input.authority.as_ref() {
        signers.push(authority.clone_keypair());
    }

    if let Some(verified_creator) = input.verified_creator {
        signers.push(verified_creator.clone_keypair());
    }

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: [ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
