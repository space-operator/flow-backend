import { FlowRunId, NodeId, User } from "./common.ts";

export interface CommandContext {
  flow_run_id: FlowRunId;
  node_id: NodeId;
  times: number;
}

export interface Context {
  flow_owner: User;
  started_by: User;
  cfg: ContextConfig;
  environment: Record<string, string>;
  endpoints: Endpoints;
  command?: CommandContext;
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
