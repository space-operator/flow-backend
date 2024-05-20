GRANT USAGE ON SCHEMA auth TO flow_runner;
GRANT SELECT ON ALL TABLES IN SCHEMA auth TO flow_runner;
GRANT SELECT ON ALL SEQUENCES IN SCHEMA auth TO flow_runner;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO flow_runner;
GRANT ALL ON TABLE auth.users TO flow_runner;
GRANT ALL ON TABLE auth.identities TO flow_runner;
