create table nodes_v1 (
    id uuid,
    type text,
    name text,
    version text,
    inputs jsonb[],
    outputs jsonb[],
    instruction_info jsonb,
    display_name text,
    description text,
    tags text[]
);
