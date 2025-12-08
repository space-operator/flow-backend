#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi

ORG=space-operator

if [ "${1:-}" = "login" ]; then
    aws ecr-public get-login-password --region us-east-1 |
        $CMD login --username AWS --password-stdin "public.ecr.aws/$ORG"
fi

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

function push {
    IMAGE="$1"
    NAME="$ORG/$IMAGE"
    URL="public.ecr.aws/$NAME"

    $CMD tag $NAME:$COMMIT $URL:$COMMIT
    $CMD push $URL:$COMMIT

    $CMD tag $NAME:$BRANCH $URL:$BRANCH
    $CMD push $URL:$BRANCH

    if [[ "$BRANCH" == "main" ]]; then
        $CMD tag $NAME:$COMMIT $URL:latest
        $CMD push $URL:latest
    fi
}

push flow-server
push cmds-server
