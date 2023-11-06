#!/usr/bin/env bash
set -Eeuxo pipefail

mkdir -p ./target/container

time docker run --rm --name BUILD-SPACE \
    -v "${PWD}":/build:Z,ro \
    -v "${PWD}/target/container":/build/target:Z,rw \
    -v SPACE-CARGO:/usr/local/cargo \
    -e RUSTFLAGS="-C target-cpu=haswell" \
    rust:latest \
    bash -c "
    cd /build/
    cargo build --release -p flow-server
    "

mkdir -p ./target/container_bin
cp ./target/container/release/flow-server ./target/container_bin
strip ./target/container_bin/*

docker build -t flow-server:latest -f ./crates/flow-server/Dockerfile ./target/container_bin
