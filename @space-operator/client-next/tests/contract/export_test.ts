import {
  contractTest,
  ownerBearerClient,
  RUN_EXPORT_TESTS,
} from "./_shared.ts";

contractTest("data export contract", async () => {
  const { client } = await ownerBearerClient();
  await client.data.export({ timeoutMs: 120_000 });
}, {
  ignore: !RUN_EXPORT_TESTS,
});
