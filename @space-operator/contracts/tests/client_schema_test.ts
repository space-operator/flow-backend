import { assert, assertEquals, assertExists } from "@std/assert";
import {
  clientJsonSchemas,
  executeFlowResultEnvelopeSchema,
  flowInputsSchema,
  flowRunWireEventSchemas,
  iValueSchema,
} from "../src/mod.ts";

Deno.test("contracts parse core client envelopes", () => {
  const result = executeFlowResultEnvelopeSchema.parse({
    flowRunId: "run-1",
    status: "pending_signature",
    signature_request: {
      id: 1,
      time: "now",
      pubkey: "pubkey",
      message: "Zm9v",
      timeout: 30,
    },
    signing_url: "https://spaceoperator.com/sign?flow_run_id=run-1",
  });

  assertEquals(result.status, "pending_signature");
  assertEquals(result.signature_request?.id, 1);
});

Deno.test("contracts validate recursive IValue payloads", () => {
  const value = iValueSchema.parse({
    M: {
      ok: { B: true },
      nested: {
        A: [{ S: "hello" }, { D: "1" }],
      },
    },
  });

  assertExists(value);
});

Deno.test("contracts expose JSON schema exports", () => {
  const schema = clientJsonSchemas.executeFlowResultEnvelope;
  assertExists(schema);
  assertEquals(schema.type, "object");
});

Deno.test("contracts accept recursive flow inputs with plain JSON and typed values", () => {
  const inputs = flowInputsSchema.parse({
    plain: {
      nested: [{ ok: true }, null],
    },
    typed: {
      M: {
        fee_payer: {
          B6: "3BhJ6n9M2wD6j4pXfVd2a6wY6Y9goVh6GJ4m6qVFXwU7jYq3s1hH9JQhY5m6X7qk8bJd1qWz3eR4tYu5nF2ZpQ",
        },
      },
    },
  });

  assertExists(inputs);
});

Deno.test("contracts export recursive flow input JSON schema", () => {
  const schema = clientJsonSchemas.startFlowParams;
  const schemaText = JSON.stringify(schema);

  assert(schemaText.includes("\"B6\""));
  assert(schemaText.includes("\"additionalProperties\""));
  assert(!schemaText.includes("\"additionalProperties\":{}"));
});

Deno.test("contracts validate flow-finish websocket events", () => {
  const event = flowRunWireEventSchemas.FlowFinish.parse({
    stream_id: 1,
    event: "FlowFinish",
    data: {
      flow_run_id: "run-1",
      time: "now",
      not_run: [],
      output: { M: { ok: { B: true } } },
    },
  });

  assertEquals(event.data.flow_run_id, "run-1");
});
