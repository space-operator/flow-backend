import type { FlowRunId, NodeId, User } from "./common.ts";
import type { PublicKey } from "@solana/web3.js";
import { Buffer, base64, bs58 } from "./deps.ts";

export interface CommandContext {
  flow_run_id: FlowRunId;
  node_id: NodeId;
  times: number;
  
}

export interface ServiceProxy {
  name: string;
  id: string;
  base_url: string;
}

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

export class Context {
  flow_owner: User;
  started_by: User;
  cfg: ContextConfig;
  environment: Record<string, string>;
  endpoints: Endpoints;
  command?: CommandContext;
  signer: ServiceProxy;

  constructor(data: ContextData) {
    this.flow_owner = data.flow_owner;
    this.started_by = data.started_by;
    this.cfg = data.cfg;
    this.environment = data.environment;
    this.endpoints = data.endpoints;
    this.command = data.command;
    this.signer = data.signer;
  }

  async requestSignature(
    pubkey: PublicKey,
    data: Buffer
  ): Promise<RequestSignatureResponse> {
    const resp = await fetch(new URL("call", this.signer.base_url), {
      method: "POST",
      body: JSON.stringify({
        envelope: "",
        svc_name: this.signer.name,
        svc_id: this.signer.id,
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
