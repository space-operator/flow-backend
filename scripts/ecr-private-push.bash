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

function validate_container_tag {
    local tag="$1"

    [[ "$tag" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]]
}

if [[ "${PUSH_BRANCH_TAGS:-0}" == "1" ]] && ! validate_container_tag "$BRANCH"; then
    echo "BRANCH '$BRANCH' is not a valid container tag; set BRANCH to a sanitized value or disable PUSH_BRANCH_TAGS." >&2
    exit 1
fi

function remote_tag_exists {
    local image="$1"
    local tag="$2"

    aws ecr describe-images \
        --region "$AWS_REGION" \
        --repository-name "$image" \
        --image-ids imageTag="$tag" \
        >/dev/null 2>&1
}

function wait_for_remote_tag {
    local image="$1"
    local tag="$2"
    local attempts="${ECR_TAG_WAIT_ATTEMPTS:-5}"
    local sleep_seconds="${ECR_TAG_WAIT_SECONDS:-2}"
    local attempt=1

    while (( attempt <= attempts )); do
        if remote_tag_exists "$image" "$tag"; then
            return 0
        fi

        if (( attempt < attempts )); then
            sleep "$sleep_seconds"
        fi

        attempt=$((attempt + 1))
    done

    return 1
}

function push_failed_because_tag_is_immutable {
    local output="$1"

    grep -qiE \
        'ImageTagAlreadyExistsException|tag invalid:.*already exists|cannot be overwritten because the tag is immutable' \
        <<<"$output"
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

    local push_output=""
    local push_status=0
    if push_output="$($CMD push "$remote_ref" 2>&1)"; then
        printf '%s\n' "$push_output"
        return 0
    else
        push_status=$?
        printf '%s\n' "$push_output"
    fi

    # A second job may win the race between describe-images and the final manifest push.
    if push_failed_because_tag_is_immutable "$push_output" && wait_for_remote_tag "$image" "$tag"; then
        echo "Skipping push for $image:$tag (tag appeared during push; treating immutable-tag conflict as success)."
        return 0
    fi

    return "$push_status"
}

function push {
    local IMAGE="$1"
    local LOCAL_NAME="space-operator/$IMAGE"
    local URL="$ECR/$IMAGE"

    push_tag_if_missing "$LOCAL_NAME:$COMMIT" "$URL:$COMMIT" "$IMAGE" "$COMMIT"

    if [[ "${PUSH_BRANCH_TAGS:-0}" == "1" ]]; then
        push_tag_if_missing "$LOCAL_NAME:$COMMIT" "$URL:$BRANCH" "$IMAGE" "$BRANCH"
    else
        echo "Skipping branch tag push for $BRANCH (immutable tags enabled; commit tags are source of truth)."
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
