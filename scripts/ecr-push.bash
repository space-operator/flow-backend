#!/usr/bin/env bash
set -Eeuxo pipefail

ORG=space-operator
IMAGE=flow-server
NAME="$ORG/$IMAGE"

if [ "${1:-}" = "login" ]; then
    aws ecr-public get-login-password --region us-east-1 |
        docker login --username AWS --password-stdin "public.ecr.aws/$ORG"
fi

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

docker tag $NAME:$COMMIT public.ecr.aws/$NAME:$COMMIT
docker push public.ecr.aws/$NAME:$COMMIT

docker tag $NAME:$BRANCH public.ecr.aws/$NAME:$BRANCH
docker push public.ecr.aws/$NAME:$BRANCH

if [[ "$BRANCH" == "main" ]]; then
    docker tag $NAME:$COMMIT public.ecr.aws/$NAME:latest
    docker push public.ecr.aws/$NAME:latest
fi
