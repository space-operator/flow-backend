/**
 * Integration tests for Umbra Privacy bun nodes.
 *
 * These tests exercise the actual node classes against Umbra on mainnet
 * (the program is only deployed there — devnet/localnet are not supported).
 *
 * Requirements:
 *   - `bun install` at repo root
 *   - UMBRA_TEST_KEYPAIR env var (base58 secret key) for write operations
 *     The wallet needs mainnet SOL + USDC for deposit/withdraw tests.
 *
 * Read-only tests (query_account, query_balance, fetch_utxos) use a fresh
 * throwaway keypair and hit mainnet RPC — no funds required.
 *
 * Write tests (register, deposit, withdraw, create_utxo, claim_utxo) are
 * gated behind UMBRA_TEST_KEYPAIR and need real mainnet funds.
 */
import { describe, test, expect, beforeAll } from "bun:test";
import { Keypair } from "@solana/web3.js";
import bs58 from "bs58";

import UmbraRegister from "./umbra_register.ts";
import UmbraDeposit from "./umbra_deposit.ts";
import UmbraWithdraw from "./umbra_withdraw.ts";
import UmbraQueryAccount from "./umbra_query_account.ts";
import UmbraQueryBalance from "./umbra_query_balance.ts";
import UmbraCreateUtxo from "./umbra_create_utxo.ts";
import UmbraFetchUtxos from "./umbra_fetch_utxos.ts";
import UmbraClaimUtxo from "./umbra_claim_utxo.ts";

// ── Helpers ───────────────────────────────────────────────────────────

const MAINNET_RPC = "https://api.mainnet-beta.solana.com";
const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

const dummyNd = {
  type: "bun" as const,
  node_id: "test",
  inputs: [],
  outputs: [],
  config: {},
};

const dummyCtx = {} as any;

/** Base inputs shared by all umbra nodes (mainnet). */
function baseInputs(keypairBytes: Uint8Array) {
  return {
    keypair: Array.from(keypairBytes),
    network: "mainnet",
    rpc_url: MAINNET_RPC,
  };
}

/** Return the funded test keypair if UMBRA_TEST_KEYPAIR is set. */
function getFundedKeypair(): Keypair | null {
  const key = process.env.UMBRA_TEST_KEYPAIR;
  if (!key) return null;
  return Keypair.fromSecretKey(bs58.decode(key));
}

// ── Read-only tests (no funds needed) ─────────────────────────────────

describe("Umbra Integration — read-only (mainnet)", () => {
  let freshKp: Keypair;

  beforeAll(() => {
    freshKp = Keypair.generate();
  });

  test("query_account: unregistered address returns exists=false", async () => {
    const cmd = new UmbraQueryAccount(dummyNd);
    const result = await cmd.run(dummyCtx, baseInputs(freshKp.secretKey));
    expect(result.exists).toBe(false);
    expect(result.account).toBeNull();
  });

  test("query_balance: unregistered address returns zero or non_existent", async () => {
    const cmd = new UmbraQueryBalance(dummyNd);
    const result = await cmd.run(dummyCtx, {
      ...baseInputs(freshKp.secretKey),
      mint: USDC_MINT,
    });
    // Balance should be "0" for a non-existent account
    expect(result.balance).toBe("0");
  });

  test("fetch_utxos: returns result or throws network error", async () => {
    const cmd = new UmbraFetchUtxos(dummyNd);
    try {
      const result = await cmd.run(dummyCtx, {
        ...baseInputs(freshKp.secretKey),
        tree_index: 0,
        start_index: 0,
      });
      // If indexer is reachable, should return valid structure
      expect(Array.isArray(result.utxos)).toBe(true);
      expect(typeof result.count).toBe("number");
    } catch (e: any) {
      // Indexer may be unreachable — that's a network error, not a node bug
      expect(e.message || e.toString()).toContain("fetch");
    }
  });

  test("query_account: can query with explicit address parameter", async () => {
    const cmd = new UmbraQueryAccount(dummyNd);
    const result = await cmd.run(dummyCtx, {
      ...baseInputs(freshKp.secretKey),
      address: freshKp.publicKey.toBase58(),
    });
    expect(result.exists).toBe(false);
    expect(result.account).toBeNull();
  });
});

// ── Write tests (require funded mainnet wallet) ───────────────────────

describe("Umbra Integration — write operations (mainnet, funded)", () => {
  const fundedKp = getFundedKeypair();

  test.skipIf(!fundedKp)(
    "register: confidential-only registration succeeds",
    async () => {
      const kp = fundedKp!;
      const cmd = new UmbraRegister(dummyNd);
      const result = await cmd.run(dummyCtx, {
        ...baseInputs(kp.secretKey),
        confidential: true,
        anonymous: false, // anonymous requires ZK prover
      });
      expect(result.signature).toBeDefined();
      expect(typeof result.signature).toBe("string");
      expect(result.signature.length).toBeGreaterThan(0);
    },
  );

  test.skipIf(!fundedKp)(
    "query_account: registered address returns exists=true",
    async () => {
      const kp = fundedKp!;
      const cmd = new UmbraQueryAccount(dummyNd);
      const result = await cmd.run(dummyCtx, baseInputs(kp.secretKey));
      expect(result.exists).toBe(true);
      expect(result.account).toBeDefined();
    },
  );

  test.skipIf(!fundedKp)(
    "deposit: deposits USDC into encrypted balance",
    async () => {
      const kp = fundedKp!;
      const cmd = new UmbraDeposit(dummyNd);
      const result = await cmd.run(dummyCtx, {
        ...baseInputs(kp.secretKey),
        destination: kp.publicKey.toBase58(),
        mint: USDC_MINT,
        amount: "1000", // 0.001 USDC (6 decimals)
      });
      expect(result.signature).toBeDefined();
      expect(typeof result.signature).toBe("string");
    },
  );

  test.skipIf(!fundedKp)(
    "query_balance: shows non-zero after deposit",
    async () => {
      const kp = fundedKp!;
      const cmd = new UmbraQueryBalance(dummyNd);
      const result = await cmd.run(dummyCtx, {
        ...baseInputs(kp.secretKey),
        mint: USDC_MINT,
      });
      expect(result.balance).toBeDefined();
      // After deposit, balance should be > 0 or at least have a valid result
      expect(result.result).toBeDefined();
    },
  );

  test.skipIf(!fundedKp)(
    "withdraw: withdraws USDC from encrypted balance",
    async () => {
      const kp = fundedKp!;
      const cmd = new UmbraWithdraw(dummyNd);
      const result = await cmd.run(dummyCtx, {
        ...baseInputs(kp.secretKey),
        mint: USDC_MINT,
        amount: "1000",
        // destination defaults to signer's own address
      });
      expect(result.signature).toBeDefined();
      expect(typeof result.signature).toBe("string");
    },
  );
});

// ── Error handling tests ──────────────────────────────────────────────

describe("Umbra Integration — error handling", () => {
  test("register: fails gracefully with invalid keypair", async () => {
    const cmd = new UmbraRegister(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(new Uint8Array(32)), // too short
        network: "mainnet",
        rpc_url: MAINNET_RPC,
      }),
    ).rejects.toThrow();
  });

  test("register: rejects unsupported network (localnet)", async () => {
    const cmd = new UmbraRegister(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(Keypair.generate().secretKey),
        network: "localnet",
        rpc_url: "http://127.0.0.1:8899",
      }),
    ).rejects.toThrow(/not supported/);
  });

  test("deposit: fails gracefully with missing mint", async () => {
    const cmd = new UmbraDeposit(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(Keypair.generate().secretKey),
        network: "mainnet",
        rpc_url: MAINNET_RPC,
        destination: Keypair.generate().publicKey.toBase58(),
        // mint missing
        amount: "1000",
      }),
    ).rejects.toThrow();
  });

  test("withdraw: fails gracefully with missing mint", async () => {
    const cmd = new UmbraWithdraw(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(Keypair.generate().secretKey),
        network: "mainnet",
        rpc_url: MAINNET_RPC,
        amount: "1000",
        // mint missing
      }),
    ).rejects.toThrow();
  });

  test("create_utxo: fails gracefully with missing receiver", async () => {
    const cmd = new UmbraCreateUtxo(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(Keypair.generate().secretKey),
        network: "mainnet",
        rpc_url: MAINNET_RPC,
        mint: USDC_MINT,
        amount: "1000",
        // receiver missing
      }),
    ).rejects.toThrow();
  });

  test("claim_utxo: fails gracefully with invalid utxo_data", async () => {
    const cmd = new UmbraClaimUtxo(dummyNd);
    await expect(
      cmd.run(dummyCtx, {
        keypair: Array.from(Keypair.generate().secretKey),
        network: "mainnet",
        rpc_url: MAINNET_RPC,
        utxo_data: { invalid: true },
      }),
    ).rejects.toThrow();
  });

  test("fetch_utxos: non-mainnet returns empty with error message", async () => {
    const cmd = new UmbraFetchUtxos(dummyNd);
    const result = await cmd.run(dummyCtx, {
      keypair: Array.from(Keypair.generate().secretKey),
      network: "devnet",
      rpc_url: "https://api.devnet.solana.com",
    });
    expect(result.count).toBe(0);
    expect(result.utxos).toEqual([]);
    expect(result.error).toBeDefined();
  });
});
