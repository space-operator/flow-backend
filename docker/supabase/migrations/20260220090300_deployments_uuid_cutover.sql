-- Convert deployment flow identifiers from INTEGER to UUID.

alter table public.flow_deployments
    add column if not exists entrypoint_v2 uuid;

alter table public.flow_deployments_flows
    add column if not exists flow_id_v2 uuid;

alter table public.flow_deployments_tags
    add column if not exists entrypoint_v2 uuid;

update public.flow_deployments d
set entrypoint_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where d.entrypoint = f.id
  and d.entrypoint_v2 is null;

update public.flow_deployments_flows df
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where df.flow_id = f.id
  and df.flow_id_v2 is null;

update public.flow_deployments_tags t
set entrypoint_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where t.entrypoint = f.id
  and t.entrypoint_v2 is null;

do $$
begin
    if exists (select 1 from public.flow_deployments where entrypoint_v2 is null) then
        raise exception 'entrypoint_v2 backfill incomplete for flow_deployments';
    end if;
    if exists (select 1 from public.flow_deployments_flows where flow_id_v2 is null) then
        raise exception 'flow_id_v2 backfill incomplete for flow_deployments_flows';
    end if;
    if exists (select 1 from public.flow_deployments_tags where entrypoint_v2 is null) then
        raise exception 'entrypoint_v2 backfill incomplete for flow_deployments_tags';
    end if;
end $$;

drop trigger if exists flow_deployments_insert on public.flow_deployments;
drop trigger if exists flow_deployments_delete on public.flow_deployments;
drop function if exists public.flow_deployments_insert();
drop function if exists public.flow_deployments_delete();

alter table public.flow_deployments drop constraint if exists flow_deployments_id_entrypoint_key;
alter table public.flow_deployments_flows drop constraint if exists flow_deployments_flows_pkey;
alter table public.flow_deployments_tags drop constraint if exists flow_deployments_tags_pkey;
alter table public.flow_deployments_tags drop constraint if exists flow_deployments_tags_deployment_id_entrypoint_fkey;

alter table public.flow_deployments
    drop column entrypoint,
    rename column entrypoint_v2 to entrypoint;

alter table public.flow_deployments_flows
    drop column flow_id,
    rename column flow_id_v2 to flow_id;

alter table public.flow_deployments_tags
    drop column entrypoint,
    rename column entrypoint_v2 to entrypoint;

alter table public.flow_deployments
    alter column entrypoint set not null;

alter table public.flow_deployments_flows
    alter column flow_id set not null;

alter table public.flow_deployments_tags
    alter column entrypoint set not null;

alter table public.flow_deployments
    add constraint flow_deployments_id_entrypoint_key unique (id, entrypoint);

alter table public.flow_deployments_flows
    add primary key (deployment_id, flow_id);

alter table public.flow_deployments_flows
    add constraint flow_deployments_flows_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;

alter table public.flow_deployments_tags
    add primary key (entrypoint, tag);

alter table public.flow_deployments_tags
    add constraint flow_deployments_tags_deployment_id_entrypoint_fkey
    foreign key (deployment_id, entrypoint)
    references public.flow_deployments(id, entrypoint)
    on delete cascade;

create or replace function public.flow_deployments_insert()
returns trigger as
$$
begin
    insert into
        public.flow_deployments_tags(entrypoint, tag, deployment_id, user_id)
    values
        (new.entrypoint, 'latest', new.id, new.user_id)
    on conflict (entrypoint, tag)
    do update set deployment_id = new.id;
    return new;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_insert
after insert on public.flow_deployments
for each row execute function public.flow_deployments_insert();

create or replace function public.flow_deployments_delete()
returns trigger as
$$
begin
    insert into
        public.flow_deployments_tags(entrypoint, tag, deployment_id, user_id)
    (
        select
            entrypoint,
            'latest' as tag,
            id as deployment_id,
            user_id
        from public.flow_deployments
        where entrypoint = old.entrypoint
        order by id desc
        limit 1
    )
    on conflict (entrypoint, tag) do nothing;

    return old;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_delete
after delete on public.flow_deployments
for each row execute function public.flow_deployments_delete();
