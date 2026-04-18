/**
 * Token ACL Gate: create an allow / allow-EOA / block list.
 * Mode: 0=Allow, 1=AllowAllEoas, 2=Block.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getCreateListInstructionAsync,
  findListConfigPda,
  Mode,
} from "@solana/token-acl-gate-sdk";
import { newSignerCache, signAndSendSingle, toAddress, toKitSigner } from "./token_acl_common.ts";

export default class TokenAclGateCreateList extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(inputs.authority, signerCache);
    const seed = toAddress(inputs.seed);
    const modeInput = inputs.mode ?? "Allow";
    const mode =
      typeof modeInput === "number"
        ? modeInput
        : (Mode as any)[modeInput] ?? Mode.Allow;

    const ix = await getCreateListInstructionAsync({
      authority: authoritySigner,
      payer: payerSigner,
      mode,
      seed,
    });

    const [listConfig] = await findListConfigPda({
      authority: authoritySigner.address,
      seed,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature, list_config: listConfig };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclGateCreateList", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclGateCreateList(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
