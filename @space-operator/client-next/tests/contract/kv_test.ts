import { assertEquals, assertRejects } from "@std/assert";
import { apiClient, contractTest, randomStoreName } from "./_shared.ts";

contractTest(
  "kv contract: create, write, read, delete items, and delete stores",
  async () => {
    const client = apiClient();
    const store = randomStoreName();

    try {
      assertEquals(await client.kv.createStore(store), { success: true });

      const firstWrite = await client.kv.write(store, "entry", {
        nested: { count: 1 },
        labels: ["a", "b"],
      });
      assertEquals(firstWrite.old_value, undefined);

      const readBack = await client.kv.read(store, "entry");
      assertEquals(readBack.toJSObject(), {
        nested: { count: 1 },
        labels: ["a", "b"],
      });

      const secondWrite = await client.kv.write(store, "entry", 42);
      assertEquals(secondWrite.old_value?.toJSObject(), {
        nested: { count: 1 },
        labels: ["a", "b"],
      });

      const deletedItem = await client.kv.deleteItem(store, "entry");
      assertEquals(deletedItem.old_value.toJSObject(), 42);

      await assertRejects(() => client.kv.read(store, "entry"));
      assertEquals(await client.kv.deleteStore(store), { success: true });
      await assertRejects(() => client.kv.deleteStore(store));
    } finally {
      await client.kv.deleteStore(store).catch(() => undefined);
    }
  },
);
