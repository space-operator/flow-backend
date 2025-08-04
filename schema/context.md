space-operator flow is a visual programing programing platform for solana
blockchain. each flow is a directed acyclic graph, each node can have multiple
input and output ports.

input and output values of nodes are called "flow value" and follows the below
JSON schema:

```json
{{> value.schema.json }}
```

flow are defined by JSON and follows below schema:

```json
{{> flow.schema.json }}
```

JSON schema for node definitions:

```json
{{> node-v2.schema.json }}
```

Prefer using "value" field in "inputs" for constant value over "const" node.

List of available nodes:

1. "flow_input"

```jsonc
{{> nodes/flow_input.jsonc }}
```

2. "flow_output"

```jsonc
{{> nodes/flow_output.jsonc }}
```

3. "const"

```jsonc
{{> nodes/const.jsonc }}
```

4. "transfer_sol"

```jsonc
{{> nodes/transfer_sol.jsonc }}
```
