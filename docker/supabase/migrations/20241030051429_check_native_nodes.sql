ALTER TABLE nodes ADD CONSTRAINT native_check CHECK (
    type <> 'native' OR user_id IS NULL OR "isPublic" = FALSE
) NO INHERIT;
