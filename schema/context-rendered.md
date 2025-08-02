space-operator flow is a visual programing programing platform for solana
blockchain. each flow is a directed acyclic graph, each node can have multiple
input and output ports.

input and output values of nodes are called "flow value" and follows the below
JSON schema:

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": "https://schema.spaceoperator.com/value.schema.json",
    "type": "object",
    "description": "Value format for inputs and outputs of nodes",
    "oneOf": [
        {
            "$ref": "#/definitions/N"
        },
        {
            "$ref": "#/definitions/S"
        },
        {
            "$ref": "#/definitions/B"
        },
        {
            "$ref": "#/definitions/U"
        },
        {
            "$ref": "#/definitions/I"
        },
        {
            "$ref": "#/definitions/U1"
        },
        {
            "$ref": "#/definitions/I1"
        },
        {
            "$ref": "#/definitions/F"
        },
        {
            "$ref": "#/definitions/D"
        },
        {
            "$ref": "#/definitions/B3"
        },
        {
            "$ref": "#/definitions/B6"
        },
        {
            "$ref": "#/definitions/BY"
        },
        {
            "$ref": "#/definitions/A"
        },
        {
            "$ref": "#/definitions/M"
        }
    ],
    "definitions": {
        "N": {
            "title": "Null",
            "description": "Null value",
            "type": "object",
            "properties": {
                "N": {
                    "const": 0
                }
            },
            "required": [
                "N"
            ],
            "additionalProperties": false
        },
        "S": {
            "title": "String",
            "description": "String value",
            "type": "object",
            "properties": {
                "S": {
                    "type": "string"
                }
            },
            "required": [
                "S"
            ],
            "additionalProperties": false
        },
        "B": {
            "title": "Boolean",
            "description": "Boolean value",
            "type": "object",
            "properties": {
                "B": {
                    "type": "boolean"
                }
            },
            "required": [
                "B"
            ],
            "additionalProperties": false
        },
        "U": {
            "title": "U64",
            "description": "Unsigned 64-bit integer",
            "type": "object",
            "properties": {
                "U": {
                    "type": "string"
                }
            },
            "required": [
                "U"
            ],
            "additionalProperties": false
        },
        "I": {
            "title": "I64",
            "description": "64-bit integer",
            "type": "object",
            "properties": {
                "I": {
                    "type": "string"
                }
            },
            "required": [
                "I"
            ],
            "additionalProperties": false
        },
        "U1": {
            "title": "U128",
            "description": "Unsigned 128-bit integer",
            "type": "object",
            "properties": {
                "U1": {
                    "type": "string"
                }
            },
            "required": [
                "U1"
            ],
            "additionalProperties": false
        },
        "I1": {
            "title": "I128",
            "description": "128-bit integer",
            "type": "object",
            "properties": {
                "I1": {
                    "type": "string"
                }
            },
            "required": [
                "I1"
            ],
            "additionalProperties": false
        },
        "F": {
            "title": "Float",
            "description": "64-bit floating-point number",
            "type": "object",
            "properties": {
                "F": {
                    "type": "string"
                }
            },
            "required": [
                "F"
            ],
            "additionalProperties": false
        },
        "D": {
            "title": "Decimal",
            "description": "Decimal using rust_decimal library",
            "type": "object",
            "properties": {
                "D": {
                    "type": "string"
                }
            },
            "required": [
                "D"
            ],
            "additionalProperties": false
        },
        "B3": {
            "title": "32-bytes",
            "description": "32-bytes binary value",
            "type": "object",
            "properties": {
                "B3": {
                    "type": "string"
                }
            },
            "required": [
                "B3"
            ],
            "additionalProperties": false
        },
        "B6": {
            "title": "64-bytes",
            "description": "64-bytes binary value",
            "type": "object",
            "properties": {
                "B6": {
                    "type": "string"
                }
            },
            "required": [
                "B6"
            ],
            "additionalProperties": false
        },
        "BY": {
            "title": "Bytes",
            "description": "Binary value",
            "type": "object",
            "properties": {
                "BY": {
                    "type": "string"
                }
            },
            "required": [
                "BY"
            ],
            "additionalProperties": false
        },
        "A": {
            "title": "Array",
            "description": "Array of values",
            "type": "object",
            "properties": {
                "A": {
                    "type": "array",
                    "items": {
                        "$ref": "#"
                    }
                }
            },
            "required": [
                "A"
            ],
            "additionalProperties": false
        },
        "M": {
            "title": "Map",
            "description": "Key-value map",
            "type": "object",
            "properties": {
                "M": {
                    "type": "object",
                    "patternProperties": {
                        "": {
                            "$ref": "#"
                        }
                    }
                }
            },
            "required": [
                "M"
            ],
            "additionalProperties": false
        }
    }
}
```

flow are defined by JSON and follows below schema:

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": "https://schema.spaceoperator.com/value.schema.json",
    "type": "object",
    "description": "JSON flow definition",
    "properties": {
        "name": {
            "type": "string",
            "description": "Display name of the flow"
        },
        "description": {
            "type": "string",
            "description": "Flow's description"
        },
        "nodes": {
            "type": "array",
            "description": "List of nodes",
            "items": {
                "$ref": "#/definitions/node"
            }
        },
        "edges": {
            "type": "array",
            "description": "List of edges",
            "items": {
                "$ref": "#/definitions/edge"
            }
        }
    },
    "required": [
        "name",
        "nodes",
        "edges"
    ],
    "definitions": {
        "node": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "Unique ID of this node, randomly generated UUIDv4"
                },
                "definition": {
                    "$ref": "https://schema.spaceoperator.com/node-v2.schema.json",
                    "description": "Node's definition"
                }
            }
        },
        "edge": {
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "ID of source node"
                },
                "sourceHandle": {
                    "type": "string",
                    "description": "Name of output port in source node"
                },
                "target": {
                    "type": "string",
                    "description": "ID of target node"
                },
                "targetHandle": {
                    "type": "string",
                    "description": "Name of input port in target node"
                }
            }
        }
    }
}
```

JSON schema for node definitions:

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": "https://schema.spaceoperator.com/node-v2.schema.json",
    "title": "Node Definition",
    "type": "object",
    "required": [
        "type",
        "name",
        "inputs",
        "outputs"
    ],
    "properties": {
        "type": {
            "title": "Node type",
            "type": "string",
            "enum": [
                "native",
                "WASM",
                "deno",
                "mock"
            ]
        },
        "name": {
            "title": "Unique name of node, name will be used to identify the node",
            "type": "string"
        },
        "display_name": {
            "title": "Display name of node",
            "type": "string"
        },
        "description": {
            "title": "Description",
            "type": "string"
        },
        "tags": {
            "title": "List of tags",
            "type": "array",
            "items": {
                "type": "string"
            }
        },
        "inputs": {
            "title": "Inputs",
            "type": "array",
            "items": {
                "$ref": "#/definitions/input"
            }
        },
        "outputs": {
            "title": "Outputs",
            "type": "array",
            "items": {
                "$ref": "#/definitions/output"
            }
        },
        "instruction_info": {
            "description": "Tell the flow graph this node will emit Solana instructions, and specify the order of outputs:\n- 'before': list of output names returned before instructions are sent.\n- 'signature': name of the signature output port.\n- 'after': list of output names returned after instructions are sent.\nNode only have to declare 'signature' and 'after', 'before' is the rest of the output.",
            "type": "object",
            "required": [
                "signature",
                "after"
            ],
            "properties": {
                "signature": {
                    "title": "Name of signature output",
                    "type": "string",
                    "default": "signature"
                },
                "after": {
                    "title": "Name of outputs that will be available after the signature output",
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "default": []
                }
            }
        }
    },
    "definitions": {
        "input": {
            "title": "Input port",
            "type": "object",
            "required": [
                "name",
                "type_bounds"
            ],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of this input port"
                },
                "type_bounds": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "required": {
                    "type": "boolean",
                    "default": true
                },
                "passthrough": {
                    "type": "boolean",
                    "description": "Passthrough input will also be available as output of the node",
                    "default": false
                },
                "defaultValue": {
                    "$ref": "https://schema.spaceoperator.com/value.schema.json",
                    "description": "Default value to use when this port is not connected to any edge"
                },
                "tooltip": {
                    "type": "string"
                }
            }
        },
        "output": {
            "title": "Output port",
            "type": "object",
            "required": [
                "name",
                "type"
            ],
            "properties": {
                "name": {
                    "type": "string"
                },
                "type": {
                    "type": "string"
                },
                "required": {
                    "type": "boolean",
                    "default": true
                },
                "tooltip": {
                    "type": "string"
                },
                "value": {
                    "$ref": "https://schema.spaceoperator.com/value.schema.json",
                    "description": "Hard-coded output value"
                }
            }
        }
    }
}
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