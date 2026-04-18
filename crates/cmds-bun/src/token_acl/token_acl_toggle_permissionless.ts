/**
 * Token ACL: toggle permissionless thaw / freeze flags on MintConfig.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getTogglePermissionlessInstructionsInstruction,
  findMintConfigPda,
} from "@solana/token-acl-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclTogglePermissionless extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(inputs.authority, signerCache);
    const mint = toAddress(inputs.mint);
    const thawEnabled = Boolean(inputs.thaw_enabled);
    const freezeEnabled = Boolean(inputs.freeze_enabled);

    const [mintConfig] = await findMintConfigPda({ mint });

    const ix = getTogglePermissionlessInstructionsInstruction({
      authority: authoritySigner,
      mintConfig,
      thawEnabled,
      freezeEnabled,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclTogglePermissionless", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclTogglePermissionless(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
