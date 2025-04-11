use flow_lib::command::prelude::*;

const JSON_INSERT: &str = "json_insert";

#[derive(Deserialize, Debug)]
struct Input {
    json_input: Value,
    path: String,
    value: Value,
}

#[derive(Serialize, Debug)]
struct Output {
    updated_json: Value,
}

async fn run(_: CommandContextX, mut input: Input) -> Result<Output, CommandError> {
    let path = value::crud::path::Path::parse(&input.path)?;
    value::crud::insert(&mut input.json_input, &path.segments, input.value)?;

    Ok(Output {
        updated_json: input.json_input,
    })
}

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("json_insert.json"))?
            .check_name(JSON_INSERT)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(JSON_INSERT, |_| build()));

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_insert() {
        let input = value::map! {
            "json_input" => value::map! {
                "a" => 1,
                "b" => value::map! {
                    "c" => value::map! {}
                }
            },
            "path" => "/b/c",
            "value" => value::map! { "d" => 3 },
        };
        let output = build().unwrap().run(<_>::default(), input).await.unwrap();
        assert_eq!(
            output,
            value::map! {
                "updated_json" => value::map! {
                    "a" => 1,
                    "b" => value::map!{
                        "c" => value::map! {
                            "d" => 3
                        }
                    }
                }
            },
        );
    }
}
