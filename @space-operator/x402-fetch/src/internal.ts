import type { Network } from "x402/types";

export type LegacyPaymentRequired = {
  x402Version?: number;
  resource?: {
    url?: string;
    description?: string;
    mimeType?: string;
  };
  accepts?: unknown[];
};

export type LegacyPaymentRequirement = {
  scheme?: unknown;
  network?: unknown;
  maxAmountRequired?: unknown;
  amount?: unknown;
  resource?: unknown;
  description?: unknown;
  mimeType?: unknown;
  outputSchema?: unknown;
  payTo?: unknown;
  maxTimeoutSeconds?: unknown;
  asset?: unknown;
  extra?: unknown;
};

export const SOLANA_CAIP2_TO_LEGACY_NETWORK: Record<string, Network> = {
  "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1": "solana-devnet",
  "solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z": "solana-devnet",
  "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp": "solana",
  "solana:devnet": "solana-devnet",
  "solana:testnet": "solana-devnet",
  "solana:mainnet": "solana",
  "solana:mainnet-beta": "solana",
};

export function normalizeLegacyNetwork(network: unknown): unknown {
  if (typeof network !== "string") {
    return network;
  }
  if (network in SOLANA_CAIP2_TO_LEGACY_NETWORK) {
    return SOLANA_CAIP2_TO_LEGACY_NETWORK[network];
  }
  if (network.startsWith("solana:")) {
    return "solana-devnet";
  }
  return network;
}

export function normalizePaymentRequirement(
  raw: unknown,
  fallbackResource?: LegacyPaymentRequired["resource"],
  requestUrl?: string,
): unknown {
  if (!raw || typeof raw !== "object") {
    return raw;
  }

  const requirement = raw as LegacyPaymentRequirement;
  if (typeof requirement.maxAmountRequired === "string") {
    return {
      ...requirement,
      network: normalizeLegacyNetwork(requirement.network),
    };
  }

  if (
    typeof requirement.amount === "string" &&
    typeof requirement.scheme === "string" &&
    typeof requirement.payTo === "string" &&
    typeof requirement.asset === "string"
  ) {
    const normalized: Record<string, unknown> = {
      scheme: requirement.scheme,
      network: normalizeLegacyNetwork(requirement.network),
      maxAmountRequired: requirement.amount,
      resource: typeof requirement.resource === "string"
        ? requirement.resource
        : fallbackResource?.url ?? requestUrl ?? "",
      description: typeof requirement.description === "string"
        ? requirement.description
        : fallbackResource?.description ?? "start flow deployment",
      mimeType: typeof requirement.mimeType === "string"
        ? requirement.mimeType
        : fallbackResource?.mimeType ?? "application/json",
      payTo: requirement.payTo,
      maxTimeoutSeconds: requirement.maxTimeoutSeconds,
      asset: requirement.asset,
    };
    if (
      requirement.outputSchema && typeof requirement.outputSchema === "object"
    ) {
      normalized.outputSchema = requirement.outputSchema;
    }
    if (requirement.extra !== null && requirement.extra !== undefined) {
      normalized.extra = requirement.extra;
    }

    return normalized;
  }

  return {
    ...requirement,
    network: normalizeLegacyNetwork(requirement.network),
  };
}
