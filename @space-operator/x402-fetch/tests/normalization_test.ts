import { assertEquals, assertFalse } from "jsr:@std/assert@^1.0.16";
import {
  normalizeLegacyNetwork,
  normalizePaymentRequirement,
} from "../src/internal.ts";

Deno.test("normalizeLegacyNetwork maps Solana CAIP-2 ids", () => {
  assertEquals(
    normalizeLegacyNetwork("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"),
    "solana-devnet",
  );
  assertEquals(
    normalizeLegacyNetwork("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"),
    "solana",
  );
  assertEquals(normalizeLegacyNetwork("base-sepolia"), "base-sepolia");
});

Deno.test("normalizePaymentRequirement converts backend v2 fields", () => {
  const normalized = normalizePaymentRequirement(
    {
      scheme: "exact",
      network: "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
      amount: "25000",
      payTo: "receiver-wallet",
      asset: "usdc",
      outputSchema: null,
      extra: { deployment_id: "dep-1" },
    },
    {
      url: "https://api.example.test/deployment/start",
      description: "paid deployment start",
      mimeType: "application/json",
    },
  ) as Record<string, unknown>;

  assertEquals(normalized.scheme, "exact");
  assertEquals(normalized.network, "solana-devnet");
  assertEquals(normalized.maxAmountRequired, "25000");
  assertEquals(normalized.resource, "https://api.example.test/deployment/start");
  assertEquals(normalized.description, "paid deployment start");
  assertEquals(normalized.mimeType, "application/json");
  assertEquals(normalized.payTo, "receiver-wallet");
  assertEquals(normalized.asset, "usdc");
  assertEquals(normalized.extra, { deployment_id: "dep-1" });
  assertFalse("outputSchema" in normalized);
});

Deno.test("normalizePaymentRequirement preserves legacy shape", () => {
  const normalized = normalizePaymentRequirement({
    scheme: "exact",
    network: "solana:mainnet-beta",
    maxAmountRequired: "1000",
    resource: "https://api.example.test/resource",
    payTo: "receiver-wallet",
    asset: "usdc",
  });

  assertEquals(normalized, {
    scheme: "exact",
    network: "solana",
    maxAmountRequired: "1000",
    resource: "https://api.example.test/resource",
    payTo: "receiver-wallet",
    asset: "usdc",
  });
});
