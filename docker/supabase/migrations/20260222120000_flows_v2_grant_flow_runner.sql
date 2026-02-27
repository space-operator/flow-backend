-- Grant flow_runner access to flows_v2.
-- The table was created by the flow2 frontend migration, which did not
-- include grants for the flow_runner role used by the Rust backend.

GRANT SELECT, INSERT, UPDATE, DELETE ON public.flows_v2 TO flow_runner;
GRANT USAGE, SELECT, UPDATE ON SEQUENCE public.flows_v2_id_seq TO flow_runner;
