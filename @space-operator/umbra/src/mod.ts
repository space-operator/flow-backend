/**
 * Shared Umbra Privacy SDK helpers for Space Operator Deno nodes.
 *
 * Re-exports the SDK factory functions and provides a convenience
 * `createUmbraClient()` that builds an IUmbraClient from raw keypair bytes.
 */

import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  // Registration
  getUserRegistrationFunction,
  // Deposit
  getDirectDepositIntoEncryptedBalanceFunction,
  // Withdrawal
  getDirectWithdrawIntoPublicBalanceV3Function,
  // Query
  getQueryUserAccountFunction,
  getQueryEncryptedBalanceFunction,
  // Mixer — deposit side
  getCreateReceiverClaimableUtxoFromPublicBalanceFunction,
  // Mixer — claim side
  getFetchClaimableUtxosFunction,
  getClaimReceiverClaimableUtxoIntoEncryptedBalanceFunction,
} from "npm:@umbra-privacy/sdk";

export {
  // Re-export all SDK functions used by nodes
  getUserRegistrationFunction,
  getDirectDepositIntoEncryptedBalanceFunction,
  getDirectWithdrawIntoPublicBalanceV3Function,
  getQueryUserAccountFunction,
  getQueryEncryptedBalanceFunction,
  getCreateReceiverClaimableUtxoFromPublicBalanceFunction,
  getFetchClaimableUtxosFunction,
  getClaimReceiverClaimableUtxoIntoEncryptedBalanceFunction,
};

/** Supported Umbra network names. */
export type UmbraNetwork = "mainnet" | "devnet" | "localnet";

/** Default indexer endpoints per network. */
const INDEXER_ENDPOINTS: Record<string, string> = {
  // Per https://sdk.umbraprivacy.com/indexer/overview — mainnet only
  mainnet: "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com",
  // devnet: no public indexer yet (planned improvement per Umbra docs)
};

/**
 * Build an IUmbraClient from raw keypair bytes, network name, and RPC URL.
 *
 * @param keypairBytes 64-byte Solana keypair (secret key ∥ public key)
 * @param network      "mainnet" | "devnet" | "localnet"
 * @param rpcUrl       Solana JSON-RPC URL (e.g. "https://api.devnet.solana.com")
 */
export async function createUmbraClient(
  keypairBytes: Uint8Array,
  network: UmbraNetwork,
  rpcUrl: string,
) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);

  // Derive WSS URL from HTTPS URL for subscription support
  const rpcSubscriptionsUrl = rpcUrl
    .replace(/^https:\/\//, "wss://")
    .replace(/^http:\/\//, "ws://");

  const args: Record<string, any> = {
    signer,
    network,
    rpcUrl,
    rpcSubscriptionsUrl,
  };

  const indexer = INDEXER_ENDPOINTS[network];
  if (indexer) {
    args.indexerApiEndpoint = indexer;
  }

  const client = await getUmbraClientFromSigner(args as any);
  return client;
}
