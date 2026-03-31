# x402-fetch

A utility package that extends the native `fetch` API to automatically handle
402 Payment Required responses using the x402 payment protocol. This package
enables seamless integration of payment functionality into your applications
when making HTTP requests.

## Compatibility Strategy

This package is also the compatibility boundary between the Flow Server's newer
x402 payment-requirements payloads and the currently published JavaScript `x402`
client packages.

Today the backend can return fields that do not match the schema the JS package
expects, including:

- CAIP-2 Solana chain ids such as `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`
- `amount` instead of `maxAmountRequired`
- resource metadata split across `resource`, `description`, and `mimeType`
- `outputSchema: null`

The wrapper normalizes that backend payload into the older JS schema before it
hands the requirements to `PaymentRequirementsSchema`.

In this package, `legacy` means the currently published JS `x402` schema shape,
not the legacy Space Operator client.

For now, this is the intended long-term boundary:

- `@space-operator/client-next` and the backend should speak the backend shape
- `@space-operator/x402-fetch` should adapt that shape to the current JS x402
  libraries

We should only remove this shim after the JS `x402` packages natively accept the
backend payload shape and the live x402 contract test passes without local
normalization.

## Installation

```bash
npm install x402-fetch
```

## Quick Start

```typescript
import { createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { wrapFetchWithPayment } from "x402-fetch";
import { baseSepolia } from "viem/chains";

// Create a wallet client
const account = privateKeyToAccount("0xYourPrivateKey");
const client = createWalletClient({
  account,
  transport: http(),
  chain: baseSepolia,
});

// Wrap the fetch function with payment handling
const fetchWithPay = wrapFetchWithPayment(fetch, client);

// Make a request that may require payment
const response = await fetchWithPay("https://api.example.com/paid-endpoint", {
  method: "GET",
});

const data = await response.json();
```

## API

### `wrapFetchWithPayment(fetch, walletClient, maxValue?, paymentRequirementsSelector?)`

Wraps the native fetch API to handle 402 Payment Required responses
automatically.

#### Parameters

- `fetch`: The fetch function to wrap (typically `globalThis.fetch`)
- `walletClient`: The wallet client used to sign payment messages (must
  implement the x402 wallet interface)
- `maxValue`: Optional maximum allowed payment amount in base units (defaults to
  0.1 USDC)
- `paymentRequirementsSelector`: Optional function to select payment
  requirements from the response (defaults to `selectPaymentRequirements`)

#### Returns

A wrapped fetch function that automatically handles 402 responses by:

1. Making the initial request
2. If a 402 response is received, parsing the payment requirements
3. Verifying the payment amount is within the allowed maximum
4. Creating a payment header using the provided wallet client
5. Retrying the request with the payment header

## Example

```typescript
import { config } from "dotenv";
import { createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { wrapFetchWithPayment } from "x402-fetch";
import { baseSepolia } from "viem/chains";

config();

const { PRIVATE_KEY, API_URL } = process.env;

const account = privateKeyToAccount(PRIVATE_KEY as `0x${string}`);
const client = createWalletClient({
  account,
  transport: http(),
  chain: baseSepolia,
});

const fetchWithPay = wrapFetchWithPayment(fetch, client);

// Make a request to a paid API endpoint
fetchWithPay(API_URL, {
  method: "GET",
})
  .then(async (response) => {
    const data = await response.json();
    console.log(data);
  })
  .catch((error) => {
    console.error(error);
  });
```
