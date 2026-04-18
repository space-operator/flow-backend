/**
 * Token ACL Gate: initialize the extra-metas PDAs for thaw/freeze so the
 * Token ACL program can resolve gate-specific accounts when calling can-thaw /
 * can-freeze. Must be called once per mint, passing the lists (allow + block)
 * the gate should consult.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getSetupExtraMetasInstruction } from "@solana/token-acl-gate-sdk";
import { findMintConfigPda } from "@solana/token-acl-sdk";
import { findThawExtraMetasAccountPda } from "@solana/token-acl-sdk";
import {
  newSignerCache,
  signAndSendSingle,
  toAddress,
  toKitSigner,
  ABL_GATE_PROGRAM_ID,
} from "./token_acl_common.ts";
import type { Address } from "@solana/kit";

export default class TokenAclGateSetupExtraMetas extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(inputs.authority, signerCache);
    const mint = toAddress(inputs.mint);

    const lists: Address[] = Array.isArray(inputs.lists)
      ? inputs.lists.map((v: any) => toAddress(v))
      : [];

    const [tokenAclMintConfig] = await findMintConfigPda({ mint });
    const [extraMetas] = await findThawExtraMetasAccountPda(
      { mint },
      { programAddress: toAddress(ABL_GATE_PROGRAM_ID) },
    );

    const ix = getSetupExtraMetasInstruction({
      authority: authoritySigner,
      payer: payerSigner,
      tokenAclMintConfig,
      mint,
      extraMetas,
      lists,
    });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature, extra_metas: extraMetas };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclGateSetupExtraMetas", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclGateSetupExtraMetas(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
