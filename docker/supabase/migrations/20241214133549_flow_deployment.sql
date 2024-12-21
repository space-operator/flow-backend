CREATE TABLE flow_deployments (
    id UUID NOT NULL,
    user_id UUID NOT NULL,
    entrypoint INTEGER NOT NULL,
    start_permission JSONB NOT NULL,
    collect_instructions BOOL NOT NULL,
    action_identity TEXT NULL,
    action_config JSONB NULL,
    fees JSONB [] NOT NULL,
    PRIMARY KEY (id)
);

-- Wallet used in a deployment
CREATE TABLE flow_deployments_wallets (
    user_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    wallet_id BIGINT NOT NULL,
    PRIMARY KEY (deployment_id, wallet_id)
);

-- Flow used in a deployment
CREATE TABLE flow_deployments_flows (
    deployment_id UUID NOT NULL,
    flow_id INTEGER NOT NULL,
    user_id UUID NOT NULL,
    nodes JSONB [] NOT NULL,
    edges JSONB [] NOT NULL,
    environment JSONB NOT NULL,
    current_network JSONB NOT NULL,
    instructions_bundling JSONB NOT NULL,
    is_public BOOLEAN NOT NULL,
    start_shared BOOLEAN NOT NULL,
    start_unverified BOOLEAN NOT NULL,
    PRIMARY KEY (deployment_id, flow_id)
);

CREATE TABLE flow_deployments_tags (
    flow_id INTEGER,
    tag TEXT,
    deployment_id UUID,
    PRIMARY KEY (flow_id, tag, deployment_id)
);