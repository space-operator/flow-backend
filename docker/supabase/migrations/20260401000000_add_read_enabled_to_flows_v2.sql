alter table public.flows_v2
    add column if not exists read_enabled boolean;

update public.flows_v2
set read_enabled = false
where read_enabled is null;

alter table public.flows_v2
    alter column read_enabled set default false,
    alter column read_enabled set not null;

comment on column public.flows_v2.read_enabled is 'Allow explicit snapshot read execution via /read endpoints.';
