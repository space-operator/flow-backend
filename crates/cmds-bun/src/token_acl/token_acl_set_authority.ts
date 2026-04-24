/**
 * Token ACL: rotate the MintConfig.authority.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getSetAuthorityInstruction,
  findMintConfigPda,
} from "@solana/token-acl-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclSetAuthority extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);
    const mint = toAddress(inputs.mint);
    const newAuthority = toAddress(inputs.new_authority);

    const [mintConfig] = await findMintConfigPda({ mint });

    const ix = getSetAuthorityInstruction({
      authority: authoritySigner,
      mintConfig,
      newAuthority,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclSetAuthority", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclSetAuthority(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
