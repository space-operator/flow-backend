import { assertEquals } from "@std/assert";
import fc from "fast-check";
import { bearerAuth, flowRunTokenAuth } from "../../src/auth/mod.ts";
import { resolveAuthHeaders } from "../../src/internal/runtime.ts";
import { normalizeFlowValue } from "../../src/internal/transport/value.ts";

const safeNumberArbitrary = fc.double({
  noDefaultInfinity: true,
  noNaN: true,
}).filter((value) =>
  /^-?[0-9]+(e[0-9]+)?(\.[0-9]+)?$/.test(value.toString())
);

const flowSafeJsonValueArbitrary: fc.Arbitrary<unknown> = fc.letrec((tie) => ({
  value: fc.oneof(
    fc.constant(null),
    fc.boolean(),
    fc.string(),
    safeNumberArbitrary,
    fc.array(tie("value"), { maxLength: 4 }),
    fc.dictionary(fc.string(), tie("value"), { maxKeys: 4 }),
  ),
})).value;

Deno.test("normalizeFlowValue round-trips JSON-compatible values", async () => {
  await fc.assert(
    fc.asyncProperty(flowSafeJsonValueArbitrary, async (value) => {
      const normalized = normalizeFlowValue(value);
      assertEquals(normalized.toJSObject(), value);
    }),
  );
});

Deno.test("bearer-style auth headers normalize optional prefixes", async () => {
  await fc.assert(
    fc.asyncProperty(
      fc.hexaString({ minLength: 1, maxLength: 32 }),
      async (token) => {
        const bearerHeaders = await resolveAuthHeaders(
          bearerAuth(`Bearer ${token}`),
        );
        const flowRunHeaders = await resolveAuthHeaders(
          flowRunTokenAuth(`Bearer ${token}`),
        );

        assertEquals(bearerHeaders.get("authorization"), `Bearer ${token}`);
        assertEquals(flowRunHeaders.get("authorization"), `Bearer ${token}`);
      },
    ),
  );
});
