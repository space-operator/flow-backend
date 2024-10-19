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

`cd` into `flow-backend` and run `spo start`, this will create a configuration file for
flow-server then compile and run it (first compilation will take several minutes).

```bash
cd flow-backend
spo start
```
Output:
```
$ cargo build --bin flow-server
    Finished `dev` profile [optimized] target(s) in 0.54s
$ target/debug/flow-server config.toml
2024-10-19T14:18:26.779958Z <span style="color:green;"> INFO</span> flow_server: native commands: [&quot;add_config_lines&quot;, &quot;add_config_lines_core&quot;, &quot;add_required_signatory&quot;, &quot;add_signatory&quot;, &quot;arweave_file_upload&quot;, &quot;arweave_nft_upload&quot;, &quot;associated_token_account&quot;, &quot;attest_from_eth&quot;, &quot;attest_token&quot;, &quot;burn_cNFT&quot;, &quot;burn_v1&quot;, &quot;cancel_proposal&quot;, &quot;cast_vote&quot;, &quot;collect&quot;, &quot;complete_native&quot;, &quot;complete_proposal&quot;, &quot;complete_transfer_wrapped&quot;, &quot;const&quot;, &quot;create_core_collection_v2&quot;, &quot;create_core_v2&quot;, &quot;create_governance&quot;, &quot;create_master_edition&quot;, &quot;create_metadata_account&quot;, &quot;create_mint_account&quot;, &quot;create_native_treasury&quot;, &quot;create_proposal&quot;, &quot;create_realm&quot;, &quot;create_streamflow_timelock&quot;, &quot;create_token_account&quot;, &quot;create_token_owner_record&quot;, &quot;create_tree&quot;, &quot;create_v1&quot;, &quot;create_wrapped&quot;, &quot;create_wrapped_on_eth&quot;, &quot;das_api&quot;, &quot;delegate_v1&quot;, &quot;deposit_governing_tokens&quot;, &quot;execute_transaction&quot;, &quot;fetch_assets&quot;, &quot;fileexplorer&quot;, &quot;finalize_vote&quot;, &quot;find_pda&quot;, &quot;flow_input&quot;, &quot;flow_output&quot;, &quot;flow_run_info&quot;, &quot;foreach&quot;, &quot;gen_metaplex_attrs&quot;, &quot;gen_pdg_attrs&quot;, &quot;generate_base&quot;, &quot;generate_keypair&quot;, &quot;get_balance&quot;, &quot;get_effect_list&quot;, &quot;get_foreign_asset_eth&quot;, &quot;get_vaa&quot;, &quot;governance_post_message&quot;, &quot;http_request&quot;, &quot;initialize_candy_guard&quot;, &quot;initialize_candy_machine&quot;, &quot;initialize_candy_machine_core&quot;, &quot;initialize_core_candy_guards&quot;, &quot;initialize_record_with_seed&quot;, &quot;initialize_token_bridge&quot;, &quot;insert_transaction&quot;, &quot;interflow&quot;, &quot;interflow_instructions&quot;, &quot;json_extract&quot;, &quot;json_get_field&quot;, &quot;json_insert&quot;, &quot;kv_create_store&quot;, &quot;kv_delete_store&quot;, &quot;kv_read_item&quot;, &quot;kv_write_item&quot;, &quot;kvexplorer&quot;, &quot;memo&quot;, &quot;mint&quot;, &quot;mint_cNFT_to_collection&quot;, &quot;mint_candy_machine_core&quot;, &quot;mint_compressed_NFT&quot;, &quot;mint_token&quot;, &quot;mint_v1&quot;, &quot;mpl_core_update_plugin&quot;, &quot;nft_complete_native&quot;, &quot;nft_complete_wrapped&quot;, &quot;nft_complete_wrapped_meta&quot;, &quot;nft_transfer_native&quot;, &quot;nft_transfer_wrapped&quot;, &quot;note&quot;, &quot;parse_pdg_attrs&quot;, &quot;parse_vaa&quot;, &quot;pdg_render&quot;, &quot;post_message&quot;, &quot;post_vaa&quot;, &quot;postgrest_builder_eq&quot;, &quot;postgrest_builder_insert&quot;, &quot;postgrest_builder_is&quot;, &quot;postgrest_builder_limit&quot;, &quot;postgrest_builder_match&quot;, &quot;postgrest_builder_neq&quot;, &quot;postgrest_builder_not&quot;, &quot;postgrest_builder_order&quot;, &quot;postgrest_builder_select&quot;, &quot;postgrest_builder_update&quot;, &quot;postgrest_builder_upsert&quot;, &quot;postgrest_execute_query&quot;, &quot;postgrest_new_query&quot;, &quot;postgrest_new_rpc&quot;, &quot;print&quot;, &quot;push_effect_list&quot;, &quot;pyth_price&quot;, &quot;range&quot;, &quot;read_record&quot;, &quot;redeem_nft_on_eth&quot;, &quot;redeem_on_eth&quot;, &quot;refund_proposal_deposit&quot;, &quot;relinquish_token_owner_record_locks&quot;, &quot;relinquish_vote&quot;, &quot;remove_required_signatory&quot;, &quot;remove_transaction&quot;, &quot;request_airdrop&quot;, &quot;revoke_governing_tokens&quot;, &quot;set_authority&quot;, &quot;set_authority_2022&quot;, &quot;set_governance_config&quot;, &quot;set_governance_delegate&quot;, &quot;set_realm_authority&quot;, &quot;set_realm_config&quot;, &quot;set_token_owner_record_locks&quot;, &quot;sign_off_proposal&quot;, &quot;storage_create_signed_url&quot;, &quot;storage_delete&quot;, &quot;storage_download&quot;, &quot;storage_get_file_metadata&quot;, &quot;storage_get_public_url&quot;, &quot;storage_list&quot;, &quot;storage_upload&quot;, &quot;supabase&quot;, &quot;to_bytes&quot;, &quot;to_string&quot;, &quot;to_vec&quot;, &quot;transfer_cNFT&quot;, &quot;transfer_from_eth&quot;, &quot;transfer_native&quot;, &quot;transfer_nft_from_eth&quot;, &quot;transfer_sol&quot;, &quot;transfer_token&quot;, &quot;transfer_wrapped&quot;, &quot;update_cNFT&quot;, &quot;update_core_v1&quot;, &quot;update_render_params&quot;, &quot;update_v1&quot;, &quot;verify_collection_v1&quot;, &quot;verify_creator_v1&quot;, &quot;verify_signatures&quot;, &quot;wait&quot;, &quot;wallet&quot;, &quot;withdraw_governing_tokens&quot;, &quot;withdraw_streamflow_timelock&quot;, &quot;wrap&quot;, &quot;wrap_core&quot;, &quot;write_to_record&quot;]
2024-10-19T14:18:26.780000Z <span style="color:green;"> INFO</span> flow_server: allow CORS origins: [&quot;*&quot;]
2024-10-19T14:18:26.796032Z <span style="color:green;"> INFO</span> db::local_storage: openning sled storage: _data/guest_local_storage
2024-10-19T14:18:28.002045Z <span style="color:olive;"> WARN</span> flow_server: missing credentials, some routes are not available: need database credentials
2024-10-19T14:18:28.002070Z <span style="color:green;"> INFO</span> flow_server: listening on &quot;0.0.0.0&quot; port 8080
2024-10-19T14:18:28.002378Z <span style="color:green;"> INFO</span> actix_server::builder: starting 8 workers
2024-10-19T14:18:28.002423Z <span style="color:green;"> INFO</span> actix_server::server: Actix runtime found; starting in Actix runtime
2024-10-19T14:18:28.002429Z <span style="color:green;"> INFO</span> actix_server::server: starting service: &quot;actix-web-service-0.0.0.0:8080&quot;, workers: 8, listening on: 0.0.0.0:8080
2024-10-19T14:18:28.027654Z <span style="color:blue;">DEBUG</span> flow_server::db_worker::token_worker: started TokenWorker c334e245-75b4-49fd-93c0-c4b25ab74f70
2024-10-19T14:18:28.027737Z <span style="color:green;"> INFO</span> flow_server::db_worker: started DBWorker
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
