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
