/**
 * Providing services and information about the current invocation for nodes to use.
 */

import type { FlowRunId, NodeId, User } from "./common.ts";
import { msgpack } from "./deps.ts";
import { Buffer, base64, bs58, web3 } from "./deps.ts";

export interface CommandContext {
  flow_run_id: FlowRunId;
  node_id: NodeId;
  times: number;
  svc: ServiceProxy;
}

export interface ServiceProxy {
  name: string;
  id: string;
  base_url: string;
}

/**
 * ContextData is Context when serialized and sent over the network.
 */
export interface ContextData {
  flow_owner: User;
  started_by: User;
  cfg: ContextConfig;
  environment: Record<string, string>;
  endpoints: Endpoints;
  command?: CommandContext;
  signer: ServiceProxy;
}

export interface RequestSignatureResponse {
  signature: Buffer;
  new_message?: Buffer;
}

export interface ExecuteResponse {
  signature?: Buffer;
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
    instructions: web3.TransactionInstruction[]
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
        accounts: i.keys.map((k) => ({
          pubkey: k.pubkey.toBytes(),
          is_signer: k.isSigner,
          is_writeable: k.isWritable,
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
  /**
   * Owner of current flow.
   */
  flow_owner: User;
  /**
   * Who started the invocation.
   */
  started_by: User;
  private _cfg: ContextConfig;
  /**
   * Environment variables.
   */
  environment: Record<string, string>;
  /**
   * URLs to call other services.
   */
  endpoints: Endpoints;
  /**
   * Context of the current node.
   */
  command?: CommandContext;
  /**
   * Solana RPC client.
   */
  solana: web3.Connection;

  private _signer: ServiceProxy;

  constructor(data: ContextData) {
    this.flow_owner = data.flow_owner;
    this.started_by = data.started_by;
    this._cfg = data.cfg;
    this.environment = data.environment;
    this.endpoints = data.endpoints;
    this.command = data.command;
    this._signer = data.signer;
    this.solana = new web3.Connection(this._cfg.solana_client.url);
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
   *
   * @param pubkey Public key
   * @param data Message data
   * @returns Signature and the (optional) [updated transaction message](https://docs.phantom.app/developer-powertools/solana-priority-fees#how-phantom-applies-priority-fees-to-dapp-transactions)
   */
  async requestSignature(
    pubkey: web3.PublicKey,
    data: Buffer
  ): Promise<RequestSignatureResponse> {
    const resp = await fetch(new URL("call", this._signer.base_url), {
      method: "POST",
      body: JSON.stringify({
        envelope: "",
        svc_name: this._signer.name,
        svc_id: this._signer.id,
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
    const signature = Buffer.from(bs58.decodeBase58(output.signature));
    const new_message = output.new_message
      ? Buffer.from(base64.decodeBase64(output.new_message))
      : undefined;
    return {
      signature,
      new_message,
    };
  }

  async execute(
    instrutions: Instructions,
    output: Record<string, any>
  ): Promise<ExecuteResponse> {
    const svc = this.command?.svc;
    if (!svc) throw new Error("service not available");

    const resp = await fetch(new URL("call", this._signer.base_url), {
      method: "POST",
      body: JSON.stringify({
        envelope: "",
        svc_name: svc.name,
        svc_id: svc.id,
        input: {
          instrutions: instrutions.encode(),
          output,
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
      ? Buffer.from(bs58.decodeBase58(data.signature))
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
