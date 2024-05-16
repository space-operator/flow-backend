-- NOTE: change to your own passwords for production environments
\set pgpass `echo "$POSTGRES_PASSWORD"`
\set flow_runner_password `echo "$FLOW_RUNNER_PASSWORD"`;

ALTER USER authenticator WITH PASSWORD :'pgpass';
ALTER USER pgbouncer WITH PASSWORD :'pgpass';
ALTER USER supabase_auth_admin WITH PASSWORD :'pgpass';
ALTER USER supabase_functions_admin WITH PASSWORD :'pgpass';
ALTER USER supabase_storage_admin WITH PASSWORD :'pgpass';

CREATE ROLE "flow_runner" WITH INHERIT NOCREATEROLE CREATEDB LOGIN REPLICATION BYPASSRLS WITH PASSWORD :'flow_runner_password';
