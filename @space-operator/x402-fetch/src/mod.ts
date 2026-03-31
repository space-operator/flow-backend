import {
  ChainIdToNetwork,
  evm,
  isMultiNetworkSigner,
  isSvmSignerWallet,
  type MultiNetworkSigner,
  type Network,
  PaymentRequirementsSchema,
  type Signer,
  type X402Config,
} from "x402/types";
import {
  createPaymentHeader,
  type PaymentRequirementsSelector,
  selectPaymentRequirements,
} from "x402/client";

type LegacyPaymentRequired = {
  x402Version?: number;
  resource?: {
    url?: string;
    description?: string;
    mimeType?: string;
  };
  accepts?: unknown[];
};

type LegacyPaymentRequirement = {
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

const SOLANA_CAIP2_TO_LEGACY_NETWORK: Record<string, Network> = {
  "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1": "solana-devnet",
  "solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z": "solana-devnet",
  "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp": "solana",
  "solana:devnet": "solana-devnet",
  "solana:testnet": "solana-devnet",
  "solana:mainnet": "solana",
  "solana:mainnet-beta": "solana",
};

function normalizeLegacyNetwork(network: unknown): unknown {
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

function normalizePaymentRequirement(
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
    if (requirement.outputSchema && typeof requirement.outputSchema === "object") {
      normalized.outputSchema = requirement.outputSchema;
    }
    if (requirement.extra !== null && requirement.extra !== undefined) {
      normalized.extra = requirement.extra;
    }

    return {
      ...normalized,
    };
  }

  return {
    ...requirement,
    network: normalizeLegacyNetwork(requirement.network),
  };
}

/**
 * Enables the payment of APIs using the x402 payment protocol.
 *
 * This function wraps the native fetch API to automatically handle 402 Payment Required responses
 * by creating and sending a payment header. It will:
 * 1. Make the initial request
 * 2. If a 402 response is received, parse the payment requirements
 * 3. Verify the payment amount is within the allowed maximum
 * 4. Create a payment header using the provided wallet client
 * 5. Retry the request with the payment header
 *
 * @param fetch - The fetch function to wrap (typically globalThis.fetch)
 * @param walletClient - The wallet client used to sign payment messages
 * @param maxValue - The maximum allowed payment amount in base units (defaults to 0.1 USDC)
 * @param paymentRequirementsSelector - A function that selects the payment requirements from the response
 * @param config - Optional configuration for X402 operations (e.g., custom RPC URLs)
 * @returns A wrapped fetch function that handles 402 responses automatically
 *
 * @example
 * ```typescript
 * const wallet = new SignerWallet(...);
 * const fetchWithPay = wrapFetchWithPayment(fetch, wallet);
 *
 * // With custom RPC configuration
 * const fetchWithPay = wrapFetchWithPayment(fetch, wallet, undefined, undefined, {
 *   svmConfig: { rpcUrl: "http://localhost:8899" }
 * });
 *
 * // Make a request that may require payment
 * const response = await fetchWithPay('https://api.example.com/paid-endpoint');
 * ```
 *
 * @throws {Error} If the payment amount exceeds the maximum allowed value
 * @throws {Error} If the request configuration is missing
 * @throws {Error} If a payment has already been attempted for this request
 * @throws {Error} If there's an error creating the payment header
 */
export function wrapFetchWithPayment(
  fetch: typeof globalThis.fetch,
  walletClient: Signer | MultiNetworkSigner,
  maxValue: bigint = BigInt(0.1 * 10 ** 6), // Default to 0.10 USDC
  paymentRequirementsSelector: PaymentRequirementsSelector =
    selectPaymentRequirements,
  config?: X402Config,
): typeof globalThis.fetch {
  return async (input: RequestInfo | URL, init?: RequestInit) => {
    const request = new Request(input, init);
    const cloned = request.clone();

    const response = await fetch(request);

    if (response.status !== 402) {
      return response;
    }

    if (request.headers.has("X-PAYMENT")) {
      throw new Error("Payment already attempted");
    }

    const paymentRequired = (await response.json()) as LegacyPaymentRequired;
    const x402Version = paymentRequired.x402Version ?? 1;
    const accepts = paymentRequired.accepts ?? [];
    const parsedPaymentRequirements = accepts.map((requirement) =>
      PaymentRequirementsSchema.parse(
        normalizePaymentRequirement(
          requirement,
          paymentRequired.resource,
          request.url,
        ),
      )
    );

    const network = isMultiNetworkSigner(walletClient)
      ? undefined
      : evm.isSignerWallet(walletClient as typeof evm.EvmSigner)
      ? ChainIdToNetwork[(walletClient as typeof evm.EvmSigner).chain?.id]
      : isSvmSignerWallet(walletClient)
      ? (["solana", "solana-devnet"] as Network[])
      : undefined;

    const selectedPaymentRequirements = paymentRequirementsSelector(
      parsedPaymentRequirements,
      network,
      "exact",
    );

    if (BigInt(selectedPaymentRequirements.maxAmountRequired) > maxValue) {
      throw new Error("Payment amount exceeds maximum allowed");
    }

    const paymentHeader = await createPaymentHeader(
      walletClient,
      x402Version,
      selectedPaymentRequirements,
      config,
    );

    cloned.headers.append("X-PAYMENT", paymentHeader);
    cloned.headers.append(
      "Access-Control-Expose-Headers",
      "X-PAYMENT-RESPONSE",
    );

    const secondResponse = await fetch(cloned);
    return secondResponse;
  };
}

export { decodeXPaymentResponse } from "x402/shared";
export {
  createSigner,
  type MultiNetworkSigner,
  type Signer,
  type X402Config,
} from "x402/types";
export { type PaymentRequirementsSelector } from "x402/client";
