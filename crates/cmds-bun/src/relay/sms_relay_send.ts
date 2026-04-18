import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { twilioSendSms } from "./relay_common.ts";

/**
 * Send an SMS via the Twilio API.
 *
 * Generic SMS sending node — works for any use case:
 * notifications, alerts, 2FA, relay, marketing, etc.
 * Use upstream Deno/Rhai nodes to construct the message
 * body for privacy relay patterns.
 */
export default class TwilioSendSms extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const { account_sid, auth_token, from, to, body, status_callback, media_url } = inputs;

    if (!account_sid) throw new Error("account_sid (Twilio Account SID) is required");
    if (!auth_token) throw new Error("auth_token (Twilio Auth Token) is required");
    if (!from) throw new Error("from phone number is required (e.g. +15551234567)");
    if (!to) throw new Error("to phone number is required");
    if (!body) throw new Error("body (message text) is required");

    console.log(`Sending SMS: ${from} -> ${to} (${body.length} chars)`);

    const result = await twilioSendSms({
      account_sid,
      auth_token,
      from,
      to,
      body,
      status_callback: status_callback || undefined,
      media_url: media_url ? (Array.isArray(media_url) ? media_url : [media_url]) : undefined,
    });

    console.log("SMS sent:", JSON.stringify(result));

    return {
      message_sid: result.sid,
      status: result.status,
      date_created: result.date_created,
    };
  }
}

// ── Tests ───────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";
try {
  describe("TwilioSendSms", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new TwilioSendSms(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
    });

    test("run: rejects with missing account_sid", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new TwilioSendSms(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow("account_sid");
    });
  });
} catch (_) {}
