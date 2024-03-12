#!/usr/bin/env bash
set -Eeuo pipefail

[ -z "$(git diff -- Cargo.lock)" ]
