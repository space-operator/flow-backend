ALTER TABLE nodes ALTER COLUMN data SET DATA TYPE jsonb USING data::jsonb;
ALTER TABLE nodes ALTER COLUMN sources SET DATA TYPE jsonb USING sources::jsonb;
ALTER TABLE nodes ALTER COLUMN targets SET DATA TYPE jsonb USING targets::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.json_schema" SET DATA TYPE jsonb USING "targets_form.json_schema"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.ui_schema" SET DATA TYPE jsonb USING "targets_form.ui_schema"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.form_data" SET DATA TYPE jsonb USING "targets_form.form_data"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.extra" SET DATA TYPE jsonb USING "targets_form.extra"::jsonb;
