-- Rename `nodes` to `node_definitions` to clarify that this table holds the
-- catalog of available node types, not inline flow node instances.

ALTER TABLE public.nodes RENAME TO node_definitions;

-- Add columns expected by the v2 export / query code.
ALTER TABLE public.node_definitions
    ADD COLUMN IF NOT EXISTS version       integer,
    ADD COLUMN IF NOT EXISTS ports         jsonb,
    ADD COLUMN IF NOT EXISTS config        jsonb,
    ADD COLUMN IF NOT EXISTS config_schema jsonb,
    ADD COLUMN IF NOT EXISTS author_handle text,
    ADD COLUMN IF NOT EXISTS is_published  boolean DEFAULT false;
