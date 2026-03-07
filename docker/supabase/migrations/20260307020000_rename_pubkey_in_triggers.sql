-- Update trigger functions to read 'pubkey' (no underscore) from raw_user_meta_data,
-- matching the key written by the Rust application code.

-- 1. validate_user: BEFORE INSERT whitelist check
CREATE OR REPLACE FUNCTION "auth"."validate_user"() RETURNS "trigger"
    LANGUAGE "plpgsql"
    AS $$
declare
myrec record;
begin
    select * into myrec from public.pubkey_whitelists
    where pubkey = new.raw_user_meta_data->>'pubkey' and pubkey is not null;
    if not found then
        raise exception 'pubkey is not in whitelists, %', new.raw_user_meta_data->>'pubkey';
    end if;

    return new;
end;
$$;

-- 2. handle_new_user: AFTER INSERT creates profile, quota, and wallet rows
CREATE OR REPLACE FUNCTION "public"."handle_new_user"() RETURNS "trigger"
LANGUAGE "plpgsql"
SECURITY DEFINER
AS $$
BEGIN
  INSERT INTO public.users_public
  (email, user_id, username, pub_key)
  VALUES (
    new.email,
    new.id,
    new.raw_user_meta_data->>'pubkey',
    new.raw_user_meta_data->>'pubkey'
  );

  INSERT INTO public.user_quotas (user_id) VALUES (new.id);

  INSERT INTO public.wallets (user_id, public_key, type, name, description)
  VALUES (new.id, new.raw_user_meta_data->>'pubkey', 'ADAPTER', 'Main wallet', 'Wallet used to sign up');

  RETURN new;
END;
$$;

-- 3. Also disable/enable the whitelist trigger during data import
CREATE OR REPLACE FUNCTION auth.disable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users DISABLE TRIGGER on_auth_user_created;
ALTER TABLE auth.users DISABLE TRIGGER on_auth_check_whitelists;
$$ SECURITY DEFINER;

CREATE OR REPLACE FUNCTION auth.enable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users ENABLE TRIGGER on_auth_user_created;
ALTER TABLE auth.users ENABLE TRIGGER on_auth_check_whitelists;
$$ SECURITY DEFINER;
