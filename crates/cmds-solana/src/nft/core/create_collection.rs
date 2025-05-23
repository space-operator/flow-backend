use std::borrow::BorrowMut;

use mpl_core::{
    instructions::CreateCollectionV2Builder,
    types::{Plugin, PluginAuthorityPair},
};
use tracing::info;

use crate::prelude::*;

// Command Name
const NAME: &str = "create_core_collection_v2";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_create_collection.json");

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
    pub fee_payer: Wallet,
    pub collection: Wallet,
    pub update_authority: Option<Wallet>,
    // TODO: should be an array of keypairs
    pub verified_creator: Option<Wallet>,
    pub name: String,
    pub uri: String,
    pub plugins: Vec<PluginAuthorityPair>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // let mut additional_signers: Vec<Keypair> = Vec::new();
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

    let mut builder = CreateCollectionV2Builder::new();

    let builder = builder.borrow_mut();

    let builder = builder
        .collection(input.collection.pubkey())
        .payer(input.fee_payer.pubkey())
        .name(input.name)
        // .external_plugin_adapters(vec![ExternalPluginAdapterInitInfo::DataStore(
        //     DataStoreInitInfo {
        //         data_authority: mpl_core::types::PluginAuthority::Owner,
        //         init_plugin_authority: Some(mpl_core::types::PluginAuthority::UpdateAuthority),
        //         schema: Some(ExternalPluginAdapterSchema::Binary),
        //     },
        // )])
        .uri(input.uri);

    let builder = if !plugins.is_empty() {
        builder.plugins(plugins)
    } else {
        builder
    };

    // TODO: check this: update_authority is not in signer list
    let builder = match input.update_authority {
        Some(update_authority) => builder.update_authority(Some(update_authority.pubkey())),
        _ => builder,
    };

    let ins = builder.instruction();

    let collection_pubkey = input.collection.pubkey();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.collection]
            .into_iter()
            .chain(input.verified_creator)
            .collect(),
        instructions: [ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "collection" => collection_pubkey,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

// [
//     {
//         "plugin": {
//             "VerifiedCreators": {
//                 "signatures": [
//                     {
//                         "address": "DpfvhHU7z1CK8eP5xbEz8c4WBNHUfqUVtAE7opP2kJBc",
//                         "verified": true
//                     }
//                 ]
//             }
//         },
//         "authority": "Owner"
//     }
// ]
