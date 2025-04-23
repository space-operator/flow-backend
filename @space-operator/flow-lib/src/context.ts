/**
 * Providing services and information about the current invocation for nodes to use.
 */

import type { FlowRunId, NodeId, User } from "./common.ts";
import { msgpack } from "./deps.ts";
import { base64, bs58, web3 } from "./deps.ts";
import { Value } from "./mod.ts";

export interface CommandContext {
  flow_run_id: FlowRunId;
  node_id: NodeId;
  times: number;
  svc: ServiceProxy;
}

export interface ContextProxy {
  data: CommandContextData;
  signer: ServiceProxy;
  execute: ServiceProxy;
  log: ServiceProxy;
}
export interface CommandContextData {
  node_id: string;
  times: number;
  flow: FlowContextData;
}
export interface FlowContextData {
  flow_run_id: FlowRunId;
  environment: Record<string, string>;
  set: FlowSetContextData;
}
export interface FlowSetContextData {
  flow_owner: User;
  started_by: User;
  endpoints: Endpoints;
  solana: SolanaClientConfig;
  http: HttpClientConfig;
}
export interface Endpoints {
  flow_server: string;
  supabase: string;
  supabase_anon_key: string;
}
export interface SolanaClientConfig {
  url: string;
  cluster: SolanaNet;
}
export interface HttpClientConfig {
  timeout_in_secs: number;
  gzip: boolean;
}
export interface ServiceProxy {
  name: string;
  id: string;
  base_url: string;
}

export interface RequestSignatureResponse {
  signature: Uint8Array;
  new_message?: Uint8Array;
}

export interface ExecuteResponse {
  signature?: Uint8Array;
}

function isPubkey(x: web3.PublicKey | web3.Keypair): x is web3.PublicKey {
  return (x as any)._bn !== undefined;
}

export class Instructions {
  #data: {
    fee_payer: Uint8Array;
    signers: Uint8Array[];
    instructions: msgpack.ValueMap[];
  };

  constructor(
    feePayer: web3.PublicKey,
    signers: Array<web3.Keypair | web3.PublicKey>,
    instructions: web3.TransactionInstruction[],
  ) {
    this.#data = {
      fee_payer: feePayer.toBytes(),
      signers: signers.map((x) => {
        if (isPubkey(x)) {
          const bytes = new Uint8Array(64);
          bytes.set(x.toBytes(), 32);
          return bytes;
        } else {
          return x.secretKey;
        }
      }),
      instructions: instructions.map((i) => ({
        program_id: i.programId.toBytes(),
        accounts: i.keys.map((k: any) => ({
          pubkey: k.pubkey.toBytes(),
          is_signer: k.isSigner,
          is_writable: k.isWritable,
        })),
        data: new Uint8Array(i.data),
      })),
    };
  }

  encode(): string {
    return base64.encodeBase64(msgpack.encode(this.#data));
  }
}

/**
 * Providing services and information about the current invocation for nodes to use.
 */
export class Context {
  #data: ContextProxy;

  /**
   * Solana RPC client.
   */
  solana: web3.Connection;

  constructor(data: ContextProxy) {
    this.#data = data;
    this.solana = new web3.Connection(data.data.flow.set.solana.url);
  }

  /**
   * Owner of current flow.
   */
  get flow_owner(): User {
    return this.#data.data.flow.set.flow_owner;
  }

  /**
   * Who started the invocation.
   */
  get started_by(): User {
    return this.#data.data.flow.set.started_by;
  }

  /**
   * Environment variables.
   */
  get environment(): Record<string, string> {
    return this.#data.data.flow.environment;
  }

  /**
   * URLs to call other services.
   */
  get endpoints(): Endpoints {
    return this.#data.data.flow.set.endpoints;
  }

  /**
   * Context of the current node.
   */
  get command(): CommandContext {
    return {
      flow_run_id: this.#data.data.flow.flow_run_id,
      node_id: this.#data.data.node_id,
      times: this.#data.data.times,
      svc: this.#data.execute,
    };
  }

  /**
   * Request a signature from user.
   * The backend with automaticaly find out who is the owner of the specified public key.
   *
   * Message data should be a serialized Solana message, produced by
   * [Transaction.serializeMessage](https://solana-labs.github.io/solana-web3.js/classes/Transaction.html#serializeMessage)
   * or [Message.serialize](https://solana-labs.github.io/solana-web3.js/classes/Message.html#serialize).
   * We only support legacy transaction at the moment.
   *
   * @param pubkey Public key
   * @param data Message data
   * @returns Signature and the (optional) [updated transaction message](https://docs.phantom.app/developer-powertools/solana-priority-fees#how-phantom-applies-priority-fees-to-dapp-transactions)
   */
  async requestSignature(
    pubkey: web3.PublicKey,
    data: Uint8Array,
  ): Promise<RequestSignatureResponse> {
    const resp = await fetch(new URL("call", this.#data.signer.base_url), {
      method: "POST",
      body: JSON.stringify({
        envelope: "",
        svc_name: this.#data.signer.name,
        svc_id: this.#data.signer.id,
        input: {
          id: null,
          time: Date.now(),
          pubkey: pubkey.toBase58(),
          message: base64.encodeBase64(data),
          timeout: 60 * 2,
          flow_run_id: this.command?.flow_run_id,
          signatures: null,
        },
      }),
      headers: {
        "content-type": "application/json",
      },
    });

    const result = await resp.json();
    if (result.success === false) {
      throw new Error(String(result.data));
    }
    const output = result.data;
    const signature = bs58.decodeBase58(output.signature);
    const new_message = output.new_message
      ? base64.decodeBase64(output.new_message)
      : undefined;
    return {
      signature,
      new_message,
    };
  }

  async execute(
    instructions: Instructions,
    output: Record<string, any>,
  ): Promise<ExecuteResponse> {
    const svc = this.command?.svc;
    if (!svc) throw new Error("service not available");

    const resp = await fetch(new URL("call", svc.base_url), {
      method: "POST",
      body: JSON.stringify({
        envelope: "",
        svc_name: svc.name,
        svc_id: svc.id,
        input: {
          instructions: instructions.encode(),
          output: new Value(output).M!,
        },
      }),
      headers: {
        "content-type": "application/json",
      },
    });

    const result = await resp.json();
    if (result.success === false) {
      throw new Error(String(result.data));
    }
    const data = result.data;
    const signature = data.signature
      ? bs58.decodeBase58(data.signature)
      : undefined;
    return {
      signature,
    };
  }
}

export interface Endpoints {
  flow_server: string;
  supabase: string;
  supabase_anon_key: string;
}

export interface ContextConfig {
  http_client: HttpClientConfig;
  solana_client: SolanaClientConfig;
  environment: Record<string, string>;
  endpoints: Endpoints;
}

export interface HttpClientConfig {
  timeout_in_secs: number;
  gzip: boolean;
}

export interface SolanaClientConfig {
  url: string;
  cluster: SolanaNet;
}

export type SolanaNet = "devnet" | "testnet" | "mainnet-beta";
