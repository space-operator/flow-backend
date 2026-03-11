#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi

ECR="311141552572.dkr.ecr.us-west-2.amazonaws.com"

if [ "${1:-}" = "login" ] || [ "${2:-}" = "login" ]; then
    aws ecr get-login-password --region us-west-2 |
        $CMD login --username AWS --password-stdin "$ECR"
fi

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

function push {
    IMAGE="$1"
    LOCAL_NAME="space-operator/$IMAGE"
    URL="$ECR/$IMAGE"

    $CMD tag "$LOCAL_NAME:$COMMIT" "$URL:$COMMIT"
    $CMD push "$URL:$COMMIT"

    $CMD tag "$LOCAL_NAME:$BRANCH" "$URL:$BRANCH"
    $CMD push "$URL:$BRANCH"

    if [[ "$BRANCH" == "main" && "${PUSH_LATEST_TAG:-0}" == "1" ]]; then
        $CMD tag "$LOCAL_NAME:$COMMIT" "$URL:latest"
        $CMD push "$URL:latest"
    elif [[ "$BRANCH" == "main" ]]; then
        echo "Skipping :latest push (immutable tags enabled)."
    fi
}

push flow-server
push cmds-server
push schema-server
