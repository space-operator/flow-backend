import { Data } from "effect";

/** No authentication token configured. */
export class AuthTokenError extends Data.TaggedError("AuthTokenError")<{
  readonly message: string;
}> {}

/** HTTP error from the REST API. Preserves status code, URL, and body. */
export class HttpApiError extends Data.TaggedError("HttpApiError")<{
  readonly status: number;
  readonly url: string;
  readonly body: string;
  readonly message: string;
}> {}

/** WebSocket protocol error — the server returned an Err response. */
export class WsProtocolError extends Data.TaggedError("WsProtocolError")<{
  readonly method: string;
  readonly message: string;
}> {}

/** WebSocket connection dropped or failed to establish. */
export class WsConnectionError extends Data.TaggedError("WsConnectionError")<{
  readonly message: string;
}> {}

/** WebSocket request or connection timeout. */
export class WsTimeoutError extends Data.TaggedError("WsTimeoutError")<{
  readonly message: string;
}> {}
