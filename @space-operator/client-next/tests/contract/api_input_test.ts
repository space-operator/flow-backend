import { assertEquals } from "@std/assert";
import {
  apiClient,
  contractTest,
  fixApiInputUrl,
  resolveFixtureFlowId,
  Value,
  withWebhookUrl,
} from "./_shared.ts";

contractTest("api input contract: subscribe and submit api input", async () => {
  const client = apiClient();
  const ws = client.ws();
  await ws.authenticate();
  const apiInputFlowId = await resolveFixtureFlowId("apiInput");
  const run = await client.flows.start(apiInputFlowId);
  const subscription = await ws.subscribeFlowRun(run.id);

  const worker = (async () => {
    for await (const event of subscription) {
      if (event.event === "ApiInput") {
        const response = await fetch(fixApiInputUrl(event.data.url), {
          method: "POST",
          headers: [["content-type", "application/json"]],
          body: JSON.stringify({ value: new Value("hello") }),
        });
        await response.text();
        break;
      }
    }
  })();

  const output = await run.output();
  await worker;
  await subscription.close();
  await ws.close();

  assertEquals(output.toJSObject().c, "hello");
});

contractTest(
  "api input contract: cancel still reports a node error",
  async () => {
    const client = apiClient();
    const ws = client.ws();
    await ws.authenticate();
    const apiInputFlowId = await resolveFixtureFlowId("apiInput");
    const run = await client.flows.start(apiInputFlowId);
    const subscription = await ws.subscribeFlowRun(run.id);
    let error: string | undefined;

    for await (const event of subscription) {
      if (event.event === "ApiInput") {
        const response = await fetch(fixApiInputUrl(event.data.url), {
          method: "DELETE",
        });
        await response.text();
      } else if (event.event === "NodeError") {
        error = event.data.error;
        break;
      }
    }

    await subscription.close();
    await ws.close();
    assertEquals(error, "canceled by user");
  },
);

contractTest(
  "api input contract: timeout still reports a node error",
  async () => {
    const client = apiClient();
    const ws = client.ws();
    await ws.authenticate();
    const apiInputFlowId = await resolveFixtureFlowId("apiInput");
    const run = await client.flows.start(apiInputFlowId);
    const subscription = await ws.subscribeFlowRun(run.id);
    let error: string | undefined;

    for await (const event of subscription) {
      if (event.event === "NodeError") {
        error = event.data.error;
        break;
      }
    }

    await subscription.close();
    await ws.close();
    assertEquals(error, "timeout");
  },
);

contractTest("api input contract: webhook mode still completes", async () => {
  await withWebhookUrl(async (webhookUrl) => {
    const client = apiClient();
    const ws = client.ws();
    await ws.authenticate();
    const apiInputFlowId = await resolveFixtureFlowId("apiInput");
    const run = await client.flows.start(apiInputFlowId, {
      inputs: {
        webhook_url: webhookUrl,
      },
    });
    const subscription = await ws.subscribeFlowRun(run.id);
    const output = await run.output({ timeoutMs: 30_000 });

    await subscription.close();
    await ws.close();
    assertEquals(output.toJSObject().c, "hello");
  });
});
