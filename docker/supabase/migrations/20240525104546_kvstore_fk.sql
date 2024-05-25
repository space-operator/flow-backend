ALTER TABLE kvstore
ADD CONSTRAINT kvstore_user_id_store_name_fkey
FOREIGN KEY (user_id, store_name) REFERENCES kvstore_metadata (user_id, store_name)
ON DELETE CASCADE;

ALTER TABLE kvstore
ADD CONSTRAINT kvstore_user_id_fkey
FOREIGN KEY (user_id) REFERENCES auth.users (id)
ON DELETE CASCADE;

ALTER TABLE kvstore_metadata
ADD CONSTRAINT kvstore_metadata_user_id_user_quotas_fkey
FOREIGN KEY (user_id) REFERENCES user_quotas (user_id)
ON DELETE CASCADE;
