-- Grant runtime access to vault-backed secrets for flow-backend.
--
-- Why:
-- flow-backend resolves vault-backed static inputs by joining
-- public.user_api_keys to vault.decrypted_secrets as the flow_runner role.
-- That role needs explicit access to the vault schema, decrypted view, and
-- pgsodium keyholder role. `public.user_api_keys` is granted conditionally
-- because older local stacks may not have that table yet.

-- flow-backend reads vault-backed secrets directly during execution.
-- The decrypted_secrets view calls pgsodium decrypt functions inline,
-- so flow_runner needs pgsodium_keyholder membership to decrypt.
GRANT USAGE ON SCHEMA vault TO flow_runner;
GRANT SELECT ON TABLE vault.decrypted_secrets TO flow_runner;
GRANT pgsodium_keyholder TO flow_runner;

DO $$
BEGIN
  IF to_regclass('public.user_api_keys') IS NOT NULL THEN
    EXECUTE 'GRANT SELECT ON TABLE public.user_api_keys TO flow_runner';
  END IF;
END
$$;
