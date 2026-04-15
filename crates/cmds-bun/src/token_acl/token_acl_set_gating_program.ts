/**
 * Token ACL: change the MintConfig.gating_program.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getSetGatingProgramInstruction,
  findMintConfigPda,
} from "@solana/token-acl-sdk";
import { signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclSetGatingProgram extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const payerSigner = await toKitSigner(inputs.fee_payer);
    const authoritySigner = await toKitSigner(inputs.authority);
    const mint = toAddress(inputs.mint);
    const newGatingProgram = toAddress(inputs.new_gating_program);

    const [mintConfig] = await findMintConfigPda({ mint });

    const ix = getSetGatingProgramInstruction({
      authority: authoritySigner,
      mintConfig,
      newGatingProgram,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclSetGatingProgram", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclSetGatingProgram(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
