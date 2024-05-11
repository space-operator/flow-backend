INSERT INTO "storage"."buckets" ("id", "name", "public") VALUES ('user-storages', 'user-storages', FALSE), ('user-public-storages', 'user-public-storages', TRUE);

CREATE POLICY "user-pubic-storages xeg75m_0" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_1" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-pubic-storages xeg75m_2" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-public-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-public-storages xeg75m_0" ON "storage"."objects" FOR SELECT TO "anon", "authenticated" USING (("bucket_id" = 'user-public-storages'::"text"));

CREATE POLICY "user-storage w6lp96_0" ON "storage"."objects" FOR SELECT TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_1" ON "storage"."objects" FOR INSERT TO "authenticated" WITH CHECK ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_2" ON "storage"."objects" FOR UPDATE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));

CREATE POLICY "user-storage w6lp96_3" ON "storage"."objects" FOR DELETE TO "authenticated" USING ((("bucket_id" = 'user-storages'::"text") AND ("path_tokens"[1] = ("auth"."uid"())::"text")));
