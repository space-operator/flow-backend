create type x402network as enum (
    'base', 'base-sepolia',
    'solana', 'solana-devnet'
);

create table flow_x402_fees (
    id bigserial primary key,
    flow_id integer not null references flows(id),
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);
alter table flow_x402_fees enable row level security;

create table flow_deployments_x402_fees (
    id bigserial primary key,
    deployment_id uuid not null references flow_deployments(id),
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);
alter table flow_deployments_x402_fees enable row level security;
