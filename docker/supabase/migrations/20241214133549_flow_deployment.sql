CREATE TABLE flow_deployments (
    id UUID NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    user_id UUID NOT NULL,
    entrypoint INTEGER NOT NULL,
    start_permission JSONB NOT NULL,
    output_instructions BOOL NOT NULL,
    action_identity TEXT NULL,
    action_config JSONB NULL,
    fees JSONB [] NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (id, entrypoint)
);

-- Wallet used in a deployment
CREATE TABLE flow_deployments_wallets (
    user_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    wallet_id BIGINT NOT NULL,
    PRIMARY KEY (deployment_id, wallet_id),
    FOREIGN KEY (user_id) REFERENCES auth.users (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id) REFERENCES flow_deployments (id) ON DELETE CASCADE
);

-- Flow used in a deployment
CREATE TABLE flow_deployments_flows (
    deployment_id UUID NOT NULL,
    flow_id INTEGER NOT NULL,
    user_id UUID NOT NULL,
    data JSONB NOT NULL,
    PRIMARY KEY (deployment_id, flow_id),
    FOREIGN KEY (user_id) REFERENCES auth.users (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id) REFERENCES flow_deployments (id) ON DELETE CASCADE
);

-- Tags to assign human-frienly references to flow deployments
CREATE TABLE flow_deployments_tags (
    entrypoint INTEGER NOT NULL,
    tag TEXT NOT NULL,
    deployment_id UUID NOT NULL,
    user_id UUID NOT NULL,
    PRIMARY KEY (entrypoint, tag),
    FOREIGN KEY (user_id) REFERENCES auth.users (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id) REFERENCES flow_deployments (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id, entrypoint) REFERENCES flow_deployments (id, entrypoint)
);

create or replace function flow_deployments_insert() returns trigger as $flow_deployments_insert$
begin
    insert into
    flow_deployments_tags(entrypoint,      tag,     deployment_id, user_id)
                   values(new.entrypoint, 'latest', new.id,        new.user_id)
    on conflict (entrypoint, tag)
    do update set deployment_id = new.id;
    return new;
end;
$flow_deployments_insert$
language plpgsql
security definer;

create trigger flow_deployments_insert after insert on flow_deployments
for each row execute function flow_deployments_insert();

GRANT SELECT, INSERT ON flow_deployments TO flow_runner;
GRANT SELECT, INSERT ON flow_deployments_wallets TO flow_runner;
GRANT SELECT, INSERT ON flow_deployments_flows TO flow_runner;
GRANT SELECT ON flow_deployments_tags TO flow_runner;

ALTER TABLE flow_deployments ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_deployments_wallets ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_deployments_flows ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_deployments_tags ENABLE ROW LEVEL SECURITY;

CREATE POLICY "owner-select" ON flow_deployments FOR SELECT TO authenticated USING (auth.uid() = user_id);
CREATE POLICY "owner-select" ON flow_deployments_wallets FOR SELECT TO authenticated USING (auth.uid() = user_id);
CREATE POLICY "owner-select" ON flow_deployments_flows FOR SELECT TO authenticated USING (auth.uid() = user_id);
CREATE POLICY "owner-select" ON flow_deployments_tags FOR SELECT TO authenticated USING (auth.uid() = user_id);
