-- Convert flow_x402_fees.flow_id from INTEGER (flows.id) to UUID (flows_v2.uuid).

alter table public.flow_x402_fees
    add column if not exists flow_id_v2 uuid;

update public.flow_x402_fees x
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where x.flow_id = f.id
  and x.flow_id_v2 is null;

do $$
begin
    if exists (select 1 from public.flow_x402_fees where flow_id_v2 is null) then
        raise exception 'flow_id_v2 backfill incomplete for flow_x402_fees';
    end if;
end $$;

alter table public.flow_x402_fees
    drop constraint if exists flow_x402_fees_flow_id_fkey;

alter table public.flow_x402_fees
    drop column flow_id;
alter table public.flow_x402_fees
    rename column flow_id_v2 to flow_id;

alter table public.flow_x402_fees
    alter column flow_id set not null;

alter table public.flow_x402_fees
    add constraint flow_x402_fees_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;
