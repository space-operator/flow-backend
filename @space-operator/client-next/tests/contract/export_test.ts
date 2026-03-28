import { apiClient, contractTest } from "./_shared.ts";

contractTest("data export contract", async () => {
  const client = apiClient();
  await client.data.export();
});
