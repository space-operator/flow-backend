import { assertEquals } from "@std/assert";
import { encodeBase58, encodeBase64 } from "../../src/deps.ts";
import {
  signAndSubmitMessageSignature,
  signAndSubmitSignature,
  web3,
} from "../../src/mod.ts";
import type { SubmitSignatureInput } from "../../src/types.ts";

function unitTest(name: string, fn: () => Promise<void>) {
  Deno.test({
    name,
    sanitizeOps: false,
    sanitizeResources: false,
    fn,
  });
}

unitTest(
  "signAndSubmitSignature sends new_msg when the signed message length changes",
  async () => {
    const publicKey = new web3.PublicKey("11111111111111111111111111111111");
    const before = new Uint8Array([1, 2, 3]);
    const after = new Uint8Array([1, 2, 3, 4]);
    const submitted: SubmitSignatureInput[] = [];

    const transaction = {
      message: {
        staticAccountKeys: [publicKey],
        serialize: () => before,
      },
      signatures: [new Uint8Array(64)],
    } as unknown as web3.VersionedTransaction;
    const signedTransaction = {
      message: {
        staticAccountKeys: [publicKey],
        serialize: () => after,
      },
      signatures: [new Uint8Array(64).fill(7)],
    } as unknown as web3.VersionedTransaction;

    await signAndSubmitSignature(
      {
        submit: async (input) => {
          submitted.push(input);
          return { success: true };
        },
      },
      {
        id: 42,
        pubkey: publicKey.toBase58(),
        buildTransaction() {
          return transaction;
        },
      } as unknown as Parameters<typeof signAndSubmitSignature>[1],
      {
        publicKey,
        signTransaction: async () => signedTransaction,
      },
    );

    assertEquals(submitted, [{
      id: 42,
      signature: encodeBase58(new Uint8Array(64).fill(7)),
      new_msg: encodeBase64(after),
    }]);
  },
);

unitTest(
  "signAndSubmitMessageSignature submits a base58 signature without new_msg",
  async () => {
    const publicKey = new web3.PublicKey("11111111111111111111111111111111");
    const message = new Uint8Array([5, 6, 7, 8]);
    const signature = new Uint8Array(64).fill(9);
    const submitted: SubmitSignatureInput[] = [];

    await signAndSubmitMessageSignature(
      {
        submit: async (input) => {
          submitted.push(input);
          return { success: true };
        },
      },
      {
        id: 43,
        pubkey: publicKey.toBase58(),
        buildMessage() {
          return message;
        },
      } as unknown as Parameters<typeof signAndSubmitMessageSignature>[1],
      {
        publicKey,
        signMessage: async (input) => {
          assertEquals(input, message);
          return signature;
        },
      },
    );

    assertEquals(submitted, [{
      id: 43,
      signature: encodeBase58(signature),
    }]);
  },
);
