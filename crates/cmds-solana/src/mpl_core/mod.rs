// Existing commands
pub mod create_asset;
pub mod create_collection;
pub mod execute;
pub mod fetch_assets;
pub mod update_asset;
pub mod update_plugin;
pub mod write_data;

// Core lifecycle
pub mod burn_collection_v1;
pub mod burn_v1;
pub mod transfer_v1;
pub mod update_collection_v1;
pub mod update_v2;

// Asset plugin management
pub mod add_plugin_v1;
pub mod approve_plugin_authority_v1;
pub mod remove_plugin_v1;
pub mod revoke_plugin_authority_v1;

// Collection plugin management
pub mod add_collection_plugin_v1;
pub mod approve_collection_plugin_authority_v1;
pub mod remove_collection_plugin_v1;
pub mod revoke_collection_plugin_authority_v1;
pub mod update_collection_plugin_v1;

// External plugin adapters (asset)
pub mod add_external_plugin_adapter_v1;
pub mod remove_external_plugin_adapter_v1;
pub mod update_external_plugin_adapter_v1;
pub mod write_external_plugin_adapter_data_v1;

// External plugin adapters (collection)
pub mod add_collection_external_plugin_adapter_v1;
pub mod remove_collection_external_plugin_adapter_v1;
pub mod update_collection_external_plugin_adapter_v1;
pub mod write_collection_external_plugin_adapter_data_v1;

// Compression & other
pub mod collect;
pub mod compress_v1;
pub mod decompress_v1;
