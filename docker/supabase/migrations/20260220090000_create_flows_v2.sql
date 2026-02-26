-- Canonical V2 flows table for scoped-node transport payloads.

create table if not exists public.flows_v2 (
    id integer primary key generated always as identity,
    uuid uuid not null default gen_random_uuid(),

    user_id uuid not null references auth.users(id) on delete cascade,

    name text not null default ''::text,
    description text not null default ''::text,
    slug text,

    "isPublic" boolean not null default false,
    gg_marketplace boolean not null default false,
    visibility_profile text,

    created_at timestamp without time zone not null default current_timestamp,
    updated_at timestamp without time zone not null default current_timestamp,

    -- Canonical V2 transport payloads.
    nodes jsonb not null default '[]'::jsonb,
    edges jsonb not null default '[]'::jsonb,
    viewport jsonb not null default '{"x":0,"y":0,"zoom":1}'::jsonb,

    environment jsonb not null default '{}'::jsonb,
    guide jsonb,
    instructions_bundling jsonb not null default '"Off"'::jsonb,
    backend_endpoint text,

    current_network jsonb not null default '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb,
    start_shared boolean not null default false,
    start_unverified boolean not null default false,
    current_branch_id integer,

    parent_flow uuid,
    linked_flows jsonb,
    lifecycle jsonb,

    meta_nodes jsonb not null default '[]'::jsonb,
    default_viewport jsonb not null default '{"x":0,"y":0,"zoom":1}'::jsonb
);

create unique index if not exists flows_v2_uuid_key on public.flows_v2 (uuid);
create unique index if not exists flows_v2_slug_key on public.flows_v2 (slug) where slug is not null;
create index if not exists idx_flows_v2_user_id on public.flows_v2 (user_id);
create index if not exists idx_flows_v2_is_public on public.flows_v2 ("isPublic");
create index if not exists idx_flows_v2_current_branch_id on public.flows_v2 (current_branch_id);
create index if not exists idx_flows_v2_nodes_gin on public.flows_v2 using gin (nodes);
create index if not exists idx_flows_v2_edges_gin on public.flows_v2 using gin (edges);

alter table public.flows_v2 enable row level security;

do $$
begin
    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-select'
    ) then
        create policy "owner-select" on public.flows_v2
            for select to authenticated using (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'public-select'
    ) then
        create policy "public-select" on public.flows_v2
            for select to anon using ("isPublic" = true);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-insert'
    ) then
        create policy "owner-insert" on public.flows_v2
            for insert to authenticated with check (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-update'
    ) then
        create policy "owner-update" on public.flows_v2
            for update to authenticated using (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-delete'
    ) then
        create policy "owner-delete" on public.flows_v2
            for delete to authenticated using (auth.uid() = user_id);
    end if;
end $$;

grant select, insert, update, delete on public.flows_v2 to authenticated;
grant select on public.flows_v2 to anon;
grant select, insert, update, delete on public.flows_v2 to flow_runner;
