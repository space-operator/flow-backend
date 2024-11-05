#!/usr/bin/env bash
set -Eeuxo pipefail

rm Cargo.lock
cargo generate-lockfile
# cargo update -p spl-token-2022@5.0.2 --precise 0.9.0
cargo update -p borsh@0.9.3
