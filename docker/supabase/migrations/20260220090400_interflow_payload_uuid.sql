-- Rewrite legacy interflow payload fields from form_data.id to config.flow_id
-- (tagged value format: {"S": "<uuid>"}).
-- The old format stores form_data.id as an integer (old serial flow ID).
-- We look up the UUID from flows_v2 and wrap it in Value::String tagged format.

WITH transformed AS (
    SELECT
        f.id,
        (
            SELECT jsonb_agg(
                CASE
                    WHEN (node #>> '{data,node_id}') IN (
                        'interflow',
                        'interflow_instructions',
                        '@spo/interflow',
                        '@spo/interflow_instructions'
                    )
                    AND (node #> '{data,config,flow_id}') IS NULL
                    AND (node #>> '{data,targets_form,form_data,id}') IS NOT NULL
                    THEN jsonb_set(
                        node,
                        '{data,config,flow_id}',
                        jsonb_build_object('S', (
                            SELECT ref_f.uuid::text
                            FROM flows_v2 ref_f
                            WHERE ref_f.id = (node #>> '{data,targets_form,form_data,id}')::integer
                        )),
                        true
                    )
                    ELSE node
                END
            )
            FROM jsonb_array_elements(f.nodes) AS node
        ) AS nodes_new
    FROM public.flows_v2 f
    WHERE jsonb_typeof(f.nodes) = 'array'
      AND EXISTS (
          SELECT 1
          FROM jsonb_array_elements(f.nodes) AS node
          WHERE (node #>> '{data,node_id}') IN (
              'interflow',
              'interflow_instructions',
              '@spo/interflow',
              '@spo/interflow_instructions'
          )
          AND (node #> '{data,config,flow_id}') IS NULL
          AND (node #>> '{data,targets_form,form_data,id}') IS NOT NULL
      )
)
UPDATE public.flows_v2 f
SET nodes = t.nodes_new
FROM transformed t
WHERE f.id = t.id;

WITH transformed AS (
    SELECT
        d.deployment_id,
        d.flow_id,
        (
            SELECT jsonb_agg(
                CASE
                    WHEN (node #>> '{data,node_id}') IN (
                        'interflow',
                        'interflow_instructions',
                        '@spo/interflow',
                        '@spo/interflow_instructions'
                    )
                    AND (node #> '{data,config,flow_id}') IS NULL
                    AND (node #>> '{data,targets_form,form_data,id}') IS NOT NULL
                    THEN jsonb_set(
                        node,
                        '{data,config,flow_id}',
                        jsonb_build_object('S', (
                            SELECT ref_f.uuid::text
                            FROM flows_v2 ref_f
                            WHERE ref_f.id = (node #>> '{data,targets_form,form_data,id}')::integer
                        )),
                        true
                    )
                    ELSE node
                END
            )
            FROM jsonb_array_elements(d.data->'nodes') AS node
        ) AS nodes_new
    FROM public.flow_deployments_flows d
    WHERE jsonb_typeof(d.data->'nodes') = 'array'
      AND EXISTS (
          SELECT 1
          FROM jsonb_array_elements(d.data->'nodes') AS node
          WHERE (node #>> '{data,node_id}') IN (
              'interflow',
              'interflow_instructions',
              '@spo/interflow',
              '@spo/interflow_instructions'
          )
          AND (node #> '{data,config,flow_id}') IS NULL
          AND (node #>> '{data,targets_form,form_data,id}') IS NOT NULL
      )
)
UPDATE public.flow_deployments_flows d
SET data = jsonb_set(d.data, '{nodes}', COALESCE(t.nodes_new, '[]'::jsonb), false)
FROM transformed t
WHERE d.deployment_id = t.deployment_id
  AND d.flow_id = t.flow_id;
