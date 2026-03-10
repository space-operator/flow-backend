use super::helper::condition::{Operator, evaluate};
use flow_lib::command::prelude::*;

const NAME: &str = "filter";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("command/filter.jsonc");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
struct Input {
    array: Value,
    operator: Operator,
    #[serde(default)]
    field: Option<String>,
    #[serde(default)]
    compare_to: Option<Value>,
}

#[derive(Serialize, Debug)]
struct Output {
    filtered: Value,
    rejected: Value,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let elements = match input.array {
        Value::Array(arr) => arr,
        other => vec![other],
    };

    let mut filtered = Vec::new();
    let mut rejected = Vec::new();

    for element in elements {
        let result = evaluate(
            &element,
            input.field.as_deref(),
            &input.operator,
            input.compare_to.as_ref(),
        )?;
        if result {
            filtered.push(element);
        } else {
            rejected.push(element);
        }
    }

    Ok(Output {
        filtered: Value::Array(filtered),
        rejected: Value::Array(rejected),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_filter_gt() {
        let output = run(
            <_>::default(),
            Input {
                array: Value::Array(vec![
                    Value::U64(1),
                    Value::U64(2),
                    Value::U64(3),
                    Value::U64(4),
                    Value::U64(5),
                ]),
                operator: Operator::Gt,
                field: None,
                compare_to: Some(Value::U64(3)),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.filtered, Value::Array(vec![Value::U64(4), Value::U64(5)]));
        assert_eq!(
            output.rejected,
            Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)])
        );
    }

    #[tokio::test]
    async fn test_filter_is_not_null() {
        let output = run(
            <_>::default(),
            Input {
                array: Value::Array(vec![
                    Value::String("a".into()),
                    Value::Null,
                    Value::String("b".into()),
                    Value::Null,
                ]),
                operator: Operator::IsNotNull,
                field: None,
                compare_to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            output.filtered,
            Value::Array(vec![Value::String("a".into()), Value::String("b".into())])
        );
        assert_eq!(output.rejected, Value::Array(vec![Value::Null, Value::Null]));
    }

    #[tokio::test]
    async fn test_filter_with_field() {
        let items = Value::Array(vec![
            Value::Map(value::map! { "status" => Value::String("active".into()) }),
            Value::Map(value::map! { "status" => Value::String("inactive".into()) }),
            Value::Map(value::map! { "status" => Value::String("active".into()) }),
        ]);
        let output = run(
            <_>::default(),
            Input {
                array: items,
                operator: Operator::Eq,
                field: Some("status".to_owned()),
                compare_to: Some(Value::String("active".into())),
            },
        )
        .await
        .unwrap();
        let filtered = match &output.filtered {
            Value::Array(arr) => arr,
            _ => panic!("expected array"),
        };
        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_non_array_wraps() {
        let output = run(
            <_>::default(),
            Input {
                array: Value::U64(42),
                operator: Operator::IsNotNull,
                field: None,
                compare_to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.filtered, Value::Array(vec![Value::U64(42)]));
        assert_eq!(output.rejected, Value::Array(vec![]));
    }

    #[tokio::test]
    async fn test_empty_array() {
        let output = run(
            <_>::default(),
            Input {
                array: Value::Array(vec![]),
                operator: Operator::IsNotNull,
                field: None,
                compare_to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.filtered, Value::Array(vec![]));
        assert_eq!(output.rejected, Value::Array(vec![]));
    }

    #[actix::test]
    async fn test_filter_flow() {
        use crate::FlowGraph;
        use flow_lib::config::client::ClientConfig;
        use flow_lib::flow_run_events::event_channel;
        use flow_lib::FlowConfig;

        use cmds_std as _;

        #[derive(serde::Deserialize)]
        struct TestFile {
            flow: ClientConfig,
        }

        tracing_subscriber::fmt::try_init().ok();
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/filter.json"
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
            "filter had errors: {:?}",
            res.node_errors
        );
        assert_eq!(
            res.output["filtered"],
            Value::Array(vec![Value::U64(4), Value::U64(5)])
        );
    }
}
