import { encodeBase58, encodeBase64, web3 } from "../deps.ts";
import type { SignatureRequest } from "../types.ts";

export interface SignAndSubmitSignatureOptions {
  publicKey: web3.PublicKey;
  signTransaction: (
    tx: web3.VersionedTransaction,
  ) => web3.VersionedTransaction | Promise<web3.VersionedTransaction>;
  logger?: (event: "transaction" | "signedTransaction", value: unknown) => void;
}

export interface SignAndSubmitMessageSignatureOptions {
  publicKey: web3.PublicKey;
  signMessage: (
    message: Uint8Array,
  ) =>
    | string
    | Uint8Array
    | ArrayBuffer
    | Promise<string | Uint8Array | ArrayBuffer>;
}

function equalBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) {
    return false;
  }
  return a.every((value, index) => value === b[index]);
}

function normalizeSignature(
  value: string | Uint8Array | ArrayBuffer,
): string {
  if (typeof value === "string") {
    return value;
  }
  const bytes = value instanceof ArrayBuffer ? new Uint8Array(value) : value;
  return encodeBase58(bytes);
}

export async function signAndSubmitSignature(
  signatures: {
    submit(input: {
      id: number;
      signature: string | Uint8Array | ArrayBuffer;
      new_msg?: string;
    }): Promise<unknown>;
  },
  request: SignatureRequest,
  options: SignAndSubmitSignatureOptions,
): Promise<void> {
  const requestedPublicKey = new web3.PublicKey(request.pubkey);
  if (!options.publicKey.equals(requestedPublicKey)) {
    throw new Error(
      `different public key: requested ${request.pubkey}, wallet ${options.publicKey.toBase58()}`,
    );
  }

  const transaction = request.buildTransaction();
  const signerPosition = transaction.message.staticAccountKeys.findIndex((
    key,
  ) => key.equals(requestedPublicKey));
  if (signerPosition === -1) {
    throw new Error("pubkey is not in signers list");
  }

  options.logger?.("transaction", transaction);
  const signedTransaction = await options.signTransaction(transaction);
  options.logger?.("signedTransaction", signedTransaction);

  const signature = signedTransaction.signatures[signerPosition];
  if (signature == null) {
    throw new Error("signature is null");
  }

  const before = Uint8Array.from(transaction.message.serialize());
  const after = Uint8Array.from(signedTransaction.message.serialize());
  const new_msg = equalBytes(before, after) ? undefined : encodeBase64(after);

  await signatures.submit({
    id: request.id,
    signature: encodeBase58(signature),
    new_msg,
  });
}

export async function signAndSubmitMessageSignature(
  signatures: {
    submit(input: {
      id: number;
      signature: string | Uint8Array | ArrayBuffer;
      new_msg?: string;
    }): Promise<unknown>;
  },
  request: SignatureRequest,
  options: SignAndSubmitMessageSignatureOptions,
): Promise<void> {
  const requestedPublicKey = new web3.PublicKey(request.pubkey);
  if (!options.publicKey.equals(requestedPublicKey)) {
    throw new Error(
      `different public key: requested ${request.pubkey}, wallet ${options.publicKey.toBase58()}`,
    );
  }

  const message = request.buildMessage();
  const signature = await options.signMessage(message);

  await signatures.submit({
    id: request.id,
    signature: normalizeSignature(signature),
  });
}

export { web3 };
