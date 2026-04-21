/**
 * Token ACL: permissioned freeze. MintConfig.authority signs and freezes a TA.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getFreezeInstructionAsync } from "@solana/token-acl-sdk";
import { TOKEN_2022_PROGRAM_ADDRESS } from "@solana-program/token-2022";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclFreeze extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);
    const mint = toAddress(inputs.mint);
    const tokenAccount = toAddress(inputs.token_account);

    const ix = await getFreezeInstructionAsync({
      authority: authoritySigner,
      mint,
      tokenAccount,
      tokenProgram: TOKEN_2022_PROGRAM_ADDRESS,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclFreeze", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclFreeze(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
