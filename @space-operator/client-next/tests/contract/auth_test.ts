import { assert } from "@std/assert";
import {
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  serviceInfo,
  serviceSupabase,
  signText,
  web3,
} from "./_shared.ts";
import { createClient } from "../../src/mod.ts";
import { createClient as createSupabaseClient } from "@supabase/supabase-js";

contractTest(
  "auth contract: init + confirm returns a usable supabase session",
  async () => {
    const { decodeBase58 } = await import("../../src/deps.ts");
    const keypair = web3.Keypair.fromSecretKey(decodeBase58(getEnv("KEYPAIR")));
    const { anon_key, supabase_url } = await serviceInfo();
    const client = createClient({ baseUrl: FLOW_SERVER_URL });
    const msg = await client.auth.init(keypair.publicKey);
    const session = await client.auth.confirm(
      msg,
      await signText(keypair, msg),
    );

    const supabase = createSupabaseClient(supabase_url, anon_key, {
      auth: { autoRefreshToken: false },
    });
    await supabase.auth.setSession(session.session);
    const user = await supabase.auth.getUser();

    assert(user.error == null);
  },
);

contractTest(
  "auth contract: init + confirm can create a new user session",
  async () => {
    const keypair = web3.Keypair.generate();
    const client = createClient({ baseUrl: FLOW_SERVER_URL });
    const session = await client.auth.loginWithSignature(
      keypair.publicKey,
      (message) => signText(keypair, message),
    );

    const supabase = await serviceSupabase();
    await supabase.auth.setSession(session.session);
    const user = await supabase.auth.getUser();

    assert(user.error == null);
  },
);
