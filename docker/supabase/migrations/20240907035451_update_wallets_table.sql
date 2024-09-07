ALTER TABLE wallets
ALTER COLUMN public_key TYPE text,
ALTER COLUMN public_key SET NOT NULL;
