#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi

PROJECT_ID=${GCP_PROJECT_ID:-seraphic-spider-445423-f4}
ORG=app-cluster-docker

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

function push {
    IMAGE="$1"
    NAME="$ORG/$IMAGE"
    URL="us-west1-docker.pkg.dev/$PROJECT_ID/$NAME"

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
