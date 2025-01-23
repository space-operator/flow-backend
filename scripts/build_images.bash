#!/usr/bin/env bash
set -Eexuo pipefail

CMD="podman"
BUILD="podman build --pull=always"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
    BUILD="docker build --pull"
fi

echo Using $CMD

PROFILE=${PROFILE:-release}

NAME="space-operator/flow-server"
DOCKERFILE="crates/flow-server/Dockerfile"

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

set -x
time $BUILD --target rustc -t "$NAME-rustc:latest" -f "$DOCKERFILE" .
time $BUILD --target planner -t "$NAME-planner:latest" -f "$DOCKERFILE" .
time $BUILD --target cacher --build-arg PROFILE=$PROFILE -t "$NAME-cacher:$PROFILE-latest" -f "$DOCKERFILE" .

BUILDER_TAG=$RANDOM
time $BUILD --target builder --build-arg PROFILE=$PROFILE -t "$NAME-builder:$BUILDER_TAG" -f "$DOCKERFILE" .

COMMIT="$(git rev-parse --verify HEAD)$DIRTY"
time $BUILD -t "$NAME:$COMMIT" -f "$DOCKERFILE" .

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
$CMD tag $NAME:$COMMIT $NAME:$BRANCH

$CMD image rm "$NAME-builder:$BUILDER_TAG"
