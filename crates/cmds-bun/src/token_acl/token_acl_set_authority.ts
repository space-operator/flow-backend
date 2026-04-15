/**
 * Token ACL: rotate the MintConfig.authority.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getSetAuthorityInstruction,
  findMintConfigPda,
} from "@solana/token-acl-sdk";
import { signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclSetAuthority extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const payerSigner = await toKitSigner(inputs.fee_payer);
    const authoritySigner = await toKitSigner(inputs.authority);
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
