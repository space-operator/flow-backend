-- Baseline squash for brand-new databases only
-- Source: docker/supabase/migrations/*.sql (concatenated in lexical order)
-- NOTE: Existing migration history was not modified.


-- >>> BEGIN 20240514130738_init.sql
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;

CREATE EXTENSION IF NOT EXISTS "pg_net" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgsodium" WITH SCHEMA "pgsodium";

COMMENT ON SCHEMA "public" IS 'standard public schema';

CREATE EXTENSION IF NOT EXISTS "autoinc" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "http" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "moddatetime" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pg_graphql" WITH SCHEMA "graphql";

CREATE EXTENSION IF NOT EXISTS "pg_stat_statements" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgjwt" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "supabase_vault" WITH SCHEMA "vault";

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA "extensions";


CREATE OR REPLACE FUNCTION "public"."handle_new_user"() RETURNS "trigger"
    LANGUAGE "plpgsql" SECURITY DEFINER
    AS $$BEGIN
  INSERT INTO public.users_public
  (email, user_id, username, pub_key)
  VALUES (
    new.email,
    new.id,
    new.raw_user_meta_data->>'pub_key',
    new.raw_user_meta_data->>'pub_key'
  );

  INSERT INTO public.user_quotas (user_id) VALUES (new.id);

  INSERT INTO public.wallets (user_id, public_key, type, name, description)
  VALUES (new.id, new.raw_user_meta_data->>'pub_key', 'ADAPTER', 'Main wallet', 'Wallet used to sign up');

  RETURN new;
END;$$;

CREATE OR REPLACE FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) RETURNS bigint
    LANGUAGE "sql"
    AS $_$UPDATE user_quotas SET credit = credit + $2 WHERE user_id = $1 AND $2 >= 0 RETURNING credit;$_$;

CREATE OR REPLACE FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) RETURNS bigint
    LANGUAGE "sql"
    AS $_$UPDATE user_quotas SET used_credit = used_credit + $2 WHERE user_id = $1 AND $2 >= 0 AND used_credit + $2 <= credit RETURNING used_credit;$_$;

CREATE OR REPLACE FUNCTION "public"."is_nft_admin"("user_id" "uuid") RETURNS boolean
    LANGUAGE "sql" STABLE SECURITY DEFINER
    AS $_$SELECT EXISTS (SELECT user_id FROM nft_admins WHERE user_id = $1);$_$;

SET default_tablespace = '';

SET default_table_access_method = "heap";

CREATE TABLE IF NOT EXISTS "public"."apikeys" (
    "key_hash" "text" NOT NULL,
    "user_id" "uuid" NOT NULL,
    "name" "text" NOT NULL,
    "trimmed_key" "text" NOT NULL,
    "created_at" timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."flow_run" (
    "user_id" "uuid" NOT NULL,
    "id" "uuid" NOT NULL,
    "flow_id" integer NOT NULL,
    "start_time" timestamp without time zone,
    "end_time" timestamp without time zone,
    "not_run" "uuid"[],
    "output" "jsonb",
    "errors" "text"[],
    "inputs" "jsonb" NOT NULL,
    "environment" "jsonb" NOT NULL,
    "instructions_bundling" "jsonb" NOT NULL,
    "network" "jsonb" NOT NULL,
    "call_depth" integer NOT NULL,
    "origin" "jsonb" NOT NULL,
    "nodes" "jsonb"[] NOT NULL,
    "edges" "jsonb"[] NOT NULL,
    "collect_instructions" boolean NOT NULL,
    "partial_config" "jsonb",
    "signers" "jsonb" NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."flow_run_logs" (
    "user_id" "uuid" NOT NULL,
    "flow_run_id" "uuid" NOT NULL,
    "log_index" integer NOT NULL,
    "node_id" "uuid",
    "times" integer,
    "time" timestamp without time zone NOT NULL,
    "log_level" character varying(5) NOT NULL,
    "content" "text" NOT NULL,
    "module" "text"
);

CREATE TABLE IF NOT EXISTS "public"."flow_run_shared" (
    "flow_run_id" "uuid" NOT NULL,
    "user_id" "uuid" NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."flows" (
    "id" integer NOT NULL,
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "name" "text" DEFAULT ''::"text" NOT NULL,
    "isPublic" boolean DEFAULT false NOT NULL,
    "description" "text" DEFAULT 'Flow Description'::"text" NOT NULL,
    "tags" "text"[] DEFAULT '{}'::"text"[] NOT NULL,
    "created_at" "date" DEFAULT "now"() NOT NULL,
    "parent_flow" bigint,
    "viewport" "jsonb" DEFAULT '{"x": 524, "y": 268, "zoom": 0.5}'::"jsonb" NOT NULL,
    "uuid" "uuid" DEFAULT "extensions"."uuid_generate_v4"(),
    "updated_at" timestamp without time zone,
    "lastest_flow_run_id" "uuid",
    "custom_networks" "jsonb"[] DEFAULT '{}'::"jsonb"[] NOT NULL,
    "current_network" "jsonb" DEFAULT '{"id": "01000000-0000-8000-8000-000000000000", "url": "https://api.devnet.solana.com", "type": "default", "wallet": "Solana", "cluster": "devnet"}'::"jsonb" NOT NULL,
    "instructions_bundling" "jsonb" DEFAULT '"Off"'::"jsonb" NOT NULL,
    "guide" "jsonb",
    "environment" "jsonb",
    "nodes" "jsonb"[],
    "edges" "jsonb"[],
    "mosaic" "jsonb",
    "start_shared" boolean DEFAULT false NOT NULL,
    "start_unverified" boolean DEFAULT false NOT NULL
);

COMMENT ON COLUMN "public"."flows"."isPublic" IS 'To know if this flow is public or not';

COMMENT ON COLUMN "public"."flows"."parent_flow" IS 'This means the flow was cloned';

COMMENT ON COLUMN "public"."flows"."viewport" IS 'flow viewport';

ALTER TABLE "public"."flows" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."flows_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."kvstore" (
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "store_name" "text" NOT NULL,
    "key" "text" NOT NULL,
    "value" "jsonb" NOT NULL,
    "last_updated" timestamp without time zone DEFAULT "now"()
);

CREATE TABLE IF NOT EXISTS "public"."kvstore_metadata" (
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "store_name" "text" NOT NULL,
    "stats_size" bigint DEFAULT 0 NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."node_run" (
    "user_id" "uuid" NOT NULL,
    "flow_run_id" "uuid" NOT NULL,
    "node_id" "uuid" NOT NULL,
    "times" integer NOT NULL,
    "start_time" timestamp without time zone,
    "end_time" timestamp without time zone,
    "output" "jsonb",
    "errors" "text"[],
    "input" "jsonb" DEFAULT '{"M": {}}'::"jsonb" NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."nodes" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "name" "text" DEFAULT ''::"text",
    "user_id" "uuid" DEFAULT "auth"."uid"(),
    "type" "text" DEFAULT 'mock'::"text",
    "sources" "jsonb" DEFAULT '[]'::"jsonb" NOT NULL,
    "targets" "jsonb" DEFAULT '[]'::"jsonb" NOT NULL,
    "targets_form.json_schema" "jsonb",
    "data" "jsonb" DEFAULT '{}'::"jsonb" NOT NULL,
    "targets_form.ui_schema" "jsonb" DEFAULT '{}'::"jsonb",
    "targets_form.form_data" "jsonb" DEFAULT '{}'::"jsonb",
    "status" "text" DEFAULT 'active'::"text",
    "unique_node_id" "text",
    "isPublic" boolean DEFAULT false,
    "targets_form.extra" "jsonb" DEFAULT '{}'::"jsonb" NOT NULL,
    "storage_path" "text",
    "licenses" "text"[]
);

COMMENT ON TABLE "public"."nodes" IS 'Nodes Table';

COMMENT ON COLUMN "public"."nodes"."data" IS 'data';

COMMENT ON COLUMN "public"."nodes"."unique_node_id" IS 'Node id i.e http.0.1';

COMMENT ON COLUMN "public"."nodes"."storage_path" IS 'Path to where wasm file is stored';

ALTER TABLE "public"."nodes" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."nodes_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."pubkey_whitelists" (
    "pubkey" "text" NOT NULL,
    "info" "text"
);

CREATE SEQUENCE IF NOT EXISTS "public"."seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE IF NOT EXISTS "public"."signature_requests" (
    "user_id" "uuid" NOT NULL,
    "id" bigint NOT NULL,
    "created_at" timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "msg" "text" NOT NULL,
    "pubkey" "text" NOT NULL,
    "signature" "text",
    "flow_run_id" "uuid",
    "signatures" "jsonb"[],
    "new_msg" "text"
);

CREATE SEQUENCE IF NOT EXISTS "public"."signature_requests_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER SEQUENCE "public"."signature_requests_id_seq" OWNED BY "public"."signature_requests"."id";

CREATE TABLE IF NOT EXISTS "public"."user_quotas" (
    "user_id" "uuid" NOT NULL,
    "kvstore_count" bigint DEFAULT 0 NOT NULL,
    "kvstore_count_limit" bigint DEFAULT 100 NOT NULL,
    "kvstore_size" bigint DEFAULT 0 NOT NULL,
    "kvstore_size_limit" bigint DEFAULT ((1024 * 1024) * 100) NOT NULL,
    "credit" bigint DEFAULT '30'::bigint NOT NULL,
    "used_credit" bigint DEFAULT 0 NOT NULL
);

CREATE TABLE IF NOT EXISTS "public"."users_public" (
    "email" "text" NOT NULL,
    "user_id" "uuid" NOT NULL,
    "username" "text" DEFAULT ''::"text",
    "description" "text" DEFAULT ''::"text",
    "pub_key" "text" NOT NULL,
    "status" "text" DEFAULT 'not_available'::"text" NOT NULL,
    "updated_at" timestamp without time zone DEFAULT "now"(),
    "avatar" "text" DEFAULT ''::"text",
    "flow_skills" "jsonb" DEFAULT '[]'::"jsonb",
    "node_skills" "jsonb" DEFAULT '[]'::"jsonb",
    "tasks_skills" "jsonb" DEFAULT '[]'::"jsonb"
);

COMMENT ON TABLE "public"."users_public" IS 'Profile data for each user.';

COMMENT ON COLUMN "public"."users_public"."pub_key" IS 'Public Key';

COMMENT ON COLUMN "public"."users_public"."status" IS 'I am available for work';

CREATE TABLE IF NOT EXISTS "public"."wallets" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "type" "text" DEFAULT 'ADAPTER'::"text",
    "adapter" "text" DEFAULT ''::"text",
    "public_key" "text",
    "user_id" "uuid" NOT NULL,
    "description" "text" DEFAULT 'Wallet used for payments'::"text" NOT NULL,
    "name" "text" DEFAULT ''::"text" NOT NULL,
    "icon" "text",
    "keypair" "text"
);

ALTER TABLE "public"."wallets" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."wallets_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

ALTER TABLE ONLY "public"."signature_requests" ALTER COLUMN "id" SET DEFAULT "nextval"('"public"."signature_requests_id_seq"'::"regclass");

ALTER TABLE ONLY "public"."apikeys"
    ADD CONSTRAINT "apikeys_pkey" PRIMARY KEY ("key_hash");

ALTER TABLE ONLY "public"."flow_run_logs"
    ADD CONSTRAINT "flow_run_logs_pkey" PRIMARY KEY ("flow_run_id", "log_index");

ALTER TABLE ONLY "public"."flow_run"
    ADD CONSTRAINT "flow_run_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."flow_run_shared"
    ADD CONSTRAINT "flow_run_shared_pkey" PRIMARY KEY ("flow_run_id", "user_id");

ALTER TABLE ONLY "public"."flows"
    ADD CONSTRAINT "flows_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."kvstore_metadata"
    ADD CONSTRAINT "kvstore_metadata_pkey" PRIMARY KEY ("user_id", "store_name");

ALTER TABLE ONLY "public"."node_run"
    ADD CONSTRAINT "node_run_pkey" PRIMARY KEY ("flow_run_id", "node_id", "times");

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_unique_node_id_key" UNIQUE ("unique_node_id");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "pubkey_unique" UNIQUE ("pub_key");

ALTER TABLE ONLY "public"."pubkey_whitelists"
    ADD CONSTRAINT "pubkey_whitelists_pkey" PRIMARY KEY ("pubkey");

ALTER TABLE ONLY "public"."signature_requests"
    ADD CONSTRAINT "signature_requests_pkey" PRIMARY KEY ("user_id", "id");

ALTER TABLE ONLY "public"."apikeys"
    ADD CONSTRAINT "uc-user_id-name" UNIQUE ("user_id", "name");

ALTER TABLE ONLY "public"."kvstore"
    ADD CONSTRAINT "uq_user_id_store_name_key" PRIMARY KEY ("user_id", "store_name", "key");

ALTER TABLE ONLY "public"."user_quotas"
    ADD CONSTRAINT "user_quotas_pkey" PRIMARY KEY ("user_id");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_email_key" UNIQUE ("email");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_pkey" PRIMARY KEY ("user_id");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_pub_key_key" UNIQUE ("pub_key");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_username_key" UNIQUE ("username");

ALTER TABLE ONLY "public"."wallets"
    ADD CONSTRAINT "wallets_pkey" PRIMARY KEY ("id");

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."flows" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('updated_at');

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."kvstore" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('last_updated');

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."users_public" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('updated_at');

ALTER TABLE ONLY "public"."flow_run"
    ADD CONSTRAINT "fk-flow_id" FOREIGN KEY ("flow_id") REFERENCES "public"."flows"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."node_run"
    ADD CONSTRAINT "fk-flow_run_id" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run_logs"
    ADD CONSTRAINT "fk-flow_run_id" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run_shared"
    ADD CONSTRAINT "fk-flow_run_id" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run_logs"
    ADD CONSTRAINT "fk-node_run_id" FOREIGN KEY ("flow_run_id", "node_id", "times") REFERENCES "public"."node_run"("flow_run_id", "node_id", "times") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."node_run"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run_logs"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."signature_requests"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."apikeys"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flow_run_shared"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."flows"
    ADD CONSTRAINT "flows_lastest_flow_run_id_fkey" FOREIGN KEY ("lastest_flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE SET NULL;

ALTER TABLE ONLY "public"."flows"
    ADD CONSTRAINT "flows_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."kvstore_metadata"
    ADD CONSTRAINT "kvstore_metadata_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."signature_requests"
    ADD CONSTRAINT "signature_requests_flow_run_id_fkey" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE SET NULL;

ALTER TABLE ONLY "public"."user_quotas"
    ADD CONSTRAINT "user_quotas_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."wallets"
    ADD CONSTRAINT "wallets_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

CREATE POLICY "Enable delete for users based on user_id" ON "public"."wallets" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable insert for authenticated users only" ON "public"."wallets" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable read access for all users" ON "public"."users_public" FOR SELECT USING (true);

CREATE POLICY "Enable read access for authenticated users" ON "public"."wallets" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable update for users based on user_id" ON "public"."users_public" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable update for users based on user_id" ON "public"."wallets" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "anon-select" ON "public"."flows" FOR SELECT TO "anon" USING (("isPublic" = true));

CREATE POLICY "anon-select" ON "public"."nodes" FOR SELECT TO "anon" USING (("isPublic" = true));

ALTER TABLE "public"."apikeys" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "authenticated-delete" ON "public"."flows" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-delete" ON "public"."nodes" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-insert" ON "public"."flows" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-insert" ON "public"."nodes" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select" ON "public"."apikeys" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select" ON "public"."flows" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true)));

CREATE POLICY "authenticated-select" ON "public"."nodes" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true)));

CREATE POLICY "authenticated-select" ON "public"."signature_requests" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select-flow_run-shared" ON "public"."flow_run" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR (EXISTS ( SELECT 1
   FROM "public"."flow_run_shared" "s"
  WHERE (("s"."flow_run_id" = "flow_run"."id") AND ("s"."user_id" = "auth"."uid"()))))));

CREATE POLICY "authenticated-select-flow_run_logs-shared" ON "public"."flow_run_logs" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR (EXISTS ( SELECT 1
   FROM "public"."flow_run_shared" "s"
  WHERE (("s"."flow_run_id" = "flow_run_logs"."flow_run_id") AND ("s"."user_id" = "auth"."uid"()))))));

CREATE POLICY "authenticated-select-flow_run_shared" ON "public"."flow_run_shared" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select-kvstore" ON "public"."kvstore" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select-kvstore_metadata" ON "public"."kvstore_metadata" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select-node_run-shared" ON "public"."node_run" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR (EXISTS ( SELECT 1
   FROM "public"."flow_run_shared" "s"
  WHERE (("s"."flow_run_id" = "node_run"."flow_run_id") AND ("s"."user_id" = "auth"."uid"()))))));

CREATE POLICY "authenticated-select-user_quotas" ON "public"."user_quotas" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-update" ON "public"."flows" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-update" ON "public"."nodes" FOR UPDATE TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true))) WITH CHECK ((("type")::"text" <> 'native'::"text"));

ALTER TABLE "public"."flow_run" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flow_run_logs" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flow_run_shared" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flows" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."kvstore" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."kvstore_metadata" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "nft_admins-select" ON "public"."user_quotas" FOR SELECT TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "nft_admins-update" ON "public"."user_quotas" FOR UPDATE TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

ALTER TABLE "public"."node_run" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."nodes" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."pubkey_whitelists" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."signature_requests" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "supabase_auth_admin-select-pubkey_whitelists" ON "public"."pubkey_whitelists" FOR SELECT TO "supabase_auth_admin" USING (true);

ALTER TABLE "public"."user_quotas" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."users_public" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."wallets" ENABLE ROW LEVEL SECURITY;

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run_logs";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."node_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."signature_requests";

REVOKE USAGE ON SCHEMA "public" FROM PUBLIC;
GRANT USAGE ON SCHEMA "public" TO "flow_runner";
GRANT USAGE ON SCHEMA "auth" TO "flow_runner";
GRANT USAGE ON SCHEMA "storage" TO "flow_runner";
GRANT ALL ON SCHEMA "public" TO PUBLIC;

GRANT ALL ON TABLE "public"."apikeys" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run_logs" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run_shared" TO "flow_runner";

GRANT ALL ON TABLE "public"."flows" TO "flow_runner";

GRANT SELECT,USAGE ON SEQUENCE "public"."flows_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."kvstore" TO "flow_runner";

GRANT ALL ON TABLE "public"."kvstore_metadata" TO "flow_runner";

GRANT ALL ON TABLE "public"."node_run" TO "flow_runner";

GRANT ALL ON TABLE "public"."nodes" TO "flow_runner";

GRANT SELECT,USAGE ON SEQUENCE "public"."nodes_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "flow_runner";

GRANT SELECT,USAGE ON SEQUENCE "public"."seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."signature_requests" TO "flow_runner";

GRANT SELECT,USAGE ON SEQUENCE "public"."signature_requests_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."user_quotas" TO "flow_runner";

GRANT ALL ON TABLE "public"."users_public" TO "flow_runner";

GRANT ALL ON TABLE "public"."wallets" TO "flow_runner";

GRANT SELECT,USAGE ON SEQUENCE "public"."wallets_id_seq" TO "flow_runner";

GRANT SELECT ON TABLE "public"."pubkey_whitelists" TO "supabase_auth_admin";

--
-- Dumped schema changes for auth and storage
--

CREATE OR REPLACE FUNCTION "auth"."validate_user"() RETURNS "trigger"
    LANGUAGE "plpgsql"
    AS $$
declare
myrec record;
begin
    select * into myrec from public.pubkey_whitelists
    where pubkey = new.raw_user_meta_data->>'pub_key' and pubkey is not null;
    if not found then
        raise exception 'pubkey is not in whitelists, %', new.raw_user_meta_data->>'pub_key';
    end if;

    return new;
end;
$$;

GRANT UPDATE ON TABLE "auth"."users" TO "flow_runner";

CREATE TABLE IF NOT EXISTS "auth"."passwords" (
    "user_id" "uuid" NOT NULL,
    "password" "text" NOT NULL
);

ALTER TABLE ONLY "auth"."passwords"
    ADD CONSTRAINT "passwords_pkey" PRIMARY KEY ("user_id");

CREATE OR REPLACE TRIGGER "on_auth_check_whitelists" BEFORE INSERT ON "auth"."users" FOR EACH ROW EXECUTE FUNCTION "auth"."validate_user"();

ALTER TABLE ONLY "auth"."passwords"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

GRANT ALL ON TABLE "auth"."passwords" TO "flow_runner";

INSERT INTO "storage"."buckets" ("id", "name", "public") VALUES
    ('user-storages', 'user-storages', FALSE),
    ('user-public-storages', 'user-public-storages', TRUE);

CREATE POLICY "user-pubic-storages xeg75m_0" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_1" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_2" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-public-storages xeg75m_0" ON "storage"."objects" FOR SELECT TO "authenticated", "anon" USING (("bucket_id" = 'user-public-storages'::"text"));

CREATE POLICY "user-storage w6lp96_0" ON "storage"."objects" FOR SELECT TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_1" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_2" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_3" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE OR REPLACE FUNCTION "public"."handle_new_user"() RETURNS "trigger"
LANGUAGE "plpgsql"
SECURITY DEFINER
AS $$
BEGIN
  INSERT INTO public.users_public
  (email, user_id, username, pub_key)
  VALUES (
    new.email,
    new.id,
    new.raw_user_meta_data->>'pub_key',
    new.raw_user_meta_data->>'pub_key'
  );

  INSERT INTO public.user_quotas (user_id) VALUES (new.id);

  INSERT INTO public.wallets (user_id, public_key, type, name, description)
  VALUES (new.id, new.raw_user_meta_data->>'pub_key', 'ADAPTER', 'Main wallet', 'Wallet used to sign up');

  RETURN new;
END;
$$;

CREATE OR REPLACE TRIGGER "on_auth_user_created" AFTER INSERT ON "auth"."users" FOR EACH ROW EXECUTE FUNCTION "public"."handle_new_user"();

RESET ALL;

-- <<< END 20240514130738_init.sql


-- >>> BEGIN 20240517061121_grant.sql
GRANT USAGE ON SCHEMA auth TO flow_runner;
GRANT SELECT ON ALL TABLES IN SCHEMA auth TO flow_runner;
GRANT SELECT ON ALL SEQUENCES IN SCHEMA auth TO flow_runner;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO flow_runner;
GRANT ALL ON TABLE auth.users TO flow_runner;
GRANT ALL ON TABLE auth.identities TO flow_runner;

-- <<< END 20240517061121_grant.sql


-- >>> BEGIN 20240518143018_auth_trigger.sql
CREATE OR REPLACE FUNCTION auth.disable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users DISABLE TRIGGER on_auth_user_created;
$$ SECURITY DEFINER;

GRANT EXECUTE ON FUNCTION auth.disable_users_triggers() to flow_runner;

CREATE OR REPLACE FUNCTION auth.enable_users_triggers()
RETURNS void
LANGUAGE SQL
AS $$
ALTER TABLE auth.users ENABLE TRIGGER on_auth_user_created;
$$ SECURITY DEFINER;

GRANT EXECUTE ON FUNCTION auth.enable_users_triggers() to flow_runner;

-- <<< END 20240518143018_auth_trigger.sql


-- >>> BEGIN 20240524150823_grant_sequence.sql
GRANT UPDATE ON ALL SEQUENCES IN SCHEMA public TO flow_runner;

-- <<< END 20240524150823_grant_sequence.sql


-- >>> BEGIN 20240525104546_kvstore_fk.sql
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

-- <<< END 20240525104546_kvstore_fk.sql


-- >>> BEGIN 20240905183752_encrypt.sql
ALTER TABLE wallets ADD COLUMN encrypted_keypair JSONB NULL;
UPDATE wallets SET encrypted_keypair['raw'] = to_json(keypair) WHERE keypair IS NOT NULL AND encrypted_keypair IS NULL;

-- <<< END 20240905183752_encrypt.sql


-- >>> BEGIN 20240906100833_grant_wallet_update.sql
GRANT INSERT, UPDATE ON wallets TO flow_runner;

-- <<< END 20240906100833_grant_wallet_update.sql


-- >>> BEGIN 20240907035451_update_wallets_table.sql
ALTER TABLE wallets
ALTER COLUMN public_key TYPE text,
ALTER COLUMN public_key SET NOT NULL;

-- <<< END 20240907035451_update_wallets_table.sql


-- >>> BEGIN 20240930062601_remove_keypair.sql
ALTER TABLE wallets DROP COLUMN keypair;

-- <<< END 20240930062601_remove_keypair.sql


-- >>> BEGIN 20241008071914_update_nodes.sql
ALTER TABLE nodes ALTER COLUMN data SET DATA TYPE jsonb USING data::jsonb;
ALTER TABLE nodes ALTER COLUMN sources SET DATA TYPE jsonb USING sources::jsonb;
ALTER TABLE nodes ALTER COLUMN targets SET DATA TYPE jsonb USING targets::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.json_schema" SET DATA TYPE jsonb USING "targets_form.json_schema"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.ui_schema" SET DATA TYPE jsonb USING "targets_form.ui_schema"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.form_data" SET DATA TYPE jsonb USING "targets_form.form_data"::jsonb;
ALTER TABLE nodes ALTER COLUMN "targets_form.extra" SET DATA TYPE jsonb USING "targets_form.extra"::jsonb;

-- <<< END 20241008071914_update_nodes.sql


-- >>> BEGIN 20241008143527_nodes_update_policy.sql
ALTER POLICY "authenticated-update" ON nodes TO authenticated USING (auth.uid() = user_id);

-- <<< END 20241008143527_nodes_update_policy.sql


-- >>> BEGIN 20241008144128_with_check.sql
ALTER POLICY "authenticated-update" ON nodes TO authenticated USING (auth.uid() = user_id) WITH CHECK (true);

-- <<< END 20241008144128_with_check.sql


-- >>> BEGIN 20241030051429_check_native_nodes.sql
ALTER TABLE nodes ADD CONSTRAINT native_check CHECK (
    type <> 'native' OR user_id IS NULL OR "isPublic" = FALSE
) NO INHERIT;

-- <<< END 20241030051429_check_native_nodes.sql


-- >>> BEGIN 20241202120303_update_flows_table.sql
UPDATE flows SET nodes = '{}'::jsonb[] WHERE nodes IS NULL;
UPDATE flows SET edges = '{}'::jsonb[] WHERE edges IS NULL;
UPDATE flows SET environment = '{}'::jsonb WHERE environment IS NULL;
ALTER TABLE flows
ALTER COLUMN nodes SET DEFAULT '{}'::jsonb[], ALTER COLUMN nodes SET NOT NULL,
ALTER COLUMN edges SET DEFAULT '{}'::jsonb[], ALTER COLUMN edges SET NOT NULL,
ALTER COLUMN parent_flow TYPE INTEGER,
ALTER COLUMN environment SET DEFAULT '{}'::jsonb, ALTER COLUMN environment SET NOT NULL;

-- <<< END 20241202120303_update_flows_table.sql


-- >>> BEGIN 20241214133549_flow_deployment.sql
CREATE TABLE flow_deployments (
    id UUID NOT NULL CHECK (id <> '00000000-0000-0000-0000-000000000000'),
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    user_id UUID NOT NULL,
    entrypoint INTEGER NOT NULL,
    start_permission JSONB NOT NULL,
    output_instructions BOOL NOT NULL,
    action_identity TEXT NULL,
    fees JSONB NOT NULL,
    solana_network JSONB NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (id, entrypoint)
);

-- Wallets used in a deployment
CREATE TABLE flow_deployments_wallets (
    user_id UUID NOT NULL,
    deployment_id UUID NOT NULL,
    wallet_id BIGINT NOT NULL,
    PRIMARY KEY (deployment_id, wallet_id),
    FOREIGN KEY (user_id) REFERENCES auth.users (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id) REFERENCES flow_deployments (id) ON DELETE CASCADE
);

-- Flows used in a deployment
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
    user_id UUID NOT NULL,
    entrypoint INTEGER NOT NULL,
    tag TEXT NOT NULL,
    deployment_id UUID NOT NULL,
    description TEXT NULL,
    PRIMARY KEY (entrypoint, tag),
    FOREIGN KEY (user_id) REFERENCES auth.users (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id) REFERENCES flow_deployments (id) ON DELETE CASCADE,
    FOREIGN KEY (deployment_id, entrypoint) REFERENCES flow_deployments (id, entrypoint)
);

create or replace function flow_deployments_insert()
returns trigger as
$$
begin
    insert into
    flow_deployments_tags(entrypoint,      tag,     deployment_id, user_id)
                   values(new.entrypoint, 'latest', new.id,        new.user_id)
    on conflict (entrypoint, tag)
    do update set deployment_id = new.id;
    return new;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_insert
after insert on flow_deployments
for each row execute function flow_deployments_insert();

create or replace function flow_deployments_delete()
returns trigger as
$$
begin
    insert into
    flow_deployments_tags (entrypoint, tag, deployment_id, user_id)
    (
        select
            entrypoint,
            'latest' as tag,
            id as deployment_id,
            user_id
        from flow_deployments
        where entrypoint = old.entrypoint
        order by id desc
        limit 1
    )
    on conflict (entrypoint, tag) do nothing;

    return old;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_delete
after delete on flow_deployments
for each row execute function flow_deployments_delete();


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

CREATE POLICY "owner-delete" ON flow_deployments FOR DELETE TO authenticated USING (auth.uid() = user_id);
CREATE POLICY "owner-update" ON flow_deployments FOR UPDATE TO authenticated USING (auth.uid() = user_id);

CREATE POLICY "owner-delete" ON flow_deployments_tags FOR DELETE TO authenticated USING (auth.uid() = user_id);
CREATE POLICY "owner-insert" ON flow_deployments_tags FOR INSERT TO authenticated WITH CHECK (auth.uid() = user_id);
CREATE POLICY "owner-update" ON flow_deployments_tags FOR DELETE TO authenticated USING (auth.uid() = user_id);

alter table flow_run add column deployment_id uuid null references flow_deployments (id) on delete set null;

-- <<< END 20241214133549_flow_deployment.sql


-- >>> BEGIN 20241230141331_wallet_purpose.sql
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS purpose CHARACTER VARYING NULL;

-- <<< END 20241230141331_wallet_purpose.sql


-- >>> BEGIN 20241230142807_flow_gg_marketplace.sql
ALTER TABLE flows ADD COLUMN IF NOT EXISTS gg_marketplace BOOLEAN NULL;

-- <<< END 20241230142807_flow_gg_marketplace.sql


-- >>> BEGIN 20250313050903_flow_deploymen_select_rls.sql
drop policy if exists "owner-select" on flow_deployments;

drop policy if exists "authenticated-select" on flow_deployments;
create policy "authenticated-select" on flow_deployments for select to authenticated
using (
    auth.uid() = user_id
    or start_permission = '"Authenticated"'::jsonb
    or start_permission = '"Anonymous"'::jsonb
);

drop policy if exists "anonymous-select" on flow_deployments;
create policy "anonymous-select" on flow_deployments for select to anon using (start_permission = '"Anonymous"'::jsonb);

-- <<< END 20250313050903_flow_deploymen_select_rls.sql


-- >>> BEGIN 20251127051655_x402.sql
create type x402network as enum (
    'base', 'base-sepolia',
    'solana', 'solana-devnet'
);

create table flow_x402_fees (
    user_id uuid references auth.users(id) on delete cascade,
    id bigserial primary key,
    flow_id integer not null references flows(id) on delete cascade,
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);
alter table flow_x402_fees enable row level security;

create table flow_deployments_x402_fees (
    user_id uuid references auth.users(id) on delete cascade,
    id bigserial primary key,
    deployment_id uuid not null references flow_deployments(id) on delete cascade,
    network x402network not null,
    pay_to bigint not null references wallets(id),
    amount decimal not null,
    enabled boolean not null
);

grant select on flow_deployments_x402_fees to flow_runner;
grant select on flow_x402_fees to flow_runner;

alter table flow_deployments_x402_fees enable row level security;
create policy "owner-select" on flow_deployments_x402_fees for select to authenticated using (auth.uid() = user_id);
create policy "owner-insert" on flow_deployments_x402_fees for insert to authenticated with check (auth.uid() = user_id);
create policy "owner-delete" on flow_deployments_x402_fees for delete to authenticated using (auth.uid() = user_id);
create policy "owner-update" on flow_deployments_x402_fees for update to authenticated using (auth.uid() = user_id);

alter table flow_x402_fees enable row level security;
create policy "owner-select" on flow_x402_fees for select to authenticated using (auth.uid() = user_id);
create policy "owner-insert" on flow_x402_fees for insert to authenticated with check (auth.uid() = user_id);
create policy "owner-delete" on flow_x402_fees for delete to authenticated using (auth.uid() = user_id);
create policy "owner-update" on flow_x402_fees for update to authenticated using (auth.uid() = user_id);

-- <<< END 20251127051655_x402.sql


-- >>> BEGIN 20260220090000_create_flows_v2.sql
-- Canonical V2 flows table for scoped-node transport payloads.

create table if not exists public.flows_v2 (
    id integer primary key generated always as identity,
    uuid uuid not null default gen_random_uuid(),

    user_id uuid not null references auth.users(id) on delete cascade,

    name text not null default ''::text,
    description text not null default ''::text,
    slug text,

    "isPublic" boolean not null default false,
    gg_marketplace boolean not null default false,
    visibility_profile text,

    created_at timestamp without time zone not null default current_timestamp,
    updated_at timestamp without time zone not null default current_timestamp,

    -- Canonical V2 transport payloads.
    nodes jsonb not null default '[]'::jsonb,
    edges jsonb not null default '[]'::jsonb,
    viewport jsonb not null default '{"x":0,"y":0,"zoom":1}'::jsonb,

    environment jsonb not null default '{}'::jsonb,
    guide jsonb,
    instructions_bundling jsonb not null default '"Off"'::jsonb,
    backend_endpoint text,

    current_network jsonb not null default '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb,
    start_shared boolean not null default false,
    start_unverified boolean not null default false,
    current_branch_id integer,

    parent_flow integer,
    linked_flows jsonb,
    lifecycle jsonb,

    meta_nodes jsonb not null default '[]'::jsonb,
    default_viewport jsonb not null default '{"x":0,"y":0,"zoom":1}'::jsonb
);

create unique index if not exists flows_v2_uuid_key on public.flows_v2 (uuid);
create unique index if not exists flows_v2_slug_key on public.flows_v2 (slug) where slug is not null;
create index if not exists idx_flows_v2_user_id on public.flows_v2 (user_id);
create index if not exists idx_flows_v2_is_public on public.flows_v2 ("isPublic");
create index if not exists idx_flows_v2_current_branch_id on public.flows_v2 (current_branch_id);
create index if not exists idx_flows_v2_nodes_gin on public.flows_v2 using gin (nodes);
create index if not exists idx_flows_v2_edges_gin on public.flows_v2 using gin (edges);

alter table public.flows_v2 enable row level security;

do $$
begin
    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-select'
    ) then
        create policy "owner-select" on public.flows_v2
            for select to authenticated using (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'public-select'
    ) then
        create policy "public-select" on public.flows_v2
            for select to anon using ("isPublic" = true);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-insert'
    ) then
        create policy "owner-insert" on public.flows_v2
            for insert to authenticated with check (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-update'
    ) then
        create policy "owner-update" on public.flows_v2
            for update to authenticated using (auth.uid() = user_id);
    end if;

    if not exists (
        select 1 from pg_policies
        where tablename = 'flows_v2' and policyname = 'owner-delete'
    ) then
        create policy "owner-delete" on public.flows_v2
            for delete to authenticated using (auth.uid() = user_id);
    end if;
end $$;

grant select, insert, update, delete on public.flows_v2 to authenticated;
grant select on public.flows_v2 to anon;
grant select, insert, update, delete on public.flows_v2 to flow_runner;

-- <<< END 20260220090000_create_flows_v2.sql


-- >>> BEGIN 20260220090100_flows_v2_runtime_fields.sql
-- Runtime field parity hardening for flows_v2.

alter table public.flows_v2
    add column if not exists current_network jsonb,
    add column if not exists start_shared boolean,
    add column if not exists start_unverified boolean,
    add column if not exists current_branch_id integer;

update public.flows_v2
set current_network = '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb
where current_network is null;

update public.flows_v2
set start_shared = false
where start_shared is null;

update public.flows_v2
set start_unverified = false
where start_unverified is null;

alter table public.flows_v2
    alter column current_network set default '{"id":"01000000-0000-8000-8000-000000000000","url":"https://api.devnet.solana.com","type":"default","wallet":"Solana","cluster":"devnet"}'::jsonb,
    alter column current_network set not null,
    alter column start_shared set default false,
    alter column start_shared set not null,
    alter column start_unverified set default false,
    alter column start_unverified set not null;

comment on column public.flows_v2.current_network is 'Runtime network config used by backend start logic.';
comment on column public.flows_v2.start_shared is 'Allow authenticated shared starts (/start_shared).';
comment on column public.flows_v2.start_unverified is 'Allow unverified starts (/start_unverified).';
comment on column public.flows_v2.current_branch_id is 'Git-like branch pointer for editor/runtime integration.';

-- <<< END 20260220090100_flows_v2_runtime_fields.sql


-- >>> BEGIN 20260220090200_flow_id_uuid_cutover.sql
-- Convert flow_run.flow_id from INTEGER (flows.id) to UUID (flows_v2.uuid).

-- Ensure V2 rows exist for legacy flows so UUID foreign keys can be enforced.
insert into public.flows_v2 (
    uuid,
    user_id,
    name,
    description,
    slug,
    "isPublic",
    gg_marketplace,
    visibility_profile,
    nodes,
    edges,
    viewport,
    environment,
    guide,
    instructions_bundling,
    backend_endpoint,
    current_network,
    start_shared,
    start_unverified,
    parent_flow,
    linked_flows,
    lifecycle
)
select
    f.uuid,
    f.user_id,
    f.name,
    coalesce(f.description, ''::text),
    f.slug,
    f."isPublic",
    f.gg_marketplace,
    f.visibility_profile,
    to_jsonb(f.nodes),
    to_jsonb(f.edges),
    coalesce(f.viewport, '{"x":0,"y":0,"zoom":1}'::jsonb),
    coalesce(f.environment, '{}'::jsonb),
    f.guide,
    coalesce(f.instructions_bundling, '"Off"'::jsonb),
    f.backend_endpoint,
    f.current_network,
    f.start_shared,
    f.start_unverified,
    case when f.parent_flow is null then null else f.parent_flow::integer end,
    f.linked_flows,
    f.lifecycle
from public.flows f
where not exists (
    select 1 from public.flows_v2 v2 where v2.uuid = f.uuid
);

alter table public.flow_run
    add column if not exists flow_id_v2 uuid;

update public.flow_run fr
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where fr.flow_id = f.id
  and fr.flow_id_v2 is null;

do $$
begin
    if exists (
        select 1 from public.flow_run where flow_id_v2 is null
    ) then
        raise exception 'flow_id_v2 backfill incomplete in flow_run';
    end if;
end $$;

alter table public.flow_run drop constraint if exists "fk-flow_id";
drop index if exists public.idx_flow_run_flow_id;

alter table public.flow_run
    drop column flow_id,
    rename column flow_id_v2 to flow_id;

alter table public.flow_run
    alter column flow_id set not null;

alter table public.flow_run
    add constraint flow_run_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;

create index if not exists idx_flow_run_flow_id on public.flow_run(flow_id);

-- <<< END 20260220090200_flow_id_uuid_cutover.sql


-- >>> BEGIN 20260220090300_deployments_uuid_cutover.sql
-- Convert deployment flow identifiers from INTEGER to UUID.

alter table public.flow_deployments
    add column if not exists entrypoint_v2 uuid;

alter table public.flow_deployments_flows
    add column if not exists flow_id_v2 uuid;

alter table public.flow_deployments_tags
    add column if not exists entrypoint_v2 uuid;

update public.flow_deployments d
set entrypoint_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where d.entrypoint = f.id
  and d.entrypoint_v2 is null;

update public.flow_deployments_flows df
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where df.flow_id = f.id
  and df.flow_id_v2 is null;

update public.flow_deployments_tags t
set entrypoint_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where t.entrypoint = f.id
  and t.entrypoint_v2 is null;

do $$
begin
    if exists (select 1 from public.flow_deployments where entrypoint_v2 is null) then
        raise exception 'entrypoint_v2 backfill incomplete for flow_deployments';
    end if;
    if exists (select 1 from public.flow_deployments_flows where flow_id_v2 is null) then
        raise exception 'flow_id_v2 backfill incomplete for flow_deployments_flows';
    end if;
    if exists (select 1 from public.flow_deployments_tags where entrypoint_v2 is null) then
        raise exception 'entrypoint_v2 backfill incomplete for flow_deployments_tags';
    end if;
end $$;

drop trigger if exists flow_deployments_insert on public.flow_deployments;
drop trigger if exists flow_deployments_delete on public.flow_deployments;
drop function if exists public.flow_deployments_insert();
drop function if exists public.flow_deployments_delete();

alter table public.flow_deployments drop constraint if exists flow_deployments_id_entrypoint_key;
alter table public.flow_deployments_flows drop constraint if exists flow_deployments_flows_pkey;
alter table public.flow_deployments_tags drop constraint if exists flow_deployments_tags_pkey;
alter table public.flow_deployments_tags drop constraint if exists flow_deployments_tags_deployment_id_entrypoint_fkey;

alter table public.flow_deployments
    drop column entrypoint,
    rename column entrypoint_v2 to entrypoint;

alter table public.flow_deployments_flows
    drop column flow_id,
    rename column flow_id_v2 to flow_id;

alter table public.flow_deployments_tags
    drop column entrypoint,
    rename column entrypoint_v2 to entrypoint;

alter table public.flow_deployments
    alter column entrypoint set not null;

alter table public.flow_deployments_flows
    alter column flow_id set not null;

alter table public.flow_deployments_tags
    alter column entrypoint set not null;

alter table public.flow_deployments
    add constraint flow_deployments_id_entrypoint_key unique (id, entrypoint);

alter table public.flow_deployments_flows
    add primary key (deployment_id, flow_id);

alter table public.flow_deployments_flows
    add constraint flow_deployments_flows_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;

alter table public.flow_deployments_tags
    add primary key (entrypoint, tag);

alter table public.flow_deployments_tags
    add constraint flow_deployments_tags_deployment_id_entrypoint_fkey
    foreign key (deployment_id, entrypoint)
    references public.flow_deployments(id, entrypoint)
    on delete cascade;

create or replace function public.flow_deployments_insert()
returns trigger as
$$
begin
    insert into
        public.flow_deployments_tags(entrypoint, tag, deployment_id, user_id)
    values
        (new.entrypoint, 'latest', new.id, new.user_id)
    on conflict (entrypoint, tag)
    do update set deployment_id = new.id;
    return new;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_insert
after insert on public.flow_deployments
for each row execute function public.flow_deployments_insert();

create or replace function public.flow_deployments_delete()
returns trigger as
$$
begin
    insert into
        public.flow_deployments_tags(entrypoint, tag, deployment_id, user_id)
    (
        select
            entrypoint,
            'latest' as tag,
            id as deployment_id,
            user_id
        from public.flow_deployments
        where entrypoint = old.entrypoint
        order by id desc
        limit 1
    )
    on conflict (entrypoint, tag) do nothing;

    return old;
end;
$$
language plpgsql
security definer;

create or replace trigger flow_deployments_delete
after delete on public.flow_deployments
for each row execute function public.flow_deployments_delete();

-- <<< END 20260220090300_deployments_uuid_cutover.sql


-- >>> BEGIN 20260220090400_interflow_payload_uuid.sql
-- Rewrite legacy interflow payload fields from form_data.id to config.flow_id (UUID string).

with transformed as (
    select
        f.id,
        (
            select jsonb_agg(
                case
                    when (node #>> '{data,node_id}') in (
                        'interflow',
                        'interflow_instructions',
                        '@spo/interflow',
                        '@spo/interflow_instructions'
                    )
                    and (node #> '{data,config,flow_id}') is null
                    and (node #> '{data,targets_form,form_data,id}') is not null
                    then jsonb_set(
                        node,
                        '{data,config,flow_id}',
                        to_jsonb(node #>> '{data,targets_form,form_data,id}'),
                        true
                    )
                    else node
                end
            )
            from jsonb_array_elements(f.nodes) as node
        ) as nodes_new
    from public.flows_v2 f
    where jsonb_typeof(f.nodes) = 'array'
      and exists (
        select 1
        from jsonb_array_elements(f.nodes) as node
        where (node #>> '{data,node_id}') in (
            'interflow',
            'interflow_instructions',
            '@spo/interflow',
            '@spo/interflow_instructions'
        )
          and (node #> '{data,config,flow_id}') is null
          and (node #> '{data,targets_form,form_data,id}') is not null
      )
)
update public.flows_v2 f
set nodes = t.nodes_new
from transformed t
where f.id = t.id;

with transformed as (
    select
        d.deployment_id,
        d.flow_id,
        (
            select jsonb_agg(
                case
                    when (node #>> '{data,node_id}') in (
                        'interflow',
                        'interflow_instructions',
                        '@spo/interflow',
                        '@spo/interflow_instructions'
                    )
                    and (node #> '{data,config,flow_id}') is null
                    and (node #> '{data,targets_form,form_data,id}') is not null
                    then jsonb_set(
                        node,
                        '{data,config,flow_id}',
                        to_jsonb(node #>> '{data,targets_form,form_data,id}'),
                        true
                    )
                    else node
                end
            )
            from jsonb_array_elements(d.data->'nodes') as node
        ) as nodes_new
    from public.flow_deployments_flows d
    where jsonb_typeof(d.data->'nodes') = 'array'
      and exists (
        select 1
        from jsonb_array_elements(d.data->'nodes') as node
        where (node #>> '{data,node_id}') in (
            'interflow',
            'interflow_instructions',
            '@spo/interflow',
            '@spo/interflow_instructions'
        )
          and (node #> '{data,config,flow_id}') is null
          and (node #> '{data,targets_form,form_data,id}') is not null
      )
)
update public.flow_deployments_flows d
set data = jsonb_set(d.data, '{nodes}', coalesce(t.nodes_new, '[]'::jsonb), false)
from transformed t
where d.deployment_id = t.deployment_id
  and d.flow_id = t.flow_id;

-- <<< END 20260220090400_interflow_payload_uuid.sql


-- >>> BEGIN 20260220090500_flow_x402_fees_uuid_cutover.sql
-- Convert flow_x402_fees.flow_id from INTEGER (flows.id) to UUID (flows_v2.uuid).

alter table public.flow_x402_fees
    add column if not exists flow_id_v2 uuid;

update public.flow_x402_fees x
set flow_id_v2 = coalesce(v2.uuid, f.uuid)
from public.flows f
left join public.flows_v2 v2 on v2.uuid = f.uuid
where x.flow_id = f.id
  and x.flow_id_v2 is null;

do $$
begin
    if exists (select 1 from public.flow_x402_fees where flow_id_v2 is null) then
        raise exception 'flow_id_v2 backfill incomplete for flow_x402_fees';
    end if;
end $$;

alter table public.flow_x402_fees
    drop constraint if exists flow_x402_fees_flow_id_fkey;

alter table public.flow_x402_fees
    drop column flow_id,
    rename column flow_id_v2 to flow_id;

alter table public.flow_x402_fees
    alter column flow_id set not null;

alter table public.flow_x402_fees
    add constraint flow_x402_fees_flow_id_fkey
    foreign key (flow_id) references public.flows_v2(uuid) on delete cascade;

-- <<< END 20260220090500_flow_x402_fees_uuid_cutover.sql
