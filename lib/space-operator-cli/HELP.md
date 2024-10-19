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
