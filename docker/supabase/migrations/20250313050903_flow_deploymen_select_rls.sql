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
