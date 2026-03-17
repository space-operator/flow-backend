-- Rename camelCase "isPublic" to snake_case is_public on flows_v2.
-- ALTER RENAME auto-updates indexes; RLS policy expressions must be recreated.

ALTER TABLE public.flows_v2 RENAME COLUMN "isPublic" TO is_public;

DROP POLICY IF EXISTS "public-select" ON public.flows_v2;
CREATE POLICY "public-select" ON public.flows_v2
    FOR SELECT TO anon USING (is_public = true);
