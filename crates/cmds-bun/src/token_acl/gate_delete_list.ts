/**
 * Token ACL Gate: delete a list (reclaims rent to authority).
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getDeleteListInstruction } from "@solana/token-acl-gate-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclGateDeleteList extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);
    const listConfig = toAddress(inputs.list_config);

    const ix = getDeleteListInstruction({
      authority: authoritySigner,
      listConfig,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclGateDeleteList", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclGateDeleteList(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
