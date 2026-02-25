#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Generate an LLM prompt to convert a legacy node-definition JSON into V2 JSONC.

Usage:
  scripts/generate_node_v2_llm_prompt.bash <legacy-json-path> [author_handle] [output-jsonc-path]

Arguments:
  legacy-json-path   Path to legacy node definition JSON (e.g. crates/cmds-std/node-definitions/const.json)
  author_handle      Optional author handle for V2 output (default: spo)
  output-jsonc-path  Optional suggested output path (default: <legacy>.v2.jsonc)

Example:
  scripts/generate_node_v2_llm_prompt.bash \
    crates/cmds-std/node-definitions/const.json \
    spo \
    crates/cmds-std/node-definitions/const.v2.jsonc > /tmp/const_prompt.txt
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 3 ]]; then
  usage
  exit 1
fi

legacy_path="$1"
author_handle="spo"

if [[ ! -f "$legacy_path" ]]; then
  echo "error: legacy file not found: $legacy_path" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 1
fi

suggested_output="${legacy_path%.json}.v2.jsonc"
if [[ $# -eq 2 ]]; then
  if [[ "$2" == *.jsonc || "$2" == *.json ]]; then
    suggested_output="$2"
  else
    author_handle="$2"
  fi
elif [[ $# -eq 3 ]]; then
  author_handle="$2"
  suggested_output="$3"
fi

node_name="$(jq -r '.data.node_id // empty' "$legacy_path")"
node_type="$(jq -r '.type // "native"' "$legacy_path")"
display_name="$(jq -r '.data.display_name // empty' "$legacy_path")"
description="$(jq -r '.data.description // empty' "$legacy_path")"
source_code="$(jq -r '.data.resources.source_code_url // empty' "$legacy_path")"

if [[ -z "$node_name" ]]; then
  echo "error: .data.node_id is missing in $legacy_path" >&2
  exit 1
fi

if [[ -z "$source_code" ]]; then
  source_code="<set-source-code-path>"
fi

schema_path="schema/node-v2.schema.json"
if [[ ! -f "$schema_path" ]]; then
  echo "error: schema not found: $schema_path" >&2
  exit 1
fi

cat <<EOF
You are converting a Space Operator legacy node definition to the V2 node-definition JSONC format.

Output requirements:
1. Return ONLY raw JSONC content (no markdown, no explanation).
2. Output must validate against schema/node-v2.schema.json.
3. Preserve runtime semantics and port names exactly.
4. Keep node identity/type stable:
   - type: "$node_type"
   - name: "$node_name"
5. Set metadata fields:
   - author_handle: "$author_handle"
   - source_code: "$source_code"
6. Use IValue-tagged defaults in 'config' when defaults exist.

Target output file path:
$suggested_output

Input metadata:
- node_id/name: $node_name
- type: $node_type
- display_name: ${display_name:-<empty>}
- description: ${description:-<empty>}

Field mapping rules (legacy -> V2):
- data.node_id -> name
- type -> type
- data.resources.source_code_url -> source_code (fallback to "$source_code")
- sources[] -> ports.outputs[]
  - sources[].name -> outputs[].name
  - sources[].type -> outputs[].type
  - sources[].optional -> outputs[].optional (default false)
- targets[] -> ports.inputs[]
  - targets[].name -> inputs[].name
  - targets[].type_bounds -> inputs[].type_bounds
  - targets[].required -> inputs[].required (default false)
  - targets[].passthrough -> inputs[].passthrough (default false)
- targets_form.json_schema -> config_schema

Additional construction rules:
- Do not include 'id' in node ports for backend authoring JSONC.
- version:
  - use data.version if present; otherwise "0.1"
- config:
  - store node config values directly under 'config' (no 'form_data', no 'ui_schema')
  - initialize 'config' using non-null target defaultValue values (if present), keyed by input name
  - if no defaults, use empty object '{}'

IValue tagging guide for 'config' values:
- string -> {"S":"..."}
- boolean -> {"B":true/false}
- unsigned integer -> {"U":"123"}
- signed integer -> {"I":"-1"}
- float/decimal -> {"D":"1.23"}
- null -> {"N":0}
- array -> {"A":[<IValue>, ...]}
- object/map -> {"M":{"k":<IValue>, ...}}

Legacy node definition JSON:
$(printf '\n```json\n')
$(cat "$legacy_path")
$(printf '\n```\n')

V2 schema (must satisfy):
$(printf '\n```json\n')
$(cat "$schema_path")
$(printf '\n```\n')
EOF
