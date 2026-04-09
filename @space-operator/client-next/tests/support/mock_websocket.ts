import type { WebSocketFactory, WebSocketLike } from "../../src/mod.ts";

export class MockWebSocket implements WebSocketLike {
  onopen: ((event?: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onclose: ((event: { code?: number; reason?: string }) => void) | null = null;
  readonly sent: string[] = [];
  closed = false;

  constructor(
    private readonly onSend: (
      socket: MockWebSocket,
      message: Record<string, unknown>,
    ) => void,
  ) {
    queueMicrotask(() => this.onopen?.({}));
  }

  send(data: string): void {
    this.sent.push(data);
    this.onSend(this, JSON.parse(data));
  }

  close(code?: number, reason?: string): void {
    if (this.closed) {
      return;
    }
    this.closed = true;
    queueMicrotask(() => this.onclose?.({ code, reason }));
  }

  serverSend(message: Record<string, unknown>) {
    queueMicrotask(() => {
      this.onmessage?.({ data: JSON.stringify(message) });
    });
  }

  serverError(error: unknown = new Error("mock websocket error")) {
    queueMicrotask(() => {
      this.onerror?.(error);
    });
  }
}

export function createMockWebSocketFactory(
  onSend: (
    socket: MockWebSocket,
    message: Record<string, unknown>,
  ) => void,
): WebSocketFactory {
  return () => new MockWebSocket(onSend);
}
