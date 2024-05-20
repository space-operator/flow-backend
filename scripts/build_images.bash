#!/usr/bin/env bash
set -Eeuo pipefail

NAME="space-operator/flow-server"
DOCKERFILE="crates/flow-server/Dockerfile"

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

set -x
time docker build --pull=always --target rustc -t "$NAME-rustc:latest" -f "$DOCKERFILE" .
time docker build --pull=always --target planner -t "$NAME-planner:latest" -f "$DOCKERFILE" .
time docker build --pull=always --target cacher -t "$NAME-cacher:latest" -f "$DOCKERFILE" .

BUILDER_TAG=$RANDOM
time docker build --pull=always --target builder -t "$NAME-builder:$BUILDER_TAG" -f "$DOCKERFILE" .

COMMIT="$(git rev-parse --verify HEAD)$DIRTY"
time docker build --pull=always -t "$NAME:$COMMIT" -f "$DOCKERFILE" .

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
docker tag $NAME:$COMMIT $NAME:$BRANCH

docker image rm "$NAME-builder:$BUILDER_TAG"
