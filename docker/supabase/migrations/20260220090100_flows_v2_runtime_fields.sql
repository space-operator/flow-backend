-- Runtime field parity hardening for flows_v2.

alter table public.flows_v2
    add column if not exists current_network jsonb,
    add column if not exists start_shared boolean,
    add column if not exists start_unverified boolean,
    add column if not exists current_branch_id integer;

update public.flows_v2
set current_network = '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb
where current_network is null;

update public.flows_v2
set start_shared = false
where start_shared is null;

update public.flows_v2
set start_unverified = false
where start_unverified is null;

alter table public.flows_v2
    alter column current_network set default '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb,
    alter column current_network set not null,
    alter column start_shared set default false,
    alter column start_shared set not null,
    alter column start_unverified set default false,
    alter column start_unverified set not null;

comment on column public.flows_v2.current_network is 'Runtime network config used by backend start logic.';
comment on column public.flows_v2.start_shared is 'Allow authenticated shared starts (/start_shared).';
comment on column public.flows_v2.start_unverified is 'Allow unverified starts (/start_unverified).';
comment on column public.flows_v2.current_branch_id is 'Git-like branch pointer for editor/runtime integration.';
