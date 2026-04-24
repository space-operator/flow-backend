import { BaseCommand, Context } from "@space-operator/flow-lib-bun";

/**
 * Parse an inbound SMS webhook payload from Twilio.
 *
 * Generic inbound SMS parser — extracts sender, recipient,
 * body, and metadata from Twilio's webhook format.
 *
 * For privacy relay patterns, wire the `from` and `body`
 * outputs into a Deno/Rhai script that performs your
 * relay routing logic.
 */
export default class TwilioReceiveSms extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const { webhook_payload } = inputs;

    if (!webhook_payload) throw new Error("webhook_payload is required");

    const payload = typeof webhook_payload === "string"
      ? JSON.parse(webhook_payload)
      : webhook_payload;

    // Twilio webhook format (form-encoded, but may arrive as parsed JSON)
    const from = payload.From || payload.from || "";
    const to = payload.To || payload.to || "";
    const body = (payload.Body || payload.body || "").trim();
    const messageSid = payload.MessageSid || payload.message_sid || payload.SmsSid || "";
    const numMedia = parseInt(payload.NumMedia || "0", 10);
    const fromCity = payload.FromCity || "";
    const fromState = payload.FromState || "";
    const fromCountry = payload.FromCountry || "";

    // Collect media URLs if any (MMS)
    const mediaUrls: string[] = [];
    for (let i = 0; i < numMedia; i++) {
      const url = payload[`MediaUrl${i}`];
      if (url) mediaUrls.push(url);
    }

    return {
      from,
      to,
      body,
      message_sid: messageSid,
      num_media: numMedia,
      media_urls: mediaUrls,
      from_location: [fromCity, fromState, fromCountry].filter(Boolean).join(", "),
      received_at: new Date().toISOString(),
    };
  }
}

// ── Tests ───────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";
try {
  describe("TwilioReceiveSms", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new TwilioReceiveSms(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
    });

    test("run: parses Twilio webhook", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new TwilioReceiveSms(nd);
      const ctx = {} as Context;
      const result = await cmd.run(ctx, {
        webhook_payload: {
          From: "+15551234567",
          To: "+15559876543",
          Body: "Hello world",
          MessageSid: "SM1234567890",
          NumMedia: "0",
          FromCity: "San Francisco",
          FromState: "CA",
          FromCountry: "US",
        },
      });
      expect(result.from).toBe("+15551234567");
      expect(result.body).toBe("Hello world");
      expect(result.from_location).toBe("San Francisco, CA, US");
    });
  });
} catch (_) {}
