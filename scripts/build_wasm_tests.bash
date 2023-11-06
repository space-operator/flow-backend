#!/usr/bin/bash
set -Eeuxo pipefail

TARGET_DIR="$PWD/target/"

for d in ./crates/space-wasm/tests/* ; do
    if [ -d "$d" ]; then
        pushd "$d" > /dev/null
        if ! [ -d "target/" ] && [ "$1" = "sub" ]; then
            btrfs subvolume create target/
        fi
        cargo build --release --quiet
        popd > /dev/null
    fi
done
