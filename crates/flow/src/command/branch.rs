use super::helper::condition::{Operator, evaluate};
use flow_lib::command::prelude::*;

const NAME: &str = "branch";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("command/branch.jsonc");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
struct Input {
    value: Value,
    operator: Operator,
    #[serde(default)]
    field: Option<String>,
    #[serde(default)]
    compare_to: Option<Value>,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejected: Option<Value>,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let result = evaluate(
        &input.value,
        input.field.as_deref(),
        &input.operator,
        input.compare_to.as_ref(),
    )?;

    if result {
        Ok(Output {
            output: Some(input.value),
            rejected: None,
        })
    } else {
        Ok(Output {
            output: None,
            rejected: Some(input.value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_true_routes_to_output() {
        let output = run(
            <_>::default(),
            Input {
                value: Value::U64(42),
                operator: Operator::Gt,
                field: None,
                compare_to: Some(Value::U64(10)),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, Some(Value::U64(42)));
        assert_eq!(output.rejected, None);
    }

    #[tokio::test]
    async fn test_false_routes_to_rejected() {
        let output = run(
            <_>::default(),
            Input {
                value: Value::U64(5),
                operator: Operator::Gt,
                field: None,
                compare_to: Some(Value::U64(10)),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, None);
        assert_eq!(output.rejected, Some(Value::U64(5)));
    }

    #[tokio::test]
    async fn test_is_null_on_null() {
        let output = run(
            <_>::default(),
            Input {
                value: Value::Null,
                operator: Operator::IsNull,
                field: None,
                compare_to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, Some(Value::Null));
        assert_eq!(output.rejected, None);
    }

    #[tokio::test]
    async fn test_field_path() {
        let value = Value::Map(value::map! {
            "user" => Value::Map(value::map! {
                "active" => Value::Bool(true),
            }),
        });
        let output = run(
            <_>::default(),
            Input {
                value: value.clone(),
                operator: Operator::IsTrue,
                field: Some("user.active".to_owned()),
                compare_to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, Some(value));
    }

    #[tokio::test]
    async fn test_skip_serializing_none() {
        let output = Output {
            output: Some(Value::U64(1)),
            rejected: None,
        };
        let json = serde_json::to_value(&output).unwrap();
        assert!(json.get("output").is_some());
        assert!(json.get("rejected").is_none()); // skipped
    }

    #[actix::test]
    async fn test_branch_flow() {
        use crate::FlowGraph;
        use flow_lib::FlowConfig;
        use flow_lib::config::client::ClientConfig;
        use flow_lib::flow_run_events::event_channel;

        use cmds_std as _;

        #[derive(serde::Deserialize)]
        struct TestFile {
            flow: ClientConfig,
        }

        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/branch.json"
        ));
        let flow_config = FlowConfig::new(serde_json::from_str::<TestFile>(json).unwrap().flow);
        let mut flow = FlowGraph::from_cfg(flow_config, <_>::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = event_channel();
        let res = flow
            .run(
                tx,
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
                <_>::default(),
            )
            .await;
        assert!(
            res.node_errors.is_empty(),
            "branch had errors: {:?}",
            res.node_errors
        );
        assert_eq!(res.output["result"], Value::U64(42));
    }
}
