CREATE ROLE "flow_runner";
ALTER ROLE "flow_runner" WITH INHERIT NOCREATEROLE CREATEDB LOGIN REPLICATION BYPASSRLS;
CREATE ROLE "nft_server";
ALTER ROLE "nft_server" WITH NOINHERIT NOCREATEROLE NOCREATEDB LOGIN BYPASSRLS;

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

ALTER SCHEMA "public" OWNER TO "postgres";

COMMENT ON SCHEMA "public" IS 'standard public schema';

CREATE EXTENSION IF NOT EXISTS "autoinc" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "http" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "moddatetime" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pg_stat_statements" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "pgjwt" WITH SCHEMA "extensions";

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA "extensions";

CREATE OR REPLACE FUNCTION "public"."assign_avatar"("user_id" "uuid") RETURNS bigint
    LANGUAGE "sql"
    AS $$
    UPDATE public.avatars_dispenser
    SET assigned_to = user_id,
        assigned_at = now()
    WHERE id = (
        SELECT id FROM public.avatars_dispenser
        WHERE assigned_to IS NULL
            AND nft_pubkey IS NULL
            AND nft_solana_net IS NULL
        LIMIT 1
    )
    RETURNING id
$$;

ALTER FUNCTION "public"."assign_avatar"("user_id" "uuid") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."assign_avatar_with_limit"("user_id" "uuid") RETURNS bigint
    LANGUAGE "sql" SECURITY DEFINER
    AS $$
    WITH free_avatars AS
    (SELECT id FROM avatars_dispenser WHERE assigned_to = user_id AND nft_mint_tx IS NULL)
    UPDATE avatars_dispenser
    SET assigned_to = user_id,
        assigned_at = now()
    WHERE 0 = (SELECT count(*) FROM free_avatars)
    AND id = (
        SELECT id FROM avatars_dispenser
        WHERE assigned_to IS NULL
            AND nft_pubkey IS NULL
            AND nft_solana_net IS NULL
        LIMIT 1
    )
    RETURNING id
$$;

ALTER FUNCTION "public"."assign_avatar_with_limit"("user_id" "uuid") OWNER TO "postgres";

SET default_tablespace = '';

SET default_table_access_method = "heap";

CREATE TABLE IF NOT EXISTS "public"."coupons" (
    "code" "text" NOT NULL,
    "created_at" timestamp without time zone DEFAULT "now"() NOT NULL,
    "owner" "uuid",
    "claimed_by" "uuid",
    "claimed_at" timestamp without time zone,
    "discount_price" numeric DEFAULT 0 NOT NULL,
    "in_use" boolean DEFAULT false
);

ALTER TABLE "public"."coupons" OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."check_coupon"("coupon" "text") RETURNS SETOF "public"."coupons"
    LANGUAGE "sql" SECURITY DEFINER
    AS $$
  select * from coupons where code = coupon and claimed_by is null
$$;

ALTER FUNCTION "public"."check_coupon"("coupon" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."claim_referral_code"("p_code" "text") RETURNS boolean
    LANGUAGE "plpgsql"
    AS $$
DECLARE
  v_referral_code_exists boolean;
  v_current_is_used boolean;
BEGIN
  -- Check if the referral code exists and is available
  SELECT EXISTS (
    SELECT 1
    FROM coupons
    WHERE code = p_code
  ) INTO v_referral_code_exists;

  IF NOT v_referral_code_exists THEN
    -- Unset the referral code if it doesn't exist
    UPDATE coupons
    SET in_use = FALSE
    WHERE code = p_code;
    RETURN FALSE;
  END IF;

  -- Update the referral code to mark it as in use or unset it
  UPDATE coupons
  SET in_use = NOT in_use
  WHERE code = p_code
  RETURNING in_use INTO v_current_is_used;

  -- Perform any additional actions or validations as needed

  -- Return the current value of isReferralCodeUsed
  RETURN v_current_is_used;
END;
$$;

ALTER FUNCTION "public"."claim_referral_code"("p_code" "text") OWNER TO "postgres";

CREATE PROCEDURE "public"."compare_campaign_renders"(IN "table1" "text", IN "table2" "text", IN "column1" "text", IN "column2" "text", IN "column3" "text")
    LANGUAGE "plpgsql"
    AS $$
BEGIN
  EXECUTE format('SELECT c.%I, c.%I, a.%I FROM %I c INNER JOIN %I a ON c.%I = a.%I', column1, column2, column3, table1, table2, column1, column1);
END;
$$;

ALTER PROCEDURE "public"."compare_campaign_renders"(IN "table1" "text", IN "table2" "text", IN "column1" "text", IN "column2" "text", IN "column3" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."generate_uid"("size" integer) RETURNS "text"
    LANGUAGE "plpgsql"
    AS $$
DECLARE
  characters TEXT := '0123456789';
  bytes BYTEA := gen_random_bytes(size);
  l INT := length(characters);
  i INT := 0;
  output TEXT := '';
BEGIN
  WHILE i < size LOOP
    output := output || substr(characters, get_byte(bytes, i) % l + 1, 1);
    i := i + 1;
  END LOOP;
  RETURN output;
END;
$$;

ALTER FUNCTION "public"."generate_uid"("size" integer) OWNER TO "postgres";

COMMENT ON FUNCTION "public"."generate_uid"("size" integer) IS '10';

CREATE OR REPLACE FUNCTION "public"."get_campaign_data"() RETURNS TABLE("nft_pubkey" "text", "new_render_id" "uuid", "current_avatar" "uuid")
    LANGUAGE "plpgsql"
    AS $$
BEGIN
  RETURN QUERY SELECT 
    c.nft_pubkey,
    c.new_render_id,
    a.render_id AS current_avatar
  FROM
    campaign_1 c
  INNER JOIN
    avatars_dispenser a ON c.nft_pubkey = a.nft_pubkey;
END; $$;

ALTER FUNCTION "public"."get_campaign_data"() OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_lastest_flow_run_id_by_node_id"("node_id" "uuid") RETURNS "uuid"
    LANGUAGE "plpgsql" IMMUTABLE
    AS $$BEGIN
  RETURN (
    SELECT
      id
    FROM
      flow_run
    WHERE
      cfg -> 'nodes' @> jsonb_build_array(jsonb_build_object('id', node_id))
    ORDER BY
      COALESCE(end_time, '-infinity') DESC
    LIMIT
      1
  );
END;$$;

ALTER FUNCTION "public"."get_lastest_flow_run_id_by_node_id"("node_id" "uuid") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_listings_with_owner"() RETURNS SETOF "record"
    LANGUAGE "sql"
    AS $$
select listings.*,to_json(owner) as owner from listings inner join users_public as owner on listings.user_id = owner.user_id;
$$;

ALTER FUNCTION "public"."get_listings_with_owner"() OWNER TO "postgres";

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

CREATE OR REPLACE FUNCTION "public"."get_mint_flow_runs_for_base"("base" bigint, "mint_flow" integer) RETURNS SETOF "public"."flow_run"
    LANGUAGE "sql"
    AS $$
  select * from flow_run
  where
    flow_id = mint_flow
    and inputs->'M'->'base_id'->>'S' = base::text
$$;

ALTER FUNCTION "public"."get_mint_flow_runs_for_base"("base" bigint, "mint_flow" integer) OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_my_avatar"() RETURNS SETOF "record"
    LANGUAGE "sql" SECURITY DEFINER
    AS $$
        SELECT id, created_at, assigned_to, assigned_at, nft_pubkey, nft_solana_net, nft_mint_tx
        FROM avatars_dispenser WHERE assigned_to = auth.uid()
    $$;

ALTER FUNCTION "public"."get_my_avatar"() OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_node_with_owner"("requested_node_id" "text") RETURNS "record"
    LANGUAGE "sql"
    AS $$select * from nodes inner join users_public as owner on nodes.user_id = owner.user_id where CAST(nodes.unique_node_id as TEXT) LIKE CAST(requested_node_id as TEXT);$$;

ALTER FUNCTION "public"."get_node_with_owner"("requested_node_id" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_node_with_owner_and_flow"("requested_node_id" "text") RETURNS SETOF "record"
    LANGUAGE "sql"
    AS $_$
SELECT
  nodes.*, owner.*, flows_array.flows
FROM nodes
LEFT JOIN users_public AS owner ON nodes.user_id = owner.user_id
LEFT JOIN LATERAL (
  SELECT ARRAY_AGG(flows) AS flows
  FROM flows
  INNER JOIN (
    SELECT flows.id, node
    FROM flows, unnest(flows.nodes) AS nodes_array, jsonb_path_query(nodes_array, '$[*].data.unique_node_id') AS node
    WHERE CAST(node AS TEXT) LIKE requested_node_id
    GROUP BY flows.id, node
  ) AS filtered ON flows.id = filtered.id
) AS flows_array ON true
WHERE CAST(nodes.unique_node_id AS TEXT) LIKE requested_node_id;
$_$;

ALTER FUNCTION "public"."get_node_with_owner_and_flow"("requested_node_id" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") RETURNS SETOF "record"
    LANGUAGE "sql"
    AS $_$
SELECT
  nodes.*, owner.*, flows_array.flows
FROM nodes
LEFT JOIN users_public AS owner ON nodes.user_id = owner.user_id
LEFT JOIN LATERAL (
  SELECT ARRAY_AGG(flows) AS flows
  FROM flows
  INNER JOIN (
    SELECT flows.id, node
    FROM flows, unnest(flows.nodes) AS nodes1, jsonb_path_query(nodes1, '$[*].data.unique_node_id') AS node
    WHERE CAST(node AS TEXT) LIKE requested_node_id
    GROUP BY flows.id, node
  ) AS filtered ON flows.id = filtered.id
) AS flows_array ON true
WHERE CAST(nodes.unique_node_id AS TEXT) LIKE requested_node_id;
$_$;

ALTER FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") OWNER TO "postgres";

COMMENT ON FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") IS 'test';

CREATE OR REPLACE FUNCTION "public"."get_nodes_with_flows_and_licenses"("requested_user_id" "text") RETURNS "record"
    LANGUAGE "sql"
    AS $_$
select 
  nodes.*,
  owner.avatar as owner_avatar,
  owner.username as owner_username,
  owner.user_id as owner_user_id,
  (select ARRAY_AGG(flows) as flows from flows inner join (select flows.id, node from flows, jsonb_path_query(flows.nodes::jsonb, '$[*].data.unique_node_id') as node where CAST(node as TEXT) like '"'||nodes.unique_node_id||'"' group by flows.id, node) as filtered on flows.id = filtered.id)
  from (nodes inner join users_public as owner on nodes.user_id = owner.user_id)
  where CAST(owner.user_id as TEXT) like requested_user_id;
  $_$;

ALTER FUNCTION "public"."get_nodes_with_flows_and_licenses"("requested_user_id" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_nodes_with_flows_and_owners"() RETURNS SETOF "record"
    LANGUAGE "sql"
    AS $_$
select 
  nodes.*,
  owner.avatar as owner_avatar,
  owner.username as owner_username,
  owner.user_id as owner_user_id,
  (select ARRAY_AGG(flows) as flows from flows inner join (select flows.id, node from flows, jsonb_path_query(flows.nodes::jsonb, '$[*].data.unique_node_id') as node where CAST(node as TEXT) like '"'||nodes.unique_node_id||'"' group by flows.id, node) as filtered on flows.id = filtered.id)
  from (nodes inner join users_public as owner on nodes.user_id = owner.user_id);
$_$;

ALTER FUNCTION "public"."get_nodes_with_flows_and_owners"() OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_nodes_with_flows_and_owners_with_user"("requested_user_id" "text") RETURNS SETOF "record"
    LANGUAGE "sql"
    AS $_$
select 
  nodes.*,
  owner.avatar as owner_avatar,
  owner.username as owner_username,
  owner.user_id as owner_user_id,
  (select ARRAY_AGG(flows) as flows from flows inner join (select flows.id, node from flows, jsonb_path_query(flows.nodes::jsonb, '$[*].data.unique_node_id') as node where CAST(node as TEXT) like '"'||nodes.unique_node_id||'"' group by flows.id, node) as filtered on flows.id = filtered.id)
  from (nodes inner join users_public as owner on nodes.user_id = owner.user_id)
  where CAST(owner.user_id as TEXT) like requested_user_id;
$_$;

ALTER FUNCTION "public"."get_nodes_with_flows_and_owners_with_user"("requested_user_id" "text") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."get_nodes_with_users"("requested_user_id" "uuid") RETURNS "record"
    LANGUAGE "sql"
    AS $$select * from nodes inner join users_public on nodes.user_id = users_public.user_id where CAST(nodes.user_id as TEXT) like CAST(requested_user_id as TEXT);$$;

ALTER FUNCTION "public"."get_nodes_with_users"("requested_user_id" "uuid") OWNER TO "postgres";

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

CREATE OR REPLACE FUNCTION "public"."increment"("row_id" integer) RETURNS "void"
    LANGUAGE "sql"
    AS $$
  update test
  set user_count = user_count + 1
  where id = row_id;
$$;

ALTER FUNCTION "public"."increment"("row_id" integer) OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."is_nft_admin"("user_id" "uuid") RETURNS boolean
    LANGUAGE "sql" STABLE SECURITY DEFINER
    AS $_$SELECT EXISTS (SELECT user_id FROM nft_admins WHERE user_id = $1);$_$;

ALTER FUNCTION "public"."is_nft_admin"("user_id" "uuid") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."reset_mint"("failed_id" "uuid") RETURNS bigint
    LANGUAGE "plpgsql" SECURITY DEFINER
    AS $$
declare failed record;
BEGIN
  select
    (inputs->'M'->'base_id'->>'S')::bigint as base,
    (output->'M'->'used_code'->>'S') as used_code
  into failed
  from flow_run
  where 
    id = failed_id
    and end_time is not null
    and errors is not null
    and array_length(errors, 1) > 0
    and user_id in (select user_id from nft_admins)
    and id not in (select flow_run_id as id from resetted);

  update avatars_dispenser
  set
    nft_pubkey = null,
    nft_solana_net = null
  where
    id = failed.base
    and nft_mint_tx is null;

  update coupons
  set
    claimed_by = null,
    claimed_at = null
  where
    code = failed.used_code;

  insert into resetted (flow_run_id) values (failed_id);

  return failed.base;
END;
$$;

ALTER FUNCTION "public"."reset_mint"("failed_id" "uuid") OWNER TO "postgres";

CREATE OR REPLACE FUNCTION "public"."set_coupon_in_use"("p_code" "text") RETURNS TABLE("is_used" boolean, "discount_price" numeric)
    LANGUAGE "plpgsql" SECURITY DEFINER
    AS $$
DECLARE
  v_referral_code_exists boolean;
  v_current_is_used boolean;
  v_discount_price numeric;
BEGIN
  -- Check if the referral code exists and is available
  SELECT EXISTS (
    SELECT 1
    FROM coupons
    WHERE code = p_code
      AND in_use = FALSE
  ) INTO v_referral_code_exists;

  IF NOT v_referral_code_exists THEN
    RETURN QUERY SELECT FALSE, 0.0;
  END IF;

  -- Update the referral code to mark it as in use
  UPDATE coupons
  SET in_use = TRUE
  WHERE code = p_code
  RETURNING in_use, coupons.discount_price INTO v_current_is_used, v_discount_price;

  -- Perform any additional actions or validations as needed

  -- Return the current value of isReferralCodeUsed and the discount price
  RETURN QUERY SELECT v_current_is_used, v_discount_price;
END;
$$;

ALTER FUNCTION "public"."set_coupon_in_use"("p_code" "text") OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."apikeys" (
    "key_hash" "text" NOT NULL,
    "user_id" "uuid" NOT NULL,
    "name" "text" NOT NULL,
    "trimmed_key" "text" NOT NULL,
    "created_at" timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

ALTER TABLE "public"."apikeys" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."avatars_dispenser" (
    "id" bigint NOT NULL,
    "created_at" timestamp without time zone DEFAULT "now"() NOT NULL,
    "render_id" "uuid" NOT NULL,
    "assigned_to" "uuid",
    "assigned_at" timestamp without time zone,
    "nft_pubkey" "text",
    "nft_solana_net" "text",
    "nft_mint_tx" "text",
    "params" "jsonb" NOT NULL
);

ALTER TABLE "public"."avatars_dispenser" OWNER TO "postgres";

ALTER TABLE "public"."avatars_dispenser" ALTER COLUMN "id" ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME "public"."avatars_dispenser_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."avatars_pruned" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"() NOT NULL,
    "render_id" "uuid" NOT NULL,
    "dispenser_id" bigint,
    "similar_to" bigint,
    "reason" "text",
    "conflict_report" "jsonb"
);

ALTER TABLE "public"."avatars_pruned" OWNER TO "postgres";

ALTER TABLE "public"."avatars_pruned" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."avatars_pruned_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."bookmarks" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "user_id" "uuid" DEFAULT "auth"."uid"(),
    "flow_id" integer NOT NULL,
    "name" "text",
    "nodes" "jsonb"[],
    "position" integer
);

ALTER TABLE "public"."bookmarks" OWNER TO "postgres";

COMMENT ON TABLE "public"."bookmarks" IS 'Nodes and edges bookmarked';

ALTER TABLE "public"."bookmarks" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."bookmarks_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."campaign_1" (
    "nft_pubkey" "text" NOT NULL,
    "new_render_params" "jsonb",
    "new_render_id" "uuid"
);

ALTER TABLE "public"."campaign_1" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."campaign_2" (
    "nft_pubkey" "text" NOT NULL,
    "new_render_params" "jsonb",
    "new_render_id" "uuid"
);

ALTER TABLE "public"."campaign_2" OWNER TO "postgres";

COMMENT ON TABLE "public"."campaign_2" IS 'This is a duplicate of campaign_1';

CREATE TABLE IF NOT EXISTS "public"."chat" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "user_id" "uuid" DEFAULT "auth"."uid"(),
    "context_id" "uuid",
    "from" "text",
    "to" "text",
    "type" "text",
    "thread_id" "json",
    "toUUID" "uuid"
);

ALTER TABLE "public"."chat" OWNER TO "postgres";

COMMENT ON COLUMN "public"."chat"."from" IS 'pubkey';

COMMENT ON COLUMN "public"."chat"."to" IS 'pubkey';

ALTER TABLE "public"."chat" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."chat_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
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

CREATE TABLE IF NOT EXISTS "public"."human_readable_effects" (
    "type" "text" NOT NULL,
    "value" "jsonb" NOT NULL,
    "pdg_name" "text" NOT NULL,
    "metaplex_name" "text" NOT NULL
);

ALTER TABLE "public"."human_readable_effects" OWNER TO "postgres";

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

CREATE TABLE IF NOT EXISTS "public"."listings" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "type" "text" DEFAULT 'flow'::"text",
    "description" "text" DEFAULT 'Description'::"text",
    "contractType" "text" DEFAULT 'fixed'::"text" NOT NULL,
    "tags" "json" DEFAULT '[]'::"json" NOT NULL,
    "user_id" "uuid" DEFAULT "auth"."uid"(),
    "price" numeric DEFAULT '0'::numeric NOT NULL,
    "sources" "json"[] DEFAULT '{}'::"json"[] NOT NULL,
    "targets" "json"[] DEFAULT '{}'::"json"[] NOT NULL,
    "useCase" "text" NOT NULL,
    "title" "text",
    "urgency" "text" DEFAULT 'Next Week'::"text",
    "privacy" "text" DEFAULT 'private'::"text",
    "updated_at" timestamp with time zone DEFAULT "now"(),
    "status" character varying DEFAULT 'active'::character varying NOT NULL,
    "owner" "json",
    "uuid" "uuid" DEFAULT "extensions"."uuid_generate_v4"() NOT NULL
);

ALTER TABLE "public"."listings" OWNER TO "postgres";

COMMENT ON TABLE "public"."listings" IS 'Listings';

COMMENT ON COLUMN "public"."listings"."status" IS 'Status of the Listing';

ALTER TABLE "public"."listings" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."listings_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

CREATE TABLE IF NOT EXISTS "public"."marketplace_bookmarks" (
    "user" "uuid" DEFAULT "auth"."uid"() NOT NULL,
    "flow_ids" "uuid"[] DEFAULT '{}'::"uuid"[] NOT NULL
);

ALTER TABLE "public"."marketplace_bookmarks" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."nft_admins" (
    "user_id" "uuid" NOT NULL
);

ALTER TABLE "public"."nft_admins" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."nft_metadata" (
    "id" "uuid" DEFAULT "extensions"."uuid_generate_v4"() NOT NULL,
    "created_at" timestamp without time zone DEFAULT "now"() NOT NULL,
    "cover_render_params" "jsonb" NOT NULL,
    "cover_render_id" "uuid" NOT NULL,
    "effects" "jsonb"[] NOT NULL,
    "renders" "jsonb"[] NOT NULL,
    "render_ids" "uuid"[] NOT NULL,
    "name" "text" NOT NULL,
    "symbol" "text" NOT NULL,
    "description" "text" NOT NULL,
    "external_url" "text" NOT NULL,
    "nft_solana_net" "text",
    "nft_pubkey" "text",
    "nft_version" integer,
    "nft_tx" "text",
    "nft_tx_time" timestamp without time zone,
    "seller_fee_basis_points" integer NOT NULL,
    "nft_delegate_record" "text"
);

ALTER TABLE "public"."nft_metadata" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."nft_owner" (
    "nft_pubkey" "text" NOT NULL,
    "nft_solana_net" "text" NOT NULL,
    "user_id" "uuid"
);

ALTER TABLE "public"."nft_owner" OWNER TO "postgres";

CREATE TABLE IF NOT EXISTS "public"."nft_referral" (
    "uuid" "uuid" DEFAULT "extensions"."uuid_generate_v4"() NOT NULL,
    "nfc" boolean DEFAULT false,
    "code" character varying NOT NULL,
    "handle" character varying,
    "date_claimed" timestamp with time zone,
    "parent" "uuid",
    "follows" boolean DEFAULT false,
    "children" "uuid"[] DEFAULT '{}'::"uuid"[] NOT NULL
);

ALTER TABLE "public"."nft_referral" OWNER TO "postgres";

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

CREATE TABLE IF NOT EXISTS "public"."proposals" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "listing_uuid" bigint,
    "from" "text" NOT NULL,
    "amount" double precision DEFAULT '0'::double precision NOT NULL,
    "proposal" "text" NOT NULL,
    "status" "text" DEFAULT 'new'::"text" NOT NULL,
    "user_id" "uuid" DEFAULT "auth"."uid"() NOT NULL
);

ALTER TABLE "public"."proposals" OWNER TO "postgres";

ALTER TABLE "public"."proposals" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."proposals_id_seq"
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

CREATE TABLE IF NOT EXISTS "public"."resetted" (
    "flow_run_id" "uuid" NOT NULL
);

ALTER TABLE "public"."resetted" OWNER TO "postgres";

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

CREATE TABLE IF NOT EXISTS "public"."tags" (
    "id" bigint NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"(),
    "name" character varying DEFAULT ''::character varying,
    "category" character varying DEFAULT ''::character varying
);

ALTER TABLE "public"."tags" OWNER TO "postgres";

ALTER TABLE "public"."tags" ALTER COLUMN "id" ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME "public"."tags_id_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);

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

CREATE TABLE IF NOT EXISTS "public"."users_private" (
    "id" "uuid" DEFAULT "gen_random_uuid"() NOT NULL,
    "created_at" timestamp with time zone DEFAULT "now"() NOT NULL,
    "user_id" "uuid" NOT NULL,
    "dark_mode" boolean DEFAULT true NOT NULL
);

ALTER TABLE "public"."users_private" OWNER TO "postgres";

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

ALTER TABLE ONLY "public"."avatars_dispenser"
    ADD CONSTRAINT "avatars_dispenser_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."avatars_dispenser"
    ADD CONSTRAINT "avatars_dispenser_render_id_key" UNIQUE ("render_id");

ALTER TABLE ONLY "public"."avatars_pruned"
    ADD CONSTRAINT "avatars_pruned_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."avatars_pruned"
    ADD CONSTRAINT "avatars_pruned_render_id_key" UNIQUE ("render_id");

ALTER TABLE ONLY "public"."bookmarks"
    ADD CONSTRAINT "bookmarks_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."campaign_1"
    ADD CONSTRAINT "campaign_1_pkey" PRIMARY KEY ("nft_pubkey");

ALTER TABLE ONLY "public"."campaign_2"
    ADD CONSTRAINT "campaign_2_pkey" PRIMARY KEY ("nft_pubkey");

ALTER TABLE ONLY "public"."chat"
    ADD CONSTRAINT "chat_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."coupons"
    ADD CONSTRAINT "coupons_pkey" PRIMARY KEY ("code");

ALTER TABLE ONLY "public"."flow_run_logs"
    ADD CONSTRAINT "flow_run_logs_pkey" PRIMARY KEY ("flow_run_id", "log_index");

ALTER TABLE ONLY "public"."flow_run"
    ADD CONSTRAINT "flow_run_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."flow_run_shared"
    ADD CONSTRAINT "flow_run_shared_pkey" PRIMARY KEY ("flow_run_id", "user_id");

ALTER TABLE ONLY "public"."flows"
    ADD CONSTRAINT "flows_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."human_readable_effects"
    ADD CONSTRAINT "human_readable_effects_pkey" PRIMARY KEY ("type", "value");

ALTER TABLE ONLY "public"."kvstore_metadata"
    ADD CONSTRAINT "kvstore_metadata_pkey" PRIMARY KEY ("user_id", "store_name");

ALTER TABLE ONLY "public"."listings"
    ADD CONSTRAINT "listings_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."marketplace_bookmarks"
    ADD CONSTRAINT "marketplace_bookmarks_pkey" PRIMARY KEY ("user");

ALTER TABLE ONLY "public"."nft_admins"
    ADD CONSTRAINT "nft_admins_pkey" PRIMARY KEY ("user_id");

ALTER TABLE ONLY "public"."nft_metadata"
    ADD CONSTRAINT "nft_metadata_nft_solana_net_nft_pubkey_nft_version_key" UNIQUE ("nft_solana_net", "nft_pubkey", "nft_version");

ALTER TABLE ONLY "public"."nft_metadata"
    ADD CONSTRAINT "nft_metadata_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."nft_owner"
    ADD CONSTRAINT "nft_owner_pkey" PRIMARY KEY ("nft_pubkey", "nft_solana_net");

ALTER TABLE ONLY "public"."nft_referral"
    ADD CONSTRAINT "nft_referral_code_key" UNIQUE ("code");

ALTER TABLE ONLY "public"."nft_referral"
    ADD CONSTRAINT "nft_referral_handle_key" UNIQUE ("handle");

ALTER TABLE ONLY "public"."nft_referral"
    ADD CONSTRAINT "nft_referral_pkey" PRIMARY KEY ("uuid");

ALTER TABLE ONLY "public"."nft_referral"
    ADD CONSTRAINT "nft_referral_uuid_key" UNIQUE ("uuid");

ALTER TABLE ONLY "public"."node_run"
    ADD CONSTRAINT "node_run_pkey" PRIMARY KEY ("flow_run_id", "node_id", "times");

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_unique_node_id_key" UNIQUE ("unique_node_id");

ALTER TABLE ONLY "public"."proposals"
    ADD CONSTRAINT "proposals_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "pubkey_unique" UNIQUE ("pub_key");

ALTER TABLE ONLY "public"."pubkey_whitelists"
    ADD CONSTRAINT "pubkey_whitelists_pkey" PRIMARY KEY ("pubkey");

ALTER TABLE ONLY "public"."resetted"
    ADD CONSTRAINT "resetted_pkey" PRIMARY KEY ("flow_run_id");

ALTER TABLE ONLY "public"."signature_requests"
    ADD CONSTRAINT "signature_requests_pkey" PRIMARY KEY ("user_id", "id");

ALTER TABLE ONLY "public"."tags"
    ADD CONSTRAINT "tags_id_key" UNIQUE ("id");

ALTER TABLE ONLY "public"."tags"
    ADD CONSTRAINT "tags_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."apikeys"
    ADD CONSTRAINT "uc-user_id-name" UNIQUE ("user_id", "name");

ALTER TABLE ONLY "public"."kvstore"
    ADD CONSTRAINT "uq_user_id_store_name_key" PRIMARY KEY ("user_id", "store_name", "key");

ALTER TABLE ONLY "public"."user_quotas"
    ADD CONSTRAINT "user_quotas_pkey" PRIMARY KEY ("user_id");

ALTER TABLE ONLY "public"."users_private"
    ADD CONSTRAINT "users_private_pkey" PRIMARY KEY ("id");

ALTER TABLE ONLY "public"."users_private"
    ADD CONSTRAINT "users_private_user_id_key" UNIQUE ("user_id");

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

CREATE INDEX "nft_idx" ON "public"."nft_metadata" USING "btree" ("nft_solana_net", "nft_pubkey");

CREATE OR REPLACE TRIGGER "New Flow" AFTER INSERT ON "public"."flows" FOR EACH ROW EXECUTE FUNCTION "supabase_functions"."http_request"('https://hooks.slack.com/services/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', 'POST', '{"Content-type":"application/json"}', '{"payload":"new flow"}', '1000');

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."flows" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('updated_at');

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."kvstore" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('last_updated');

CREATE OR REPLACE TRIGGER "handle_updated_at" BEFORE UPDATE ON "public"."users_public" FOR EACH ROW EXECUTE FUNCTION "extensions"."moddatetime"('updated_at');

ALTER TABLE ONLY "public"."avatars_dispenser"
    ADD CONSTRAINT "avatars_dispenser_assigned_to_fkey" FOREIGN KEY ("assigned_to") REFERENCES "auth"."users"("id") ON DELETE SET NULL;

ALTER TABLE ONLY "public"."bookmarks"
    ADD CONSTRAINT "bookmarks_flow_id_fkey" FOREIGN KEY ("flow_id") REFERENCES "public"."flows"("id");

ALTER TABLE ONLY "public"."bookmarks"
    ADD CONSTRAINT "bookmarks_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."chat"
    ADD CONSTRAINT "chat_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."coupons"
    ADD CONSTRAINT "coupons_claimed_by_fkey" FOREIGN KEY ("claimed_by") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."coupons"
    ADD CONSTRAINT "coupons_owner_fkey" FOREIGN KEY ("owner") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

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

ALTER TABLE ONLY "public"."listings"
    ADD CONSTRAINT "listings_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."nft_admins"
    ADD CONSTRAINT "nft_admins_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."nft_owner"
    ADD CONSTRAINT "nft_owner_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE SET NULL;

ALTER TABLE ONLY "public"."nodes"
    ADD CONSTRAINT "nodes_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."proposals"
    ADD CONSTRAINT "proposals_listing_uuid_fkey" FOREIGN KEY ("listing_uuid") REFERENCES "public"."listings"("id");

ALTER TABLE ONLY "public"."proposals"
    ADD CONSTRAINT "proposals_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id");

ALTER TABLE ONLY "public"."resetted"
    ADD CONSTRAINT "resetted_flow_run_id_fkey" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."signature_requests"
    ADD CONSTRAINT "signature_requests_flow_run_id_fkey" FOREIGN KEY ("flow_run_id") REFERENCES "public"."flow_run"("id") ON DELETE SET NULL;

ALTER TABLE ONLY "public"."user_quotas"
    ADD CONSTRAINT "user_quotas_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."users_private"
    ADD CONSTRAINT "users_private_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE ONLY "public"."users_public"
    ADD CONSTRAINT "users_public_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

ALTER TABLE ONLY "public"."wallets"
    ADD CONSTRAINT "wallets_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "auth"."users"("id") ON DELETE CASCADE;

CREATE POLICY "Allow insert for all users" ON "public"."nft_referral" FOR INSERT WITH CHECK (true);

CREATE POLICY "Allow update for all users" ON "public"."listings" FOR UPDATE USING (true) WITH CHECK (true);

CREATE POLICY "Enable delete for users based on user_id" ON "public"."listings" FOR DELETE USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable delete for users based on user_id" ON "public"."proposals" FOR DELETE USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable delete for users based on user_id" ON "public"."wallets" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable insert for authenticated users only" ON "public"."chat" FOR INSERT TO "authenticated" WITH CHECK (true);

CREATE POLICY "Enable insert for authenticated users only" ON "public"."listings" FOR INSERT TO "authenticated" WITH CHECK (true);

CREATE POLICY "Enable insert for authenticated users only" ON "public"."proposals" FOR INSERT TO "authenticated" WITH CHECK (true);

CREATE POLICY "Enable insert for authenticated users only" ON "public"."wallets" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable read access for all users" ON "public"."chat" FOR SELECT USING (true);

CREATE POLICY "Enable read access for all users" ON "public"."listings" FOR SELECT USING (true);

CREATE POLICY "Enable read access for all users" ON "public"."nft_referral" FOR SELECT USING (true);

CREATE POLICY "Enable read access for all users" ON "public"."proposals" FOR SELECT USING (true);

CREATE POLICY "Enable read access for all users" ON "public"."users_public" FOR SELECT USING (true);

CREATE POLICY "Enable read access for authenticated users" ON "public"."wallets" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable update for all users" ON "public"."nft_referral" FOR UPDATE USING (true);

CREATE POLICY "Enable update for users based on user_id" ON "public"."users_public" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "Enable update for users based on user_id" ON "public"."wallets" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "anon-select" ON "public"."flows" FOR SELECT TO "anon" USING (("isPublic" = true));

CREATE POLICY "anon-select" ON "public"."nodes" FOR SELECT TO "anon" USING (("isPublic" = true));

ALTER TABLE "public"."apikeys" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "authenticated" ON "public"."campaign_2" TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "authenticated,anon-select" ON "public"."human_readable_effects" FOR SELECT TO "anon", "authenticated" USING (true);

CREATE POLICY "authenticated-all" ON "public"."avatars_dispenser" TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "authenticated-all" ON "public"."campaign_1" TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "authenticated-all" ON "public"."nft_metadata" TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "authenticated-all" ON "public"."users_private" TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-delete" ON "public"."bookmarks" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-delete" ON "public"."flows" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-delete" ON "public"."nodes" FOR DELETE TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-insert" ON "public"."bookmarks" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-insert" ON "public"."flows" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-insert" ON "public"."nodes" FOR INSERT TO "authenticated" WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select" ON "public"."apikeys" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select" ON "public"."bookmarks" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR (EXISTS ( SELECT 1
   FROM "public"."flows" "f"
  WHERE (("f"."id" = "bookmarks"."flow_id") AND "f"."isPublic")))));

CREATE POLICY "authenticated-select" ON "public"."coupons" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "owner"));

CREATE POLICY "authenticated-select" ON "public"."flows" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true)));

CREATE POLICY "authenticated-select" ON "public"."nft_metadata" FOR SELECT TO "authenticated" USING (true);

CREATE POLICY "authenticated-select" ON "public"."nft_owner" FOR SELECT TO "authenticated" USING (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-select" ON "public"."nodes" FOR SELECT TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true)));

CREATE POLICY "authenticated-select" ON "public"."resetted" FOR SELECT TO "authenticated" USING (true);

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

CREATE POLICY "authenticated-update" ON "public"."bookmarks" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-update" ON "public"."flows" FOR UPDATE TO "authenticated" USING (("auth"."uid"() = "user_id")) WITH CHECK (("auth"."uid"() = "user_id"));

CREATE POLICY "authenticated-update" ON "public"."nodes" FOR UPDATE TO "authenticated" USING ((("auth"."uid"() = "user_id") OR ("isPublic" = true))) WITH CHECK ((("type")::"text" <> 'native'::"text"));

ALTER TABLE "public"."avatars_dispenser" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."bookmarks" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."campaign_1" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."campaign_2" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."chat" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."coupons" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flow_run" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flow_run_logs" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flow_run_shared" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."flows" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."human_readable_effects" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."kvstore" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."kvstore_metadata" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."listings" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."marketplace_bookmarks" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."nft_admins" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "nft_admins-all" ON "public"."coupons" TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "nft_admins-select" ON "public"."user_quotas" FOR SELECT TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

CREATE POLICY "nft_admins-update" ON "public"."user_quotas" FOR UPDATE TO "authenticated" USING ("public"."is_nft_admin"("auth"."uid"()));

ALTER TABLE "public"."nft_metadata" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."nft_owner" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."nft_referral" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."node_run" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."nodes" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."proposals" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."pubkey_whitelists" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."resetted" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."signature_requests" ENABLE ROW LEVEL SECURITY;

CREATE POLICY "supabase_auth_admin-select-pubkey_whitelists" ON "public"."pubkey_whitelists" FOR SELECT TO "supabase_auth_admin" USING (true);

ALTER TABLE "public"."user_quotas" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."users_private" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."users_public" ENABLE ROW LEVEL SECURITY;

ALTER TABLE "public"."wallets" ENABLE ROW LEVEL SECURITY;

ALTER PUBLICATION "supabase_realtime" OWNER TO "postgres";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."flow_run_logs";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."node_run";

ALTER PUBLICATION "supabase_realtime" ADD TABLE ONLY "public"."signature_requests";

REVOKE USAGE ON SCHEMA "public" FROM PUBLIC;
GRANT ALL ON SCHEMA "public" TO PUBLIC;
GRANT USAGE ON SCHEMA "public" TO "anon";
GRANT USAGE ON SCHEMA "public" TO "authenticated";
GRANT USAGE ON SCHEMA "public" TO "service_role";

GRANT ALL ON FUNCTION "public"."assign_avatar"("user_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."assign_avatar"("user_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."assign_avatar"("user_id" "uuid") TO "service_role";

GRANT ALL ON FUNCTION "public"."assign_avatar_with_limit"("user_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."assign_avatar_with_limit"("user_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."assign_avatar_with_limit"("user_id" "uuid") TO "service_role";

GRANT ALL ON TABLE "public"."coupons" TO "anon";
GRANT ALL ON TABLE "public"."coupons" TO "authenticated";
GRANT ALL ON TABLE "public"."coupons" TO "service_role";

GRANT ALL ON FUNCTION "public"."check_coupon"("coupon" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."check_coupon"("coupon" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."check_coupon"("coupon" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."claim_referral_code"("p_code" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."claim_referral_code"("p_code" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."claim_referral_code"("p_code" "text") TO "service_role";

GRANT ALL ON PROCEDURE "public"."compare_campaign_renders"(IN "table1" "text", IN "table2" "text", IN "column1" "text", IN "column2" "text", IN "column3" "text") TO "anon";
GRANT ALL ON PROCEDURE "public"."compare_campaign_renders"(IN "table1" "text", IN "table2" "text", IN "column1" "text", IN "column2" "text", IN "column3" "text") TO "authenticated";
GRANT ALL ON PROCEDURE "public"."compare_campaign_renders"(IN "table1" "text", IN "table2" "text", IN "column1" "text", IN "column2" "text", IN "column3" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."generate_uid"("size" integer) TO "anon";
GRANT ALL ON FUNCTION "public"."generate_uid"("size" integer) TO "authenticated";
GRANT ALL ON FUNCTION "public"."generate_uid"("size" integer) TO "service_role";

GRANT ALL ON FUNCTION "public"."get_campaign_data"() TO "anon";
GRANT ALL ON FUNCTION "public"."get_campaign_data"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_campaign_data"() TO "service_role";

GRANT ALL ON FUNCTION "public"."get_lastest_flow_run_id_by_node_id"("node_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."get_lastest_flow_run_id_by_node_id"("node_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_lastest_flow_run_id_by_node_id"("node_id" "uuid") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_listings_with_owner"() TO "anon";
GRANT ALL ON FUNCTION "public"."get_listings_with_owner"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_listings_with_owner"() TO "service_role";

GRANT ALL ON TABLE "public"."flow_run" TO "anon";
GRANT ALL ON TABLE "public"."flow_run" TO "authenticated";
GRANT ALL ON TABLE "public"."flow_run" TO "service_role";
GRANT ALL ON TABLE "public"."flow_run" TO "flow_runner";

GRANT ALL ON FUNCTION "public"."get_mint_flow_runs_for_base"("base" bigint, "mint_flow" integer) TO "anon";
GRANT ALL ON FUNCTION "public"."get_mint_flow_runs_for_base"("base" bigint, "mint_flow" integer) TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_mint_flow_runs_for_base"("base" bigint, "mint_flow" integer) TO "service_role";

GRANT ALL ON FUNCTION "public"."get_my_avatar"() TO "anon";
GRANT ALL ON FUNCTION "public"."get_my_avatar"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_my_avatar"() TO "service_role";

GRANT ALL ON FUNCTION "public"."get_node_with_owner"("requested_node_id" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."get_node_with_owner"("requested_node_id" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_node_with_owner"("requested_node_id" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow"("requested_node_id" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow"("requested_node_id" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow"("requested_node_id" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_node_with_owner_and_flow2"("requested_node_id" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_licenses"("requested_user_id" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_licenses"("requested_user_id" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_licenses"("requested_user_id" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners"() TO "anon";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners"() TO "service_role";

GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners_with_user"("requested_user_id" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners_with_user"("requested_user_id" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_nodes_with_flows_and_owners_with_user"("requested_user_id" "text") TO "service_role";

GRANT ALL ON FUNCTION "public"."get_nodes_with_users"("requested_user_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."get_nodes_with_users"("requested_user_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."get_nodes_with_users"("requested_user_id" "uuid") TO "service_role";

GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "anon";
GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "authenticated";
GRANT ALL ON FUNCTION "public"."handle_new_user"() TO "service_role";

GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "anon";
GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "authenticated";
GRANT ALL ON FUNCTION "public"."increase_credit"("user_id" "uuid", "amount" bigint) TO "service_role";

GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "anon";
GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "authenticated";
GRANT ALL ON FUNCTION "public"."increase_used_credit"("user_id" "uuid", "amount" bigint) TO "service_role";

GRANT ALL ON FUNCTION "public"."increment"("row_id" integer) TO "anon";
GRANT ALL ON FUNCTION "public"."increment"("row_id" integer) TO "authenticated";
GRANT ALL ON FUNCTION "public"."increment"("row_id" integer) TO "service_role";

GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."is_nft_admin"("user_id" "uuid") TO "service_role";

GRANT ALL ON FUNCTION "public"."reset_mint"("failed_id" "uuid") TO "anon";
GRANT ALL ON FUNCTION "public"."reset_mint"("failed_id" "uuid") TO "authenticated";
GRANT ALL ON FUNCTION "public"."reset_mint"("failed_id" "uuid") TO "service_role";

GRANT ALL ON FUNCTION "public"."set_coupon_in_use"("p_code" "text") TO "anon";
GRANT ALL ON FUNCTION "public"."set_coupon_in_use"("p_code" "text") TO "authenticated";
GRANT ALL ON FUNCTION "public"."set_coupon_in_use"("p_code" "text") TO "service_role";

GRANT ALL ON TABLE "public"."apikeys" TO "anon";
GRANT ALL ON TABLE "public"."apikeys" TO "authenticated";
GRANT ALL ON TABLE "public"."apikeys" TO "service_role";
GRANT ALL ON TABLE "public"."apikeys" TO "flow_runner";

GRANT ALL ON TABLE "public"."avatars_dispenser" TO "anon";
GRANT ALL ON TABLE "public"."avatars_dispenser" TO "authenticated";
GRANT ALL ON TABLE "public"."avatars_dispenser" TO "service_role";
GRANT SELECT ON TABLE "public"."avatars_dispenser" TO "nft_server";

GRANT ALL ON SEQUENCE "public"."avatars_dispenser_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."avatars_dispenser_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."avatars_dispenser_id_seq" TO "service_role";

GRANT ALL ON TABLE "public"."avatars_pruned" TO "anon";
GRANT ALL ON TABLE "public"."avatars_pruned" TO "authenticated";
GRANT ALL ON TABLE "public"."avatars_pruned" TO "service_role";

GRANT ALL ON SEQUENCE "public"."avatars_pruned_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."avatars_pruned_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."avatars_pruned_id_seq" TO "service_role";

GRANT ALL ON TABLE "public"."bookmarks" TO "anon";
GRANT ALL ON TABLE "public"."bookmarks" TO "authenticated";
GRANT ALL ON TABLE "public"."bookmarks" TO "service_role";
GRANT SELECT ON TABLE "public"."bookmarks" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."bookmarks_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."bookmarks_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."bookmarks_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."bookmarks_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."campaign_1" TO "anon";
GRANT ALL ON TABLE "public"."campaign_1" TO "authenticated";
GRANT ALL ON TABLE "public"."campaign_1" TO "service_role";

GRANT ALL ON TABLE "public"."campaign_2" TO "anon";
GRANT ALL ON TABLE "public"."campaign_2" TO "authenticated";
GRANT ALL ON TABLE "public"."campaign_2" TO "service_role";

GRANT ALL ON TABLE "public"."chat" TO "anon";
GRANT ALL ON TABLE "public"."chat" TO "authenticated";
GRANT ALL ON TABLE "public"."chat" TO "service_role";
GRANT SELECT ON TABLE "public"."chat" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."chat_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."chat_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."chat_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."chat_id_seq" TO "flow_runner";

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

GRANT ALL ON TABLE "public"."human_readable_effects" TO "anon";
GRANT ALL ON TABLE "public"."human_readable_effects" TO "authenticated";
GRANT ALL ON TABLE "public"."human_readable_effects" TO "service_role";

GRANT ALL ON TABLE "public"."kvstore" TO "anon";
GRANT ALL ON TABLE "public"."kvstore" TO "authenticated";
GRANT ALL ON TABLE "public"."kvstore" TO "service_role";
GRANT ALL ON TABLE "public"."kvstore" TO "flow_runner";

GRANT ALL ON TABLE "public"."kvstore_metadata" TO "anon";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "authenticated";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "service_role";
GRANT ALL ON TABLE "public"."kvstore_metadata" TO "flow_runner";

GRANT ALL ON TABLE "public"."listings" TO "anon";
GRANT ALL ON TABLE "public"."listings" TO "authenticated";
GRANT ALL ON TABLE "public"."listings" TO "service_role";
GRANT SELECT ON TABLE "public"."listings" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."listings_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."listings_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."listings_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."listings_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."marketplace_bookmarks" TO "anon";
GRANT ALL ON TABLE "public"."marketplace_bookmarks" TO "authenticated";
GRANT ALL ON TABLE "public"."marketplace_bookmarks" TO "service_role";

GRANT ALL ON TABLE "public"."nft_admins" TO "anon";
GRANT ALL ON TABLE "public"."nft_admins" TO "authenticated";
GRANT ALL ON TABLE "public"."nft_admins" TO "service_role";

GRANT ALL ON TABLE "public"."nft_metadata" TO "anon";
GRANT ALL ON TABLE "public"."nft_metadata" TO "authenticated";
GRANT ALL ON TABLE "public"."nft_metadata" TO "service_role";
GRANT SELECT ON TABLE "public"."nft_metadata" TO "nft_server";

GRANT ALL ON TABLE "public"."nft_owner" TO "anon";
GRANT ALL ON TABLE "public"."nft_owner" TO "authenticated";
GRANT ALL ON TABLE "public"."nft_owner" TO "service_role";

GRANT ALL ON TABLE "public"."nft_referral" TO "anon";
GRANT ALL ON TABLE "public"."nft_referral" TO "authenticated";
GRANT ALL ON TABLE "public"."nft_referral" TO "service_role";
GRANT SELECT ON TABLE "public"."nft_referral" TO "flow_runner";

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

GRANT ALL ON TABLE "public"."proposals" TO "anon";
GRANT ALL ON TABLE "public"."proposals" TO "authenticated";
GRANT ALL ON TABLE "public"."proposals" TO "service_role";
GRANT SELECT ON TABLE "public"."proposals" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."proposals_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."proposals_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."proposals_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."proposals_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "anon";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "authenticated";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "service_role";
GRANT ALL ON TABLE "public"."pubkey_whitelists" TO "flow_runner";
GRANT SELECT ON TABLE "public"."pubkey_whitelists" TO "supabase_auth_admin";

GRANT ALL ON TABLE "public"."resetted" TO "anon";
GRANT ALL ON TABLE "public"."resetted" TO "authenticated";
GRANT ALL ON TABLE "public"."resetted" TO "service_role";

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

GRANT ALL ON TABLE "public"."tags" TO "anon";
GRANT ALL ON TABLE "public"."tags" TO "authenticated";
GRANT ALL ON TABLE "public"."tags" TO "service_role";
GRANT SELECT ON TABLE "public"."tags" TO "flow_runner";

GRANT ALL ON SEQUENCE "public"."tags_id_seq" TO "anon";
GRANT ALL ON SEQUENCE "public"."tags_id_seq" TO "authenticated";
GRANT ALL ON SEQUENCE "public"."tags_id_seq" TO "service_role";
GRANT SELECT ON SEQUENCE "public"."tags_id_seq" TO "flow_runner";

GRANT ALL ON TABLE "public"."user_quotas" TO "anon";
GRANT ALL ON TABLE "public"."user_quotas" TO "authenticated";
GRANT ALL ON TABLE "public"."user_quotas" TO "service_role";
GRANT ALL ON TABLE "public"."user_quotas" TO "flow_runner";

GRANT ALL ON TABLE "public"."users_private" TO "anon";
GRANT ALL ON TABLE "public"."users_private" TO "authenticated";
GRANT ALL ON TABLE "public"."users_private" TO "service_role";

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

RESET ALL;
