/**
 * Token ACL: permissionless thaw. Anyone can call; gating program decides.
 *
 * Uses the idempotent variant by default (returns early if TA already
 * `Initialized`). Automatically resolves the thaw-extra-metas PDA + remaining
 * accounts declared by the gating program's TLV metadata.
 *
 * When `create_ata: true`, this bundles an ATA-create instruction before the
 * thaw using the SDK's `createTokenAccountWithAcl` helper — the holder's
 * self-onboarding path (one tx creates ATA and thaws it permissionlessly).
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  createThawPermissionlessIdempotentInstructionWithExtraMetas,
  createThawPermissionlessInstructionWithExtraMetas,
} from "@solana/token-acl-sdk";
import { fetchEncodedAccount } from "@solana/kit";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstruction,
  TOKEN_2022_PROGRAM_ADDRESS,
} from "@solana-program/token-2022";
import {
  TOKEN_ACL_PROGRAM_ID,
  createRpc,
  newSignerCache,
  signAndSendMany,
  signAndSendSingle,
  toAddress,
  toKitSigner,
} from "./token_acl_common.ts";

export default class TokenAclThawPermissionless extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(inputs.fee_payer, signerCache);
    const authoritySigner = inputs.authority
      ? await toKitSigner(inputs.authority, signerCache)
      : payerSigner;

    const mint = toAddress(inputs.mint);
    const tokenAccountOwner = toAddress(inputs.token_account_owner);
    const idempotent = inputs.idempotent !== false;
    const createAta = Boolean(inputs.create_ata);

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const { rpc } = createRpc(rpcUrl);
    const programAddress = toAddress(TOKEN_ACL_PROGRAM_ID);

    // Derive ATA if not supplied
    const tokenAccount = inputs.token_account
      ? toAddress(inputs.token_account)
      : (await findAssociatedTokenPda({
          mint,
          owner: tokenAccountOwner,
          tokenProgram: TOKEN_2022_PROGRAM_ADDRESS,
        }))[0];

    const accountRetriever = (addr: any) => fetchEncodedAccount(rpc, addr);

    if (createAta) {
      // Build the ATA-create instruction manually. We bypass the SDK's
      // createTokenAccountWithAcl helper because it requires the mint to
      // carry a TokenMetadata extension declaring `token_acl` — for the
      // minimum-viable ACL pilot we only require DefaultAccountState.
      const ataCreateIx = getCreateAssociatedTokenIdempotentInstruction({
        owner: tokenAccountOwner,
        mint,
        ata: tokenAccount,
        payer: payerSigner,
        tokenProgram: TOKEN_2022_PROGRAM_ADDRESS,
      });
      const thawIx =
        await createThawPermissionlessIdempotentInstructionWithExtraMetas(
          authoritySigner,
          tokenAccount,
          mint,
          tokenAccountOwner,
          programAddress,
          accountRetriever,
        );
      const signature = await signAndSendMany(rpcUrl, payerSigner, [
        ataCreateIx,
        thawIx,
      ]);
      return { signature, token_account: tokenAccount };
    }

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
    return { signature, token_account: tokenAccount };
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
