#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi


ORG=space-operator
IMAGE=flow-server
NAME="$ORG/$IMAGE"

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

$CMD tag $NAME:$COMMIT public.ecr.aws/$NAME:$COMMIT
$CMD push public.ecr.aws/$NAME:$COMMIT

$CMD tag $NAME:$BRANCH public.ecr.aws/$NAME:$BRANCH
$CMD push public.ecr.aws/$NAME:$BRANCH

if [[ "$BRANCH" == "main" ]]; then
    $CMD tag $NAME:$COMMIT public.ecr.aws/$NAME:latest
    $CMD push public.ecr.aws/$NAME:latest
fi
