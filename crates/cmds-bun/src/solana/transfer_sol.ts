import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { Instructions } from "@space-operator/flow-lib-bun/context";
import bs58 from "bs58";
import {
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";

type InstructionSigner = Keypair | PublicKey;

export function resolveTransferSignerInput(input: unknown): InstructionSigner {
  if (input instanceof Keypair || input instanceof PublicKey) {
    return input;
  }
  if (input instanceof Uint8Array) {
    if (input.length === 64) return Keypair.fromSecretKey(input);
    if (input.length === 32) return new PublicKey(input);
  }
  if (Array.isArray(input)) {
    if (input.length === 64) return Keypair.fromSecretKey(new Uint8Array(input));
    if (input.length === 32) return new PublicKey(input);
  }
  if (typeof input === "object" && input !== null) {
    const record = input as Record<string, unknown>;
    const secretKey = extractSecretKeyBytes(record);
    if (secretKey) {
      return Keypair.fromSecretKey(secretKey);
    }
    const publicKey = extractPublicKey(record);
    if (publicKey) {
      return publicKey;
    }
  }
  throw new Error(
    "transfer_sol_bun requires `sender` and `fee_payer` to be either a local Solana keypair or a Flow adapter wallet object with `public_key`.",
  );
}

export function encodeTransactionSignature(
  signature: Uint8Array | undefined,
): string | undefined {
  return signature ? bs58.encode(signature) : undefined;
}

function toSignerPubkey(signer: InstructionSigner): PublicKey {
  return signer instanceof Keypair ? signer.publicKey : signer;
}

export default class TransferSol extends BaseCommand {
  override async run(
    ctx: Context,
    inputs: {
      fee_payer?: unknown;
      sender: unknown;
      recipient: PublicKey;
      amount: number;
      submit?: boolean;
    },
  ): Promise<{ signature?: string }> {
    const senderSigner = resolveTransferSignerInput(inputs.sender);
    const senderPubkey = toSignerPubkey(senderSigner);
    const recipient = inputs.recipient;
    const lamports = inputs.amount;
    const submit = inputs.submit ?? true;

    console.log(
      `Transferring ${lamports} lamports to ${recipient.toBase58()}`,
    );

    const instruction = SystemProgram.transfer({
      fromPubkey: senderPubkey,
      toPubkey: recipient,
      lamports,
    });

    if (!submit) {
      return {};
    }

    // Build signers list, with optional separate fee payer
    const signers: Array<Keypair | PublicKey> = [];
    let feePayer = senderPubkey;

    if (inputs.fee_payer) {
      const feePayerSigner = resolveTransferSignerInput(inputs.fee_payer);
      const feePayerPubkey = toSignerPubkey(feePayerSigner);
      if (!feePayerPubkey.equals(senderPubkey)) {
        feePayer = feePayerPubkey;
        signers.push(feePayerSigner);
      }
    }
    signers.push(senderSigner);

    const instructions = new Instructions(feePayer, signers, [instruction]);

    const result = await ctx.execute(instructions, {});

    const signature = encodeTransactionSignature(result.signature);

    console.log("Transfer complete:", signature);

    return { signature };
  }
}

function extractSecretKeyBytes(record: Record<string, unknown>): Uint8Array | null {
  const nestedSecretKey = record.secretKey ??
    (record._keypair &&
        typeof record._keypair === "object"
      ? (record._keypair as Record<string, unknown>).secretKey
      : undefined);

  if (nestedSecretKey instanceof Uint8Array && nestedSecretKey.length === 64) {
    return nestedSecretKey;
  }
  if (Array.isArray(nestedSecretKey) && nestedSecretKey.length === 64) {
    return new Uint8Array(nestedSecretKey);
  }
  if (typeof nestedSecretKey === "object" && nestedSecretKey !== null) {
    return extractIndexedBytes(nestedSecretKey as Record<string, unknown>, 64);
  }

  return extractIndexedBytes(record, 64);
}

function extractPublicKey(record: Record<string, unknown>): PublicKey | null {
  const adapterPublicKey = extractRustWalletAdapterPublicKey(record);
  const candidate = adapterPublicKey ?? record;
  if (candidate instanceof PublicKey) {
    return candidate;
  }
  if (candidate instanceof Uint8Array && candidate.length === 32) {
    return new PublicKey(candidate);
  }
  if (Array.isArray(candidate) && candidate.length === 32) {
    return new PublicKey(candidate);
  }
  if (typeof candidate === "object" && candidate !== null) {
    const candidateRecord = candidate as Record<string, unknown>;
    if (typeof candidateRecord.S === "string" && candidateRecord.S.length > 0) {
      return new PublicKey(candidateRecord.S);
    }
    if (typeof candidateRecord.B3 === "string" && candidateRecord.B3.length > 0) {
      return new PublicKey(candidateRecord.B3);
    }
    if (typeof (candidate as { toBase58?: () => string }).toBase58 === "function") {
      return new PublicKey((candidate as { toBase58: () => string }).toBase58());
    }
    const indexed = extractIndexedBytes(candidateRecord, 32);
    if (indexed) {
      return new PublicKey(indexed);
    }
  }
  return null;
}

function extractRustWalletAdapterPublicKey(
  record: Record<string, unknown>,
): unknown {
  if (!("public_key" in record) || !("token" in record)) {
    return undefined;
  }
  return record.token === null || typeof record.token === "string"
    ? record.public_key
    : undefined;
}

function extractIndexedBytes(
  record: Record<string, unknown>,
  size: number,
): Uint8Array | null {
  if (!(String(0) in record) || !(String(size - 1) in record)) {
    return null;
  }
  const bytes = new Uint8Array(size);
  for (let i = 0; i < size; i++) {
    bytes[i] = Number(record[String(i)] ?? 0);
  }
  return bytes;
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("TransferSol", () => {
    test("build: class can be instantiated", () => {
      const nd = {
        type: "bun",
        node_id: "test",
        inputs: [],
        outputs: [],
        config: {},
      } as any;
      const cmd = new TransferSol(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("resolveTransferSignerInput: accepts adapter wallet objects", () => {
      const publicKey = Keypair.generate().publicKey;
      const signer = resolveTransferSignerInput({
        public_key: publicKey.toBytes(),
        token: null,
      });
      expect(toSignerPubkey(signer).toBase58()).toBe(publicKey.toBase58());
    });

    test("resolveTransferSignerInput: rejects non-Rust wallet-shaped plain objects", () => {
      const publicKey = Keypair.generate().publicKey;
      expect(() =>
        resolveTransferSignerInput({
          publicKey: publicKey.toBytes(),
        })
      ).toThrow("Flow adapter wallet object with `public_key`");
    });

    test("run: returns base58 transaction signatures", async () => {
      const nd = {
        type: "bun",
        node_id: "test",
        inputs: [],
        outputs: [],
        config: {},
      } as any;
      const cmd = new TransferSol(nd);
      const sender = Keypair.generate().publicKey;
      const recipient = Keypair.generate().publicKey;

      const result = await cmd.run(
        {
          async execute() {
            return { signature: new Uint8Array([1, 2, 3, 4]) };
          },
        } as Context,
        {
          sender: { public_key: sender.toBytes(), token: null },
          recipient,
          amount: 1,
        },
      );

      expect(result.signature).toBe(bs58.encode(new Uint8Array([1, 2, 3, 4])));
    });
  });
} catch (_) {
  // Not running under `bun test`
}
