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

List of available nodes:

1. "flow_input"

```json
{
    "$schema": "https://schema.spaceoperator.com/node-v2.schema.json",
    "type": "native",
    "name": "flow_input",
    "outputs": [
        {
            "name": "${flow input's name}",
            "type": "free",
            "required": true,
            "tooltip": "Input of flow, change the \"name\" value to change input's name",
            "value": "${ default value to use when user doesn't provide an input value when calling flow }"
        }
    ],
    "inputs": []
}
```

2. "addition"

```json
{
    "$schema": "https://schema.spaceoperator.com/node-v2.schema.json",
    "type": "native",
    "name": "addition",
    "outputs": [
        {
            "name": "output",
            "type": "free",
            "required": true,
            "tooltip": "result of a + b"
        }
    ],
    "inputs": [
        {
            "name": "a",
            "type_bounds": ["number"]
        },
        {
            "name": "b",
            "type_bounds": ["number"]
        }
    ]
}
```

3. "const"

```json
{
    "$schema": "https://schema.spaceoperator.com/node-v2.schema.json",
    "type": "native",
    "name": "const",
    "outputs": [
        {
            "name": "output",
            "type": "free",
            "required": true,
            "value": "${constant value}"
        }
    ],
    "inputs": []
}
```