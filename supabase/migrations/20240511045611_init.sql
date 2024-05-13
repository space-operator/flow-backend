
SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

CREATE EXTENSION IF NOT EXISTS "pg_net" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgsodium" WITH SCHEMA "pgsodium";

ALTER SCHEMA "public" OWNER TO "postgres";

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

CREATE ROLE "flow_runner";
ALTER ROLE "flow_runner" WITH INHERIT NOCREATEROLE CREATEDB LOGIN REPLICATION BYPASSRLS;

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

ALTER FUNCTION "public"."handle_new_user"() OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) RETURNS bigint
    LANGUAGE "sql"
    AS $_$UPDATE user_quotas SET credit = credit + $2 WHERE user_id = $1 AND $2 >= 0 RETURNING credit;$_$;

ALTER FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) RETURNS bigint
    LANGUAGE "sql"
    AS $_$UPDATE user_quotas SET used_credit = used_credit + $2 WHERE user_id = $1 AND $2 >= 0 AND used_credit + $2 <= credit RETURNING used_credit;$_$;

ALTER FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."is_nft_admin"("user_id" "uuid") RETURNS boolean
    LANGUAGE "sql" STABLE SECURITY DEFINER
    AS $_$SELECT EXISTS (SELECT user_id FROM nft_admins WHERE user_id = $1);$_$;

ALTER FUNCTION "public"."is_nft_admin"("user_id" "uuid") OWNER TO "postgres";

SET default_tablespace = '';

SET default_table_access_method = "heap";

CREATE TABLE IF NOT EXISTS "public"."apikeys" (
    "key_hash" "text" NOT NULL,
    "user_id" "uuid" NOT NULL,
    "name" "text" NOT NULL,
    "trimmed_key" "text" NOT NULL,
    "created_at" timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

ALTER TABLE "public"."apikeys" OWNER TO "postgres";

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

ALTER TABLE "public"."flow_run" OWNER TO "postgres";

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

ALTER TABLE "public"."flow_run_logs" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."flow_run_shared" (
    "flow_run_id" "uuid" NOT NULL,
    "user_id" "uuid" NOT NULL
);

ALTER TABLE "public"."flow_run_shared" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."flows" (
    "id" integer NOT NULL,
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "name" character varying DEFAULT ''::character varying NOT NULL,
    "isPublic" boolean DEFAULT false NOT NULL,
    "description" "text" DEFAULT 'Flow Description'::"text" NOT NULL,
    "tags" "text"[] DEFAULT '{}'::"text"[] NOT NULL,
    "created_at" "date" DEFAULT "now"() NOT NULL,
    "parent_flow" bigint,
    "viewport" "json" DEFAULT '{   "x": 524,   "y": 268,   "zoom": 0.5 }'::"json" NOT NULL,
    "uuid" "uuid" DEFAULT "extensions"."uuid_generate_v4"(),
    "updated_at" timestamp without time zone,
    "lastest_flow_run_id" "uuid",
    "custom_networks" "jsonb"[] DEFAULT '{}'::"jsonb"[] NOT NULL,
    "current_network" "jsonb" DEFAULT '{"id": "01000000-0000-8000-8000-000000000000", "url": "https://api.devnet.solana.com", "type": "default", "wallet": "Solana", "cluster": "devnet"}'::"jsonb" NOT NULL,
    "instructions_bundling" "jsonb" DEFAULT '"Off"'::"jsonb" NOT NULL,
    "guide" "json",
    "environment" "jsonb",
    "nodes" "jsonb"[],
    "edges" "jsonb"[],
    "mosaic" "jsonb",
    "start_shared" boolean DEFAULT false NOT NULL,
    "start_unverified" boolean DEFAULT false NOT NULL
);

ALTER TABLE "public"."flows" OWNER TO "postgres";

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
    "key" character varying NOT NULL,
    "value" "jsonb" NOT NULL,
    "last_updated" timestamp without time zone DEFAULT "now"()
);

ALTER TABLE "public"."kvstore" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."kvstore_metadata" (
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "store_name" character varying NOT NULL,
    "stats_size" bigint DEFAULT 0 NOT NULL
);

ALTER TABLE "public"."kvstore_metadata" OWNER TO "postgres";

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

ALTER TABLE "public"."node_run" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."nodes" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "name" character varying DEFAULT ''::character varying,
    "user_id" "uuid" DEFAULT "auth"."uid"(),
    "type" character varying DEFAULT 'mock'::character varying,
    "sources" "json" DEFAULT '[]'::"json" NOT NULL,
    "targets" "json" DEFAULT '[]'::"json" NOT NULL,
    "targets_form.json_schema" "json",
    "data" "json" DEFAULT '{}'::"json" NOT NULL,
    "targets_form.ui_schema" "json" DEFAULT '{}'::"json",
    "targets_form.form_data" "json" DEFAULT '{}'::"json",
    "status" "text" DEFAULT 'active'::"text",
    "unique_node_id" character varying,
    "isPublic" boolean DEFAULT false,
    "targets_form.extra" "json" DEFAULT '{}'::"json" NOT NULL,
    "storage_path" "text",
    "licenses" "text"[]
);

ALTER TABLE "public"."nodes" OWNER TO "postgres";

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
    "pubkey" character varying NOT NULL,
    "info" character varying
);

ALTER TABLE "public"."pubkey_whitelists" OWNER TO "postgres";

CREATE SEQUENCE IF NOT EXISTS "public"."seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER TABLE "public"."seq" OWNER TO "postgres";

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

ALTER TABLE "public"."signature_requests" OWNER TO "postgres";

CREATE SEQUENCE IF NOT EXISTS "public"."signature_requests_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

ALTER TABLE "public"."signature_requests_id_seq" OWNER TO "postgres";

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

ALTER TABLE "public"."user_quotas" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."users_public" (
    "email" "text" NOT NULL,
    "user_id" "uuid" NOT NULL,
    "username" "text" DEFAULT ''::"text",
    "description" "text" DEFAULT ''::"text",
    "pub_key" "text" NOT NULL,
    "status" "text" DEFAULT 'not_available'::"text" NOT NULL,
    "updated_at" timestamp without time zone DEFAULT "now"(),
    "avatar" "text" DEFAULT ''::"text",
    "flow_skills" "json" DEFAULT '[]'::"json",
    "node_skills" "json" DEFAULT '[]'::"json",
    "tasks_skills" "json" DEFAULT '[]'::"json"
);

ALTER TABLE "public"."users_public" OWNER TO "postgres";

COMMENT ON TABLE "public"."users_public" IS 'Profile data for each user.';

COMMENT ON COLUMN "public"."users_public"."pub_key" IS 'Public Key';

COMMENT ON COLUMN "public"."users_public"."status" IS 'I am available for work';

CREATE TABLE IF NOT EXISTS "public"."wallets" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "type" "text" DEFAULT 'ADAPTER'::"text",
    "adapter" "text" DEFAULT ''::"text",
    "public_key" character varying,
    "user_id" "uuid" NOT NULL,
    "description" "text" DEFAULT 'Wallet used for payments'::"text" NOT NULL,
    "name" "text" DEFAULT ''::"text" NOT NULL,
    "icon" "text",
    "keypair" "text"
);

ALTER TABLE "public"."wallets" OWNER TO "postgres";

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

ALTER PUBLICATION "supabase_realtime" OWNER TO "postgres";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run_logs";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."node_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."signature_requests";

REVOKE USAGE ON SCHEMA "public" FROM PUBLIC;
GRANT USAGE ON SCHEMA "public" TO "anon";
GRANT USAGE ON SCHEMA "public" TO "authenticated";
GRANT USAGE ON SCHEMA "public" TO "service_role";
GRANT ALL ON SCHEMA "public" TO PUBLIC;

GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "anon";
GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "service_role";

GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "anon";
GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "authenticated";
GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "service_role";

GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "anon";
GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "authenticated";
GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "service_role";

GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "service_role";

GRANT ALL ON TABLE "public"."apikeys" TO "anon";
GRANT ALL ON TABLE "public"."apikeys" TO "authenticated";
GRANT ALL ON TABLE "public"."apikeys" TO "service_role";
GRANT ALL ON TABLE "public"."apikeys" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run" TO "anon";
GRANT ALL ON TABLE "public"."flow_run" TO "authenticated";
GRANT ALL ON TABLE "public"."flow_run" TO "service_role";
GRANT ALL ON TABLE "public"."flow_run" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run_logs" TO "anon";
GRANT ALL ON TABLE "public"."flow_run_logs" TO "authenticated";
GRANT ALL ON TABLE "public"."flow_run_logs" TO "service_role";
GRANT ALL ON TABLE "public"."flow_run_logs" TO "flow_runner";

GRANT ALL ON TABLE "public"."flow_run_shared" TO "anon";
GRANT ALL ON TABLE "public"."flow_run_shared" TO "authenticated";
GRANT ALL ON TABLE "public"."flow_run_shared" TO "service_role";
GRANT ALL ON TABLE "public"."flow_run_shared" TO "flow_runner";

GRANT ALL ON TABLE "public"."flows" TO "anon";
GRANT ALL ON TABLE "public"."flows" TO "authenticated";
GRANT ALL ON TABLE "public"."flows" TO "service_role";
GRANT ALL ON TABLE "public"."flows" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."flows_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."flows_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."flows_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."flows_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."kvstore" TO "anon";
GRANT ALL ON TABLE "public"."kvstore" TO "authenticated";
GRANT ALL ON TABLE "public"."kvstore" TO "service_role";
GRANT ALL ON TABLE "public"."kvstore" TO "flow_runner";

GRANT ALL ON TABLE "public"."kvstore_metadata" TO "anon";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "authenticated";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "service_role";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "flow_runner";

GRANT ALL ON TABLE "public"."node_run" TO "anon";
GRANT ALL ON TABLE "public"."node_run" TO "authenticated";
GRANT ALL ON TABLE "public"."node_run" TO "service_role";
GRANT ALL ON TABLE "public"."node_run" TO "flow_runner";

GRANT ALL ON TABLE "public"."nodes" TO "anon";
GRANT ALL ON TABLE "public"."nodes" TO "authenticated";
GRANT ALL ON TABLE "public"."nodes" TO "service_role";
GRANT ALL ON TABLE "public"."nodes" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."nodes_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."nodes_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."nodes_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."nodes_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "anon";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "authenticated";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "service_role";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "flow_runner";
GRANT SELECT ON TABLE "public"."pubkey_whitelists" TO "supabase_auth_admin";

GRANT ALL ON SEQUENCE "public"."seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."signature_requests" TO "anon";
GRANT ALL ON TABLE "public"."signature_requests" TO "authenticated";
GRANT ALL ON TABLE "public"."signature_requests" TO "service_role";
GRANT ALL ON TABLE "public"."signature_requests" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."signature_requests_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."signature_requests_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."signature_requests_id_seq" TO "service_role";
GRANT SELECT,USAGE ON SEQUENCE "public"."signature_requests_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."user_quotas" TO "anon";
GRANT ALL ON TABLE "public"."user_quotas" TO "authenticated";
GRANT ALL ON TABLE "public"."user_quotas" TO "service_role";
GRANT ALL ON TABLE "public"."user_quotas" TO "flow_runner";

GRANT ALL ON TABLE "public"."users_public" TO "anon";
GRANT ALL ON TABLE "public"."users_public" TO "authenticated";
GRANT ALL ON TABLE "public"."users_public" TO "service_role";
GRANT SELECT ON TABLE "public"."users_public" TO "flow_runner";

GRANT ALL ON TABLE "public"."wallets" TO "anon";
GRANT ALL ON TABLE "public"."wallets" TO "authenticated";
GRANT ALL ON TABLE "public"."wallets" TO "service_role";
GRANT SELECT ON TABLE "public"."wallets" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."wallets_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."wallets_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."wallets_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."wallets_id_seq" TO "flow_runner";

ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON SEQUENCES  TO "postgres";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON SEQUENCES  TO "anon";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON SEQUENCES  TO "authenticated";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON SEQUENCES  TO "service_role";

ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON FUNCTIONS  TO "postgres";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON FUNCTIONS  TO "anon";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON FUNCTIONS  TO "authenticated";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON FUNCTIONS  TO "service_role";

ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON TABLES  TO "postgres";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON TABLES  TO "anon";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON TABLES  TO "authenticated";
ALTER DEFAULT PRIVILEGES FOR ROLE "postgres" IN SCHEMA "public" GRANT ALL ON TABLES  TO "service_role";

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

ALTER FUNCTION "auth"."validate_user"() OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "auth"."passwords" (
    "user_id" "uuid" NOT NULL,
    "password" character varying NOT NULL
);

ALTER TABLE "auth"."passwords" OWNER TO "postgres";

ALTER TABLE ONLY "auth"."passwords"
    ADD CONSTRAINT "passwords_pkey" PRIMARY KEY ("user_id");

CREATE OR REPLACE TRIGGER "on_auth_check_whitelists" BEFORE INSERT ON "auth"."users" FOR EACH ROW EXECUTE FUNCTION "auth"."validate_user"();

ALTER TABLE ONLY "auth"."passwords"
    ADD CONSTRAINT "fk-user_id" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

CREATE POLICY "user-pubic-storages xeg75m_0" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_1" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_2" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-public-storages xeg75m_0" ON "storage"."objects" FOR SELECT TO "authenticated", "anon" USING (("bucket_id" = 'user-public-storages'::"text"));

CREATE POLICY "user-storage w6lp96_0" ON "storage"."objects" FOR SELECT TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_1" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_2" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_3" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

GRANT ALL ON TABLE "auth"."passwords" TO "flow_runner";

GRANT UPDATE ON TABLE "auth"."users" TO "flow_runner";
