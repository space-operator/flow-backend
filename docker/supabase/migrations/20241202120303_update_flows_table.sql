UPDATE flows SET nodes = '{}'::jsonb[] WHERE nodes IS NULL;
UPDATE flows SET edges = '{}'::jsonb[] WHERE edges IS NULL;
UPDATE flows SET environment = '{}'::jsonb WHERE environment IS NULL;
ALTER TABLE flows
ALTER COLUMN nodes SET DEFAULT '{}'::jsonb[], ALTER COLUMN nodes SET NOT NULL,
ALTER COLUMN edges SET DEFAULT '{}'::jsonb[], ALTER COLUMN edges SET NOT NULL,
ALTER COLUMN parent_flow TYPE INTEGER,
ALTER COLUMN environment SET DEFAULT '{}'::jsonb, ALTER COLUMN environment SET NOT NULL;
