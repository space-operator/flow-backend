create type x402network as enum (
    'base', 'base-sepolia',
    'solana', 'solana-devnet'
);

create table flow_x402_fees (
    user_id uuid references auth.users(id) on delete cascade,
    id bigserial primary key,
    flow_id integer not null references flows(id) on delete cascade,
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);
alter table flow_x402_fees enable row level security;

create table flow_deployments_x402_fees (
    user_id uuid references auth.users(id) on delete cascade,
    id bigserial primary key,
    deployment_id uuid not null references flow_deployments(id) on delete cascade,
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);

grant select on flow_deployments_x402_fees to flow_runner;
grant select on flow_x402_fees to flow_runner;

alter table flow_deployments_x402_fees enable row level security;
create policy "owner-select" on flow_deployments_x402_fees for select to authenticated using (auth.uid() = user_id);
create policy "owner-insert" on flow_deployments_x402_fees for insert to authenticated with check (auth.uid() = user_id);
create policy "owner-delete" on flow_deployments_x402_fees for delete to authenticated using (auth.uid() = user_id);
create policy "owner-update" on flow_deployments_x402_fees for update to authenticated using (auth.uid() = user_id);

alter table flow_x402_fees enable row level security;
create policy "owner-select" on flow_x402_fees for select to authenticated using (auth.uid() = user_id);
create policy "owner-insert" on flow_x402_fees for insert to authenticated with check (auth.uid() = user_id);
create policy "owner-delete" on flow_x402_fees for delete to authenticated using (auth.uid() = user_id);
create policy "owner-update" on flow_x402_fees for update to authenticated using (auth.uid() = user_id);
