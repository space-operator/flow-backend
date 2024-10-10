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

```
Usage: spo [OPTIONS] [COMMAND]

Commands:
  login  Login to Space Operator using API key
  node   Manage your nodes
  help   Print this message or the help of the given subcommand(s)

Options:
      --url <URL>  URL of flow-server to use (default: https://dev-api.spaceoperator.com)
  -h, --help       Print help
```
