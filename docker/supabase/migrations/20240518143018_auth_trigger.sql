CREATE OR REPLACE FUNCTION auth.disable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users DISABLE TRIGGER on_auth_user_created;
$$ SECURITY DEFINER;

GRANT EXECUTE ON FUNCTION auth.disable_users_triggers() to flow_runner;

CREATE OR REPLACE FUNCTION auth.enable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users ENABLE TRIGGER on_auth_user_created;
$$ SECURITY DEFINER;

GRANT EXECUTE ON FUNCTION auth.enable_users_triggers() to flow_runner;
