#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi

PROJECT_ID=${GCP_PROJECT_ID:-seraphic-spider-445423-f4}
LOCAL_ORG=space-operator
ORG=app-cluster-docker

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

function push {
    IMAGE="$1"
    LOCAL_NAME="$LOCAL_ORG/$IMAGE"
    URL="us-west1-docker.pkg.dev/$PROJECT_ID/$ORG/$IMAGE"

    $CMD tag $LOCAL_NAME:$COMMIT $URL:$COMMIT
    $CMD push $URL:$COMMIT

    $CMD tag $LOCAL_NAME:$BRANCH $URL:$BRANCH
    $CMD push $URL:$BRANCH

    if [[ "$BRANCH" == "main" ]]; then
        $CMD tag $LOCAL_NAME:$COMMIT $URL:latest
        $CMD push $URL:latest
    fi
}

push flow-server
push cmds-server
push schema-server
