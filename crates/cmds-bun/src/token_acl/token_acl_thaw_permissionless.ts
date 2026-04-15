/**
 * Token ACL: permissionless thaw. Anyone can call; gating program decides.
 *
 * Uses the idempotent variant by default (returns early if TA already
 * `Initialized`). Automatically resolves the thaw-extra-metas PDA + remaining
 * accounts declared by the gating program's TLV metadata.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  createThawPermissionlessIdempotentInstructionWithExtraMetas,
  createThawPermissionlessInstructionWithExtraMetas,
} from "@solana/token-acl-sdk";
import { fetchEncodedAccount } from "@solana/kit";
import {
  TOKEN_ACL_PROGRAM_ID,
  createRpc,
  signAndSendSingle,
  toAddress,
  toKitSigner,
} from "./token_acl_common.ts";

export default class TokenAclThawPermissionless extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const payerSigner = await toKitSigner(inputs.fee_payer);
    const authoritySigner = inputs.authority
      ? await toKitSigner(inputs.authority)
      : payerSigner;

    const mint = toAddress(inputs.mint);
    const tokenAccount = toAddress(inputs.token_account);
    const tokenAccountOwner = toAddress(inputs.token_account_owner);
    const idempotent = inputs.idempotent !== false;

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const { rpc } = createRpc(rpcUrl);
    const programAddress = toAddress(TOKEN_ACL_PROGRAM_ID);

    const accountRetriever = (addr: any) => fetchEncodedAccount(rpc, addr);

    const ix = idempotent
      ? await createThawPermissionlessIdempotentInstructionWithExtraMetas(
          authoritySigner,
          tokenAccount,
          mint,
          tokenAccountOwner,
          programAddress,
          accountRetriever,
        )
      : await createThawPermissionlessInstructionWithExtraMetas(
          authoritySigner,
          tokenAccount,
          mint,
          tokenAccountOwner,
          programAddress,
          accountRetriever,
        );

    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);
    return { signature };
  }
}

import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclThawPermissionless", () => {
    test("build", () => {
      const nd = { type: "bun", node_id: "t", inputs: [], outputs: [], config: {} } as any;
      expect(new TokenAclThawPermissionless(nd)).toBeInstanceOf(BaseCommand);
    });
  });
} catch (_) {}
