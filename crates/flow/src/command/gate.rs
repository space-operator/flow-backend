use flow_lib::command::prelude::*;

const NAME: &str = "gate";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("command/gate.jsonc");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
struct Input {
    condition: bool,
    value: Value,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rejected: Option<Value>,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.condition {
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
    async fn test_true_passes_value() {
        let output = run(
            <_>::default(),
            Input {
                condition: true,
                value: Value::U64(42),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, Some(Value::U64(42)));
        assert_eq!(output.rejected, None);
    }

    #[tokio::test]
    async fn test_false_rejects_value() {
        let output = run(
            <_>::default(),
            Input {
                condition: false,
                value: Value::U64(42),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.output, None);
        assert_eq!(output.rejected, Some(Value::U64(42)));
    }

    #[tokio::test]
    async fn test_skip_serializing_none() {
        let output = Output {
            output: Some(Value::String("hello".into())),
            rejected: None,
        };
        let json = serde_json::to_value(&output).unwrap();
        assert!(json.get("output").is_some());
        assert!(json.get("rejected").is_none());
    }
}
