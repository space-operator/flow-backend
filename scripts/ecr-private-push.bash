#!/usr/bin/env bash
set -Eeuxo pipefail

CMD="podman"
if [[ "${1:-}" == docker ]]; then
    CMD="docker"
fi

AWS_REGION="${AWS_REGION:-us-west-2}"
ECR="311141552572.dkr.ecr.us-west-2.amazonaws.com"

if [ "${1:-}" = "login" ] || [ "${2:-}" = "login" ]; then
    aws ecr get-login-password --region "$AWS_REGION" |
        $CMD login --username AWS --password-stdin "$ECR"
fi

DIRTY=""
if [[ "$(git describe --always --dirty)" == *-dirty ]]; then
    DIRTY="-dirty"
fi

BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)$DIRTY}"
COMMIT="$(git rev-parse --verify HEAD)$DIRTY"

function remote_tag_exists {
    local image="$1"
    local tag="$2"

    aws ecr describe-images \
        --region "$AWS_REGION" \
        --repository-name "$image" \
        --image-ids imageTag="$tag" \
        >/dev/null 2>&1
}

function push_tag_if_missing {
    local local_ref="$1"
    local remote_ref="$2"
    local image="$3"
    local tag="$4"

    if remote_tag_exists "$image" "$tag"; then
        echo "Skipping push for $image:$tag (tag already exists; immutable tags enabled)."
        return 0
    fi

    $CMD tag "$local_ref" "$remote_ref"
    $CMD push "$remote_ref"
}

function push {
    local IMAGE="$1"
    local LOCAL_NAME="space-operator/$IMAGE"
    local URL="$ECR/$IMAGE"

    push_tag_if_missing "$LOCAL_NAME:$COMMIT" "$URL:$COMMIT" "$IMAGE" "$COMMIT"

    if [[ "$BRANCH" == "main" ]]; then
        echo "Skipping branch tag push for main (immutable tags enabled; commit tags are source of truth)."
    else
        push_tag_if_missing "$LOCAL_NAME:$BRANCH" "$URL:$BRANCH" "$IMAGE" "$BRANCH"
    fi

    if [[ "$BRANCH" == "main" && "${PUSH_LATEST_TAG:-0}" == "1" ]]; then
        push_tag_if_missing "$LOCAL_NAME:$COMMIT" "$URL:latest" "$IMAGE" "latest"
    elif [[ "$BRANCH" == "main" ]]; then
        echo "Skipping :latest push (immutable tags enabled)."
    fi
}

push flow-server
push cmds-server
push schema-server
