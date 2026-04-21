/**
 * Token ACL: create MintConfig PDA for a Token-2022 mint.
 *
 * Pre-requisite: the mint's freeze authority must already be set to the Token
 * ACL program's MintConfig PDA (or, if the mint's current freeze authority
 * signs this tx, `createConfig` will set it for them).
 *
 * After this runs, the Token ACL program acts as the mint's delegated freeze
 * authority, and the provided gating program decides who can permissionlessly
 * thaw / freeze token accounts.
 */
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getCreateConfigInstructionAsync,
  findMintConfigPda,
} from "@solana/token-acl-sdk";
import { TOKEN_2022_PROGRAM_ADDRESS } from "@solana-program/token-2022";
import {
  ABL_GATE_PROGRAM_ID,
  newSignerCache,
  signAndSendSingle,
  toAddress,
  toKitSigner,
} from "./token_acl_common.ts";
import type { Address } from "@solana/kit";

export default class TokenAclCreateConfig extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const signerCache = newSignerCache();
    const payerSigner = await toKitSigner(ctx, inputs.fee_payer, signerCache);
    const authoritySigner = await toKitSigner(ctx, inputs.authority, signerCache);

    const mint = toAddress(inputs.mint);
    const gatingProgram: Address = inputs.gating_program
      ? toAddress(inputs.gating_program)
      : toAddress(ABL_GATE_PROGRAM_ID);

    const ix = await getCreateConfigInstructionAsync({
      payer: payerSigner.address,
      authority: authoritySigner,
      mint,
      gatingProgram,
      tokenProgram: TOKEN_2022_PROGRAM_ADDRESS,
    });

    const [mintConfig] = await findMintConfigPda({ mint });

    const rpcUrl = inputs.rpc_url ?? "https://api.devnet.solana.com";
    const signature = await signAndSendSingle(rpcUrl, payerSigner, ix);

    return { signature, mint_config_pda: mintConfig };
  }
}

// ── Tests (only run under `bun test`) ─────────────────────────────────
import { test, expect, describe } from "bun:test";
try {
  describe("TokenAclCreateConfig", () => {
    test("build: class can be instantiated", () => {
      const nd = {
        type: "bun",
        node_id: "test",
        inputs: [],
        outputs: [],
        config: {},
      } as any;
      const cmd = new TokenAclCreateConfig(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects without keypair", async () => {
      const nd = {
        type: "bun",
        node_id: "test",
        inputs: [],
        outputs: [],
        config: {},
      } as any;
      const cmd = new TokenAclCreateConfig(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // not under `bun test`
}
