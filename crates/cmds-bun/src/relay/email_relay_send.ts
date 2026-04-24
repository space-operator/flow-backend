import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { resendSendEmail } from "./relay_common.ts";

/**
 * Send an email via the Resend API.
 *
 * Generic email sending node — works for any use case:
 * transactional, notifications, relay, marketing, etc.
 * Use upstream Deno/Rhai nodes to construct from/reply-to
 * addresses for privacy relay patterns.
 */
export default class ResendSendEmail extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const { api_key, from, to, reply_to, subject, text, html, cc, bcc } = inputs;

    if (!api_key) throw new Error("api_key (Resend API key) is required");
    if (!from) throw new Error("from email address is required");
    if (!to) throw new Error("to email address is required");
    if (!subject) throw new Error("subject is required");
    if (!text && !html) throw new Error("text or html body is required");

    console.log(`Sending email: ${from} -> ${to}`);

    const result = await resendSendEmail({
      api_key,
      from,
      to,
      reply_to: reply_to || undefined,
      subject,
      text: text || undefined,
      html: html || undefined,
      cc: cc ? (Array.isArray(cc) ? cc : [cc]) : undefined,
      bcc: bcc ? (Array.isArray(bcc) ? bcc : [bcc]) : undefined,
    });

    console.log("Email sent:", JSON.stringify(result));

    return {
      message_id: result.id,
      status: "sent",
    };
  }
}

// ── Tests ───────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";
try {
  describe("ResendSendEmail", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new ResendSendEmail(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing api_key", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new ResendSendEmail(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow("api_key");
    });

    test("run: rejects with missing body", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new ResendSendEmail(nd);
      const ctx = {} as Context;
      await expect(
        cmd.run(ctx, { api_key: "k", from: "a@b.c", to: "d@e.f", subject: "Hi" })
      ).rejects.toThrow("text or html");
    });
  });
} catch (_) {}
