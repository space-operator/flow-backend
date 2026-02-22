import { Context, Effect, Layer, Schema } from "effect";
import { encodeBase64 } from "@std/encoding/base64";
import { bs58, type IValue, Value, web3 } from "./deps.ts";
import { SpaceHttpClient } from "./HttpApi.ts";
import type { AuthTokenError, HttpApiError } from "./Errors.ts";
import type { FlowId, FlowRunId } from "./Schema/Common.ts";
import {
  type DeploymentSpecifier,
  DeployFlowOutput,
  formatDeploymentQuery,
  type StartDeploymentParams,
  StartDeploymentOutput,
  StartFlowOutput,
  type StartFlowParams,
  StartFlowSharedOutput,
  type StartFlowSharedParams,
  type StartFlowUnverifiedParams,
  StartFlowUnverifiedOutput,
  type StopFlowParams,
  StopFlowOutput,
  type SubmitSignatureParams,
  SubmitSignatureOutput,
} from "./Schema/Rest.ts";
import {
  SignatureRequest,
  SignatureRequestSchema,
} from "./Schema/Ws.ts";

export interface FlowServiceShape {
  readonly startFlow: (
    id: FlowId | string,
    params: StartFlowParams,
  ) => Effect.Effect<typeof StartFlowOutput.Type, HttpApiError | AuthTokenError>;

  readonly startFlowShared: (
    id: FlowId | string,
    params: StartFlowSharedParams,
  ) => Effect.Effect<typeof StartFlowSharedOutput.Type, HttpApiError | AuthTokenError>;

  readonly startFlowUnverified: (
    id: FlowId | string,
    publicKey: string,
    params: StartFlowUnverifiedParams,
  ) => Effect.Effect<typeof StartFlowUnverifiedOutput.Type, HttpApiError | AuthTokenError>;

  readonly stopFlow: (
    runId: FlowRunId | string,
    params: StopFlowParams,
  ) => Effect.Effect<typeof StopFlowOutput.Type, HttpApiError | AuthTokenError>;

  readonly getFlowOutput: (
    runId: FlowRunId | string,
    token?: string,
  ) => Effect.Effect<Value, HttpApiError | AuthTokenError>;

  readonly getSignatureRequest: (
    runId: FlowRunId | string,
    token?: string,
  ) => Effect.Effect<SignatureRequest, HttpApiError | AuthTokenError>;

  readonly submitSignature: (
    params: SubmitSignatureParams,
  ) => Effect.Effect<typeof SubmitSignatureOutput.Type, HttpApiError>;

  readonly signAndSubmitSignature: (
    req: SignatureRequest,
    publicKey: web3.PublicKey,
    signTransaction: (
      tx: web3.VersionedTransaction,
    ) => web3.VersionedTransaction | Promise<web3.VersionedTransaction>,
  ) => Effect.Effect<void, HttpApiError | Error>;

  readonly deployFlow: (
    id: FlowId | string,
  ) => Effect.Effect<string, HttpApiError | AuthTokenError>;

  readonly startDeployment: (
    spec: DeploymentSpecifier,
    params?: StartDeploymentParams,
    token?: string,
  ) => Effect.Effect<typeof StartDeploymentOutput.Type, HttpApiError | AuthTokenError>;

  readonly exportData: () => Effect.Effect<
    Record<string, unknown>,
    HttpApiError | AuthTokenError
  >;

  readonly importData: (
    data: unknown,
  ) => Effect.Effect<void, HttpApiError | AuthTokenError>;
}

export class FlowService extends Context.Tag("FlowService")<
  FlowService,
  FlowServiceShape
>() {}

// --- Helper: compare byte arrays ---

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

export const FlowServiceLive: Layer.Layer<
  FlowService,
  never,
  SpaceHttpClient
> = Layer.effect(
  FlowService,
  Effect.gen(function* () {
    const http = yield* SpaceHttpClient;

    return {
      startFlow: (id, params) =>
        http.post(`/flow/start/${id}`, params, StartFlowOutput),

      startFlowShared: (id, params) =>
        http.post(`/flow/start_shared/${id}`, params, StartFlowSharedOutput),

      startFlowUnverified: (id, publicKey, params) =>
        http.post(
          `/flow/start_unverified/${id}`,
          params,
          StartFlowUnverifiedOutput,
          { auth: false, customToken: publicKey },
        ),

      stopFlow: (runId, params) =>
        http.post(`/flow/stop/${runId}`, {
          // Server uses the "millies" spelling
          timeout_millies: params.timeout_millis,
        }, StopFlowOutput),

      getFlowOutput: (runId, token) =>
        http
          .get(
            `/flow/output/${runId}`,
            Schema.Unknown,
            token ? { auth: false, customToken: token } : undefined,
          )
          .pipe(Effect.map((json) => Value.fromJSON(json as IValue))),

      getSignatureRequest: (runId, token) =>
        http
          .get(
            `/flow/signature_request/${runId}`,
            SignatureRequestSchema,
            token ? { auth: false, customToken: token } : undefined,
          )
          .pipe(Effect.map((data) => new SignatureRequest(data))),

      submitSignature: (params) =>
        http.post("/signature/submit", params, SubmitSignatureOutput, {
          auth: false,
        }),

      signAndSubmitSignature: (req, publicKey, signTransaction) =>
        Effect.gen(function* () {
          const requestedPublicKey = new web3.PublicKey(req.pubkey);
          if (!publicKey.equals(requestedPublicKey)) {
            return yield* Effect.fail(
              new Error(
                `different public key:\nrequested: ${req.pubkey}\nwallet: ${publicKey.toBase58()}`,
              ),
            );
          }

          const tx = req.buildTransaction();
          const signerPosition = tx.message.staticAccountKeys.findIndex(
            (pk) => pk.equals(requestedPublicKey),
          );
          if (signerPosition === -1) {
            return yield* Effect.fail(
              new Error("pubkey is not in signers list"),
            );
          }

          const signedTx = yield* Effect.tryPromise({
            try: () => Promise.resolve(signTransaction(tx)),
            catch: (e) =>
              new Error(
                `signing failed: ${e instanceof Error ? e.message : String(e)}`,
              ),
          });

          const signature = signedTx.signatures[signerPosition];
          if (signature == null) {
            return yield* Effect.fail(new Error("signature is null"));
          }

          const before = tx.message.serialize();
          const after = signedTx.message.serialize();
          const new_msg = bytesEqual(before, after)
            ? undefined
            : encodeBase64(after);

          yield* http.post(
            "/signature/submit",
            {
              id: req.id,
              signature: bs58.encodeBase58(signature),
              new_msg,
            },
            SubmitSignatureOutput,
            { auth: false },
          );
        }),

      deployFlow: (id) =>
        http
          .post(`/flow/deploy/${id}`, {}, DeployFlowOutput)
          .pipe(Effect.map((r) => r.deployment_id as string)),

      startDeployment: (spec, params, token) =>
        http.post(
          `/deployment/start?${formatDeploymentQuery(spec)}`,
          params,
          StartDeploymentOutput,
          token ? { auth: false, customToken: token } : undefined,
        ),

      exportData: () =>
        http.post(
          "/data/export",
          undefined,
          Schema.Record({ key: Schema.String, value: Schema.Unknown }),
        ),

      importData: (data) =>
        http.postVoid("/data/import", data),
    };
  }),
);
