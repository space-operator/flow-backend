/**
 * Token ACL Gate: remove a wallet from an allow/block list by closing its
 * entry PDA.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getRemoveWalletInstruction,
  findWalletEntryPda,
} from "@solana/token-acl-gate-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclGateRemoveWallet extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);
    const listConfig = toAddress(inputs.list_config);
    const wallet = toAddress(inputs.wallet);

    const [walletEntry] = await findWalletEntryPda({ listConfig, wallet });

    const ix = getRemoveWalletInstruction({
      authority: authoritySigner,
      listConfig,
      walletEntry,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclGateRemoveWallet", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclGateRemoveWallet(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
