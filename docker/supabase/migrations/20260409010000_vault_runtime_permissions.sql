-- Harden vault RPC execution and grant runtime read access for flow-backend.
--
-- Why:
-- 1. The vault RPCs are SECURITY DEFINER and trust the caller-supplied p_user_id.
--    They must not be executable by PUBLIC / anon / authenticated roles.
-- 2. flow-backend resolves vault-backed static inputs by joining
--    public.user_api_keys to vault.decrypted_secrets as the flow_runner role.
--    That role needs explicit access to both objects.

-- Ensure the SECURITY DEFINER functions run as postgres.
ALTER FUNCTION public.create_user_vault_secret(uuid, text, text, text, text)
  OWNER TO postgres;
ALTER FUNCTION public.update_user_vault_secret(uuid, uuid, text)
  OWNER TO postgres;
ALTER FUNCTION public.delete_user_vault_secret(uuid, uuid)
  OWNER TO postgres;

-- Lock down direct RPC execution. The Next.js API routes call these through the
-- service-role client after authenticating the user server-side.
REVOKE ALL ON FUNCTION public.create_user_vault_secret(uuid, text, text, text, text)
  FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.update_user_vault_secret(uuid, uuid, text)
  FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.delete_user_vault_secret(uuid, uuid)
  FROM PUBLIC, anon, authenticated;

GRANT EXECUTE ON FUNCTION public.create_user_vault_secret(uuid, text, text, text, text)
  TO service_role;
GRANT EXECUTE ON FUNCTION public.update_user_vault_secret(uuid, uuid, text)
  TO service_role;
GRANT EXECUTE ON FUNCTION public.delete_user_vault_secret(uuid, uuid)
  TO service_role;

-- flow-backend reads vault-backed secrets directly during execution.
-- The decrypted_secrets view calls pgsodium decrypt functions inline,
-- so flow_runner needs pgsodium_keyholder membership to decrypt.
GRANT USAGE ON SCHEMA vault TO flow_runner;
GRANT SELECT ON TABLE public.user_api_keys TO flow_runner;
GRANT SELECT ON TABLE vault.decrypted_secrets TO flow_runner;
GRANT pgsodium_keyholder TO flow_runner;
