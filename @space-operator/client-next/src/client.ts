import {
  type ApiKeysNamespace,
  createApiKeysNamespace,
} from "./api_keys/mod.ts";
import { type AuthNamespace, createAuthNamespace } from "./auth/mod.ts";
import { ClientCore } from "./internal/core.ts";
import { createDataNamespace, type DataNamespace } from "./data/mod.ts";
import {
  createDeploymentsNamespace,
  type DeploymentsNamespace,
} from "./deployments/mod.ts";
import { createEventsNamespace, type EventsNamespace } from "./events/mod.ts";
import { createFlowsNamespace, type FlowsNamespace } from "./flows/mod.ts";
import { WebSocketSession } from "./internal/transport/ws.ts";
import { createKvNamespace, type KvNamespace } from "./kv/mod.ts";
import {
  createServiceNamespace,
  type ServiceNamespace,
} from "./service/mod.ts";
import {
  createSignaturesNamespace,
  type SignaturesNamespace,
} from "./signatures/mod.ts";
import {
  createWalletsNamespace,
  type WalletsNamespace,
} from "./wallets/mod.ts";
import type { AuthStrategy, CreateClientOptions } from "./types.ts";

function resolveClientAuth(
  defaultAuth: AuthStrategy | undefined,
  options: { auth?: AuthStrategy | undefined },
): AuthStrategy | undefined {
  return Object.prototype.hasOwnProperty.call(options, "auth")
    ? options.auth
    : defaultAuth;
}

export class SpaceOperatorClient {
  readonly auth: AuthNamespace;
  readonly flows: FlowsNamespace;
  readonly deployments: DeploymentsNamespace;
  readonly events: EventsNamespace;
  readonly signatures: SignaturesNamespace;
  readonly wallets: WalletsNamespace;
  readonly apiKeys: ApiKeysNamespace;
  readonly kv: KvNamespace;
  readonly data: DataNamespace;
  readonly service: ServiceNamespace;

  constructor(private readonly core: ClientCore) {
    this.auth = createAuthNamespace(core);
    this.flows = createFlowsNamespace(core);
    this.deployments = createDeploymentsNamespace(core);
    this.events = createEventsNamespace(core);
    this.signatures = createSignaturesNamespace(core);
    this.wallets = createWalletsNamespace(core);
    this.apiKeys = createApiKeysNamespace(core);
    this.kv = createKvNamespace(core);
    this.data = createDataNamespace(core);
    this.service = createServiceNamespace(core);
  }

  withAuth(auth: AuthStrategy): SpaceOperatorClient {
    return new SpaceOperatorClient(this.core.withAuth(auth));
  }

  ws(options: { auth?: AuthStrategy } = {}): WebSocketSession {
    return new WebSocketSession(
      this.core.config,
      resolveClientAuth(this.core.config.auth, options),
    );
  }
}

export function createClient(
  options: CreateClientOptions,
): SpaceOperatorClient {
  return new SpaceOperatorClient(new ClientCore(options));
}
