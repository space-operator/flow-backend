# Space Operator CLI

[![Crates.io][crates-badge]][crates-url]
[![AGPLv3 licensed][AGPLv3-badge]][AGPLv3-url]

[crates-badge]: https://img.shields.io/crates/v/space-operator-cli.svg
[crates-url]: https://crates.io/crates/space-operator-cli
[AGPLv3-badge]: https://img.shields.io/badge/license-AGPLv3-blue.svg
[AGPLv3-url]: ../../LICENSE

CLI for [Space Operator](https://spaceoperator.com).

## Install

Install using `cargo install`:

```shell
cargo install space-operator-cli --force
```

Binary name: `spo`

## Login

Run `spo login`:

```bash
$ spo login
Go to https://spaceoperator.com/dashboard/profile/apikey go generate a key
Please paste your API key below
```

Enter your API key to login.

## Run flow-server

Clone [flow-backend](https://github.com/space-operator/flow-backend) repository:

```bash
git clone https://github.com/space-operator/flow-backend
```

`cd` into `flow-backend` and run `spo start`:

```bash
cd flow-backend
spo start
```

This will create a configuration file for flow-server then compile and run it
(first compilation will take several minutes).

Example output:
```
generated config.toml
$ cargo build --bin flow-server
    Finished `dev` profile [optimized] target(s) in 0.60s
$ target/debug/flow-server config.toml
2024-10-19T14:19:36.514531Z  INFO flow_server: native commands: ["add_config_lines", "add_config_lines_core", "add_required_signatory", "add_signatory", "arweave_file_upload", "arweave_nft_upload", "associated_token_account", "attest_from_eth", "attest_token", "burn_cNFT", "burn_v1", "cancel_proposal", "cast_vote", "collect", "complete_native", "complete_proposal", "complete_transfer_wrapped", "const", "create_core_collection_v2", "create_core_v2", "create_governance", "create_master_edition", "create_metadata_account", "create_mint_account", "create_native_treasury", "create_proposal", "create_realm", "create_streamflow_timelock", "create_token_account", "create_token_owner_record", "create_tree", "create_v1", "create_wrapped", "create_wrapped_on_eth", "das_api", "delegate_v1", "deposit_governing_tokens", "execute_transaction", "fetch_assets", "fileexplorer", "finalize_vote", "find_pda", "flow_input", "flow_output", "flow_run_info", "foreach", "gen_metaplex_attrs", "gen_pdg_attrs", "generate_base", "generate_keypair", "get_balance", "get_effect_list", "get_foreign_asset_eth", "get_vaa", "governance_post_message", "http_request", "initialize_candy_guard", "initialize_candy_machine", "initialize_candy_machine_core", "initialize_core_candy_guards", "initialize_record_with_seed", "initialize_token_bridge", "insert_transaction", "interflow", "interflow_instructions", "json_extract", "json_get_field", "json_insert", "kv_create_store", "kv_delete_store", "kv_read_item", "kv_write_item", "kvexplorer", "memo", "mint", "mint_cNFT_to_collection", "mint_candy_machine_core", "mint_compressed_NFT", "mint_token", "mint_v1", "mpl_core_update_plugin", "nft_complete_native", "nft_complete_wrapped", "nft_complete_wrapped_meta", "nft_transfer_native", "nft_transfer_wrapped", "note", "parse_pdg_attrs", "parse_vaa", "pdg_render", "post_message", "post_vaa", "postgrest_builder_eq", "postgrest_builder_insert", "postgrest_builder_is", "postgrest_builder_limit", "postgrest_builder_match", "postgrest_builder_neq", "postgrest_builder_not", "postgrest_builder_order", "postgrest_builder_select", "postgrest_builder_update", "postgrest_builder_upsert", "postgrest_execute_query", "postgrest_new_query", "postgrest_new_rpc", "print", "push_effect_list", "pyth_price", "range", "read_record", "redeem_nft_on_eth", "redeem_on_eth", "refund_proposal_deposit", "relinquish_token_owner_record_locks", "relinquish_vote", "remove_required_signatory", "remove_transaction", "request_airdrop", "revoke_governing_tokens", "set_authority", "set_authority_2022", "set_governance_config", "set_governance_delegate", "set_realm_authority", "set_realm_config", "set_token_owner_record_locks", "sign_off_proposal", "storage_create_signed_url", "storage_delete", "storage_download", "storage_get_file_metadata", "storage_get_public_url", "storage_list", "storage_upload", "supabase", "to_bytes", "to_string", "to_vec", "transfer_cNFT", "transfer_from_eth", "transfer_native", "transfer_nft_from_eth", "transfer_sol", "transfer_token", "transfer_wrapped", "update_cNFT", "update_core_v1", "update_render_params", "update_v1", "verify_collection_v1", "verify_creator_v1", "verify_signatures", "wait", "wallet", "withdraw_governing_tokens", "withdraw_streamflow_timelock", "wrap", "wrap_core", "write_to_record"]
2024-10-19T14:19:36.514586Z  INFO flow_server: allow CORS origins: ["*"]
2024-10-19T14:19:36.528967Z  INFO db::local_storage: openning sled storage: _data/guest_local_storage
2024-10-19T14:19:37.728815Z  WARN flow_server: missing credentials, some routes are not available: need database credentials
2024-10-19T14:19:37.728846Z  INFO flow_server: listening on "0.0.0.0" port 8080
2024-10-19T14:19:37.729177Z  INFO actix_server::builder: starting 8 workers
2024-10-19T14:19:37.729198Z  INFO actix_server::server: Actix runtime found; starting in Actix runtime
2024-10-19T14:19:37.729207Z  INFO actix_server::server: starting service: "actix-web-service-0.0.0.0:8080", workers: 8, listening on: 0.0.0.0:8080
2024-10-19T14:19:37.759200Z DEBUG flow_server::db_worker::token_worker: started TokenWorker c334e245-75b4-49fd-93c0-c4b25ab74f70
2024-10-19T14:19:37.759314Z  INFO flow_server::db_worker: started DBWorker
```



# Command-Line Help for `spo`

This document contains the help content for the `spo` command-line program.

**Command Overview:**

* [`spo`↴](#spo)
* [`spo login`↴](#spo-login)
* [`spo start`↴](#spo-start)
* [`spo node`↴](#spo-node)
* [`spo node new`↴](#spo-node-new)
* [`spo node upload`↴](#spo-node-upload)
* [`spo generate`↴](#spo-generate)
* [`spo generate input`↴](#spo-generate-input)
* [`spo generate output`↴](#spo-generate-output)
* [`spo generate config`↴](#spo-generate-config)

## `spo`

**Usage:** `spo [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `login` — Login to Space Operator using API key
* `start` — Start flow-server
* `node` — Manage your nodes
* `generate` — Generate various things

###### **Options:**

* `--url <URL>` — URL of flow-server to use (default: https://dev-api.spaceoperator.com)



## `spo login`

Login to Space Operator using API key

**Usage:** `spo login`



## `spo start`

Start flow-server

**Usage:** `spo start [CONFIG]`

###### **Arguments:**

* `<CONFIG>` — Path to configuration file



## `spo node`

Manage your nodes

**Usage:** `spo node <COMMAND>`

###### **Subcommands:**

* `new` — Generate a new node
* `upload` — Upload nodes



## `spo node new`

Generate a new node

**Usage:** `spo node new [OPTIONS]`

###### **Options:**

* `--allow-dirty` — Allow dirty git repository
* `-p`, `--package <PACKAGE>` — Specify which Rust package to add the new node to



## `spo node upload`

Upload nodes

**Usage:** `spo node upload [OPTIONS] <PATH>`

###### **Arguments:**

* `<PATH>` — Path to JSON node definition file

###### **Options:**

* `--dry-run` — Only print diff, don't do anything
* `--no-confirm` — Don't ask for confirmation



## `spo generate`

Generate various things

**Usage:** `spo generate <COMMAND>`

###### **Subcommands:**

* `input` — Generate input struct
* `output` — Generate output struct
* `config` — Generate configuration file for flow-server



## `spo generate input`

Generate input struct

**Usage:** `spo generate input <PATH>`

###### **Arguments:**

* `<PATH>` — Path to node definition file



## `spo generate output`

Generate output struct

**Usage:** `spo generate output <PATH>`

###### **Arguments:**

* `<PATH>` — Path to node definition file



## `spo generate config`

Generate configuration file for flow-server

**Usage:** `spo generate config [PATH]`

###### **Arguments:**

* `<PATH>` — Path to save configuration file (default: config.toml)



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
