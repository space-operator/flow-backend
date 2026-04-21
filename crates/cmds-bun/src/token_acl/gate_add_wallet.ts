/**
 * Token ACL Gate: add a wallet to an allow/block list.
 * Creates the per-wallet entry PDA that the gate's can-thaw/freeze check reads.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getAddWalletInstructionAsync } from "@solana/token-acl-gate-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclGateAddWallet extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);
    const listConfig = toAddress(inputs.list_config);
    const wallet = toAddress(inputs.wallet);

    const ix = await getAddWalletInstructionAsync({
      authority: authoritySigner,
      payer: payerSigner,
      listConfig,
      wallet,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclGateAddWallet", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclGateAddWallet(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
