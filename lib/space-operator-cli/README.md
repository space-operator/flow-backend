# Space Operator CLI

[![Crates.io][crates-badge]][crates-url]
[![AGPLv3 licensed][AGPLv3-badge]][AGPLv3-url]

[crates-badge]: https://img.shields.io/crates/v/space-operator-cli.svg
[crates-url]: https://crates.io/crates/space-operator-cli
[AGPLv3-badge]: https://img.shields.io/badge/license-AGPLv3-blue.svg
[AGPLv3-url]: ../../LICENSE

CLI for [Space Operator](https://spaceoperator.com).

Table of contents:
- [Install](#install)
- [Login](#login)
- [Run flow-server](#run-flow-server)
- [Generate a new native node](#generate-a-new-native-node)
- [Generate input and output struct](#generate-input-and-output-struct)
- [Upload node](#upload-node)
- [Command-Line Help for spo](#command-line-help-for-spo)

## Install

Install using `cargo install`:

```shell
cargo install space-operator-cli --force
```

Binary name: `spo`

```
$ spo --help
Usage: spo [OPTIONS] [COMMAND]

Commands:
  login     Login to Space Operator using API key
  start     Start flow-server [aliases: s]
  node      Manage your nodes [aliases: n]
  generate  Generate various things [aliases: g]
  run       Run various binaries
  help      Print this message or the help of the given subcommand(s)

Options:
      --url <URL>  URL of flow-server to use (default: https://dev-api.spaceoperator.com)
  -h, --help       Print help
```

## Login

Run `spo login`:

```bash
$ spo login
Go to https://spaceoperator.com/dashboard/profile/apikey go generate a key
Please paste your API key below
```

Enter your API key to login.

## Run flow-server

### Run with a configuration file (TOML, JSON, or JSONC)

Clone [flow-backend](https://github.com/space-operator/flow-backend) repository:

```bash
git clone https://github.com/space-operator/flow-backend
```

`cd` into `flow-backend` and run `spo start`:


```bash
cd flow-backend
spo start config.jsonc
```

### Run with docker database

Start a local docker deployment as instructed in [docker/README.md](/docker/README.md).

Stop the docker flow-server:

```bash
docker compose down flow-server
```

Build and start a flow-server that connects to local docker database:

```bash
spo start --docker
```

## Generate a new native node

Make sure you are inside [flow-backend](https://github.com/space-operator/flow-backend) repository.

Generate with `spo node new`:
```
$ spo node new
could not determine which package to update
use `-p` option to specify a package
available packages:
    client
    flow-lib
    spo-helius
    flow-value
    space-lib
    cmds-deno
    command-rpc
    tower-rpc
    cmds-pdg
    pdg-common
    cmds-solana
    cmds-std
    db
    flow
    rhai-script
    space-wasm
    utils
    flow-server
```

Because our workspace have several packages, you must specify one of them to use
(our CLI can automatically choose one if you are inside one of them).

```
$ spo node new -p cmds-solana
```

Fill the prompts for node definition, for example:

```
$ spo node new -p cmds-solana
using package: cmds-solana
enter ? for help
? module path: ?
enter valid Rust module path to save the node (empty to save at root)
? module path:
? node id: transfer
? display name: Transfer
description:

adding node inputs (enter empty name to finish)
? name: fee_payer
? input type: keypair
? optional (true/false): false
? passthrough (true/false): true

adding node inputs (enter empty name to finish)
? name: amount
? input type: decimal
? optional (true/false): false
? passthrough (true/false): false

adding node inputs (enter empty name to finish)
? name:

adding node outputs (enter empty name to finish)
? name: balance
? output type: decimal
? optional (true/false): true

adding node outputs (enter empty name to finish)
? name:
will this node emit Solana instructions? (y/n): y
adding `signature` output
adding `submit` input
adding instruction info: {
  "before": [
    "balance",
    "fee_payer"
  ],
  "signature": "signature",
  "after": []
}
writing node definition to crates/cmds-solana/node-definitions/transfer.json
writing code to crates/cmds-solana/src/transfer.rs
updating module crates/cmds-solana/src/lib.rs
upload node (y/n): y
node: transfer
command is not in database
upload? (y/n): y
inserted new node, id=1256
view your node:
https://spaceoperator.com/dashboard/nodes/c334e245-75b4-49fd-93c0-c4b25ab74f70.transfer.0.1
```

Generated Rust code:
```rust
use flow_lib::command::prelude::*;
const NAME: &str = "transfer";
flow_lib::submit!(CommandDescription::new(NAME, | _ | build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/transfer.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsDecimal")]
    pub amount: Decimal,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsDecimal")]
    pub balance: Option<Decimal>,
    #[serde_as(as = "Option<Signature>")]
    pub signature: Option<Signature>,
}
async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);
    let signature = ctx
        .execute(Instructions::default(), value::map! {})
        .await?
        .signature;
    Err(CommandError::msg("unimplemented"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();
        build().unwrap().run(ctx, ValueSet::new()).await.unwrap_err();
    }
}
```

Then, you can use `spo start` to run a local flow-server and test your node in flow editor.

## Generate input and output struct

If you updated the node definition manually, you can use `spo generate input` and
`spo generate output` to generate new type definitions.

For example

```shell
spo generate input crates/cmds-solana/node-definitions/nft/v1/mint_v1.json
```

```rust
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
struct Input {
    fee_payer: Wallet,
    authority: Option<Wallet>,
    #[serde_as(as = "AsPubkey")]
    mint_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token_owner: Pubkey,
    amount: Option<u64>,
    #[serde_as(as = "Option<AsPubkey>")]
    delegate_record: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    authorization_rules_program: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    authorization_rules: Option<Pubkey>,
    authorization_data: Option<JsonValue>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}
```

## Upload node

Use `spo node upload` to upload new node definition or update existing one.
We only support `native` node at the moment.

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
* [`spo run`↴](#spo-run)

## `spo`

**Usage:** `spo [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `login` — Login to Space Operator using API key
* `start` — Start flow-server
* `node` — Manage your nodes
* `generate` — Generate various things
* `run` — Run various binaries

###### **Options:**

* `--url <URL>` — URL of flow-server to use (default: https://dev-api.spaceoperator.com)



## `spo login`

Login to Space Operator using API key

**Usage:** `spo login`



## `spo start`

Start flow-server

**Usage:** `spo start [OPTIONS] [CONFIG]`

**Command Alias:** `s`

###### **Arguments:**

* `<CONFIG>` — Path to configuration file

###### **Options:**

* `--docker` — Connect to local Docker instance
* `--release` — Use `--release` build



## `spo node`

Manage your nodes

**Usage:** `spo node <COMMAND>`

**Command Alias:** `n`

###### **Subcommands:**

* `new` — Generate a new node
* `upload` — Upload nodes



## `spo node new`

Generate a new node

**Usage:** `spo node new [OPTIONS]`

**Command Alias:** `n`

###### **Options:**

* `--allow-dirty` — Allow dirty git repository
* `-p`, `--package <PACKAGE>` — Specify which Rust package to add the new node to



## `spo node upload`

Upload nodes

**Usage:** `spo node upload [OPTIONS] <PATH>`

**Command Alias:** `u`

###### **Arguments:**

* `<PATH>` — Path to JSON node definition file

###### **Options:**

* `--dry-run` — Only print diff, don't do anything
* `--no-confirm` — Don't ask for confirmation



## `spo generate`

Generate various things

**Usage:** `spo generate <COMMAND>`

**Command Alias:** `g`

###### **Subcommands:**

* `input` — Generate input struct
* `output` — Generate output struct



## `spo generate input`

Generate input struct

**Usage:** `spo generate input <PATH>`

**Command Alias:** `i`

###### **Arguments:**

* `<PATH>` — Path to node definition file



## `spo generate output`

Generate output struct

**Usage:** `spo generate output <PATH>`

**Command Alias:** `o`

###### **Arguments:**

* `<PATH>` — Path to node definition file



## `spo run`

Run various binaries

**Usage:** `spo run [OPTIONS] <BIN>`

###### **Arguments:**

* `<BIN>` — Specify binary to run

  Possible values: `all-cmds-server`, `deno-cmds-server`


###### **Options:**

* `--release` — Run in release mode



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
