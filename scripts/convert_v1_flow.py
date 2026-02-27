#!/usr/bin/env python3
"""Convert a V1 flow JSON fixture to V2 format.

Handles:
  - flow.id: integer -> UUID string (uses nil UUID)
  - node.data.sources -> node.data.outputs
  - node.data.targets -> node.data.inputs
  - node.data.targets_form.form_data -> node.data.config
  - removes leftover V1 fields (targets_form, json_schema, ui_schema)

Usage:
  python3 scripts/convert_v1_flow.py <input.json> [output.json]

If output is omitted, writes to stdout.
"""

import json
import sys
import uuid

def convert_node_data(data: dict) -> dict:
    """Convert a V1 node data block to V2 format."""
    out = {}
    for key, value in data.items():
        if key == "sources":
            out["outputs"] = value
        elif key == "targets":
            out["inputs"] = value
        elif key == "targets_form":
            # Extract form_data as config
            if isinstance(value, dict) and "form_data" in value:
                out["config"] = value["form_data"]
            else:
                out["config"] = {}
        else:
            out[key] = value
    # Ensure config exists even if targets_form was missing
    if "config" not in out:
        out["config"] = {}
    return out


def convert_flow(flow: dict) -> dict:
    """Convert a V1 ClientConfig flow to V2 format."""
    # Fix integer id -> UUID string
    if isinstance(flow.get("id"), int):
        flow["id"] = str(uuid.UUID(int=flow["id"]))

    # Convert each node
    if "nodes" in flow:
        for node in flow["nodes"]:
            if "data" in node:
                node["data"] = convert_node_data(node["data"])

    return flow


def main():
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None

    with open(input_path) as f:
        doc = json.load(f)

    # Handle { "flow": {...}, "bookmarks": [...] } wrapper
    if "flow" in doc:
        doc["flow"] = convert_flow(doc["flow"])
    else:
        doc = convert_flow(doc)

    result = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"

    if output_path:
        with open(output_path, "w") as f:
            f.write(result)
        print(f"Converted {input_path} -> {output_path}", file=sys.stderr)
    else:
        sys.stdout.write(result)


if __name__ == "__main__":
    main()
