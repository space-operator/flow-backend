ALTER TABLE wallets ADD COLUMN encrypted_keypair JSONB NULL;
UPDATE wallets SET encrypted_keypair['raw'] = to_json(keypair) WHERE keypair IS NOT NULL AND encrypted_keypair IS NULL;
