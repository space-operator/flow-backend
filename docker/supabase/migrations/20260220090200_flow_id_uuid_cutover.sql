-- Convert flow_run.flow_id from INTEGER (flows.id) to UUID (flows_v2.uuid).

-- Ensure V2 rows exist for legacy flows so UUID foreign keys can be enforced.
insert into public.flows_v2 (
    uuid,
    user_id,
    name,
    description,
    slug,
    "isPublic",
    gg_marketplace,
    visibility_profile_id,
    nodes,
    edges,
    viewport,
    environment,
    guide,
    instructions_bundling,
    backend_endpoint,
    current_network,
    start_shared,
    start_unverified,
    parent_flow
)
select
    f.uuid,
    f.user_id,
    f.name,
    coalesce(f.description, ''::text),
    f.slug,
    f."isPublic",
    f.gg_marketplace,
    f.visibility_profile_id,
    to_jsonb(f.nodes),
    to_jsonb(f.edges),
    coalesce(f.viewport::jsonb, '{"x":0,"y":0,"zoom":1}'::jsonb),
    coalesce(f.environment, '{}'::jsonb),
    f.guide::jsonb,
    coalesce(f.instructions_bundling #>> '{}', 'Off'),
    f.backend_endpoint,
    f.current_network,
    f.start_shared,
    f.start_unverified,
    (select pf.uuid from public.flows pf where pf.id = f.parent_flow)
from public.flows f
where not exists (
    select 1 from public.flows_v2 v2 where v2.uuid = f.uuid
);

alter table public.flow_run
    add column if not exists flow_id_v2 uuid;

update public.flow_run fr
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where fr.flow_id = f.id
  and fr.flow_id_v2 is null;

do $$
begin
    if exists (
        select 1 from public.flow_run where flow_id_v2 is null
    ) then
        raise exception 'flow_id_v2 backfill incomplete in flow_run';
    end if;
end $$;

alter table public.flow_run drop constraint if exists "fk-flow_id";
drop index if exists public.idx_flow_run_flow_id;

alter table public.flow_run
    drop column flow_id;

alter table public.flow_run
    rename column flow_id_v2 to flow_id;

alter table public.flow_run
    alter column flow_id set not null;

alter table public.flow_run
    add constraint flow_run_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;

create index if not exists idx_flow_run_flow_id on public.flow_run(flow_id);
