import { BaseCommand, Context } from "@space-operator/flow-lib-bun";

/**
 * Parse an inbound email webhook payload.
 *
 * Generic inbound email parser — works with Resend or SendGrid
 * inbound parse webhooks. Extracts sender, recipient, subject,
 * and body from the webhook payload.
 *
 * For privacy relay patterns, wire the `to` output into a
 * Deno/Rhai script that parses your relay address format.
 */
export default class ResendReceiveEmail extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const { webhook_payload } = inputs;

    if (!webhook_payload) throw new Error("webhook_payload is required");

    const payload = typeof webhook_payload === "string"
      ? JSON.parse(webhook_payload)
      : webhook_payload;

    // Normalize across Resend and SendGrid webhook formats
    const to = normalizeRecipient(payload.to);
    const from = payload.from || payload.sender || "";
    const subject = payload.subject || "";
    const textBody = payload.text || payload.plain || payload.TextBody || "";
    const htmlBody = payload.html || payload.HtmlBody || "";
    const headers = payload.headers || {};

    // Strip quoted reply content (keep only new message)
    const cleanBody = stripQuotedReply(textBody);

    return {
      from,
      to,
      subject,
      body: cleanBody,
      raw_body: textBody,
      html_body: htmlBody,
      headers,
      received_at: new Date().toISOString(),
    };
  }
}

/** Normalize recipient field (may be string, array, or array of objects). */
function normalizeRecipient(to: any): string {
  if (!to) return "";
  if (typeof to === "string") return to;
  if (Array.isArray(to)) {
    const first = to[0];
    if (typeof first === "string") return first;
    if (first?.address) return first.address;
    if (first?.email) return first.email;
    return String(first);
  }
  return String(to);
}

/** Strip common quoted reply markers from email body. */
function stripQuotedReply(text: string): string {
  const lines = text.split("\n");
  const cutoff = lines.findIndex(
    (line) =>
      (line.startsWith("On ") && line.includes(" wrote:")) ||
      line.startsWith(">") ||
      line.startsWith("---") ||
      line.startsWith("___") ||
      /^-{2,}\s*Original Message\s*-{2,}/i.test(line)
  );
  if (cutoff > 0) {
    return lines.slice(0, cutoff).join("\n").trim();
  }
  return text.trim();
}

// ── Tests ───────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";
try {
  describe("ResendReceiveEmail", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new ResendReceiveEmail(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
    });

    test("run: parses webhook payload", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new ResendReceiveEmail(nd);
      const ctx = {} as Context;
      const result = await cmd.run(ctx, {
        webhook_payload: {
          to: ["user@example.com"],
          from: "sender@test.com",
          subject: "Hello",
          text: "Message body\n\nOn Mon wrote:\n> Old message",
        },
      });
      expect(result.to).toBe("user@example.com");
      expect(result.from).toBe("sender@test.com");
      expect(result.body).toBe("Message body");
    });
  });
} catch (_) {}
