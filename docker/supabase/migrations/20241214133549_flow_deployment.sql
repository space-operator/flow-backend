CREATE TABLE flow_deployments (
    user_id UUID NOT NULL,
    id UUID NOT NULL,
    entrypoint INTEGER NOT NULL,
    start_permission TEXT NOT NULL
);

-- All wallets used in the deployment
CREATE TABLE flow_deployments_wallets (
    user_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    wallet_id BIGINT NOT NULL
);

-- All flows used in the deployment
CREATE TABLE flow_deployments_flows (
    user_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    flow_id INTEGER NOT NULL,
    nodes JSONB [] NOT NULL,
    edges JSONB [] NOT NULL,
    environment JSONB NOT NULL,
    current_network JSONB NOT NULL,
    instructions_bundling JSONB NOT NULL,
    is_public BOOLEAN NOT NULL,
    start_shared BOOLEAN NOT NULL,
    start_unverified BOOLEAN NOT NULL
);

CREATE TABLE flow_deployments_tags (flow_id INTEGER, tag TEXT, deployment_id UUID);