import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  createPrivacyCashClient,
  toRecipientAddress,
} from "./privacy_cash_common.ts";
import type { PrivacyCash } from "privacycash";

type PrivacyCashWithdrawClient = Pick<PrivacyCash, "withdraw">;

export async function executePrivacyCashWithdraw(
  client: PrivacyCashWithdrawClient,
  amount: unknown,
  recipientInput: unknown,
): Promise<{ signature: string }> {
  const amountLamports = Number(amount);
  if (!Number.isFinite(amountLamports) || amountLamports <= 0) {
    throw new Error(`Invalid amount: ${amount}. Must be a positive number of lamports.`);
  }

  const recipient = toRecipientAddress(recipientInput);

  console.log(`Withdrawing ${amountLamports} lamports from Privacy Cash...`);
  console.log(`  recipient: ${recipient}`);

  const result = await client.withdraw({
    lamports: amountLamports,
    recipientAddress: recipient,
  });

  console.log("Withdraw complete:", JSON.stringify(result));

  return {
    signature: result.tx ?? String(result),
  };
}

export default class PrivacyCashWithdraw extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = createPrivacyCashClient(
      inputs.keypair,
      inputs.rpc_url,
    );
    return await executePrivacyCashWithdraw(client, inputs.amount, inputs.recipient);
  }
}

// ── Tests ───────
import { test, expect, describe } from "bun:test";
try {
  describe("PrivacyCashWithdraw", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashWithdraw(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashWithdraw(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });

    test("executePrivacyCashWithdraw: forwards lamports and recipient to the SDK", async () => {
      let received: unknown;
      const client = {
        async withdraw(args: unknown) {
          received = args;
          return { tx: "withdraw-signature" };
        },
      } satisfies PrivacyCashWithdrawClient;

      await expect(
        executePrivacyCashWithdraw(client, "456", {
          toBase58: () => "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh",
        }),
      ).resolves.toEqual({
        signature: "withdraw-signature",
      });
      expect(received).toEqual({
        lamports: 456,
        recipientAddress: "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh",
      });
    });
  });
} catch (_) {}
