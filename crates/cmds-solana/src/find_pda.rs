use crate::prelude::*;

const FIND_PDA: &str = "find_pda";
const DEFINITION: &str = flow_lib::node_definition!("find_pda.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(FIND_PDA));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(FIND_PDA, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub program_id: Pubkey,
    #[serde(default)]
    pub seeds: Vec<Value>,
    pub seed_1: Option<Value>,
    pub seed_2: Option<Value>,
    pub seed_3: Option<Value>,
    pub seed_4: Option<Value>,
    pub seed_5: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::pubkey")]
    pub pda: Pubkey,
}

fn seed_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::B32(v) => v.to_vec(),
        Value::String(v) => v.as_bytes().to_vec(),
        _ => vec![],
    }
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let seed_values: Vec<Value> = if !input.seeds.is_empty() {
        input.seeds
    } else {
        [
            input.seed_1,
            input.seed_2,
            input.seed_3,
            input.seed_4,
            input.seed_5,
        ]
        .into_iter()
        .flatten()
        .collect()
    };

    let seeds: Vec<Vec<u8>> = seed_values
        .iter()
        .map(seed_bytes)
        .filter(|s| !s.is_empty())
        .collect();

    let seeds: Vec<&[u8]> = seeds.iter().map(|s| &s[..]).collect();
    let pda = Pubkey::find_program_address(&seeds, &input.program_id).0;

    Ok(Output { pda })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_string_seeds() {
        let program_id = Pubkey::new_unique();
        let expected = Pubkey::find_program_address(&[b"hello", b"world"], &program_id).0;

        let output = build()
            .unwrap()
            .run(
                CommandContext::default(),
                value::map! {
                    "program_id" => program_id,
                    "seed_1" => Value::String("hello".to_owned()),
                    "seed_2" => Value::String("world".to_owned()),
                },
            )
            .await
            .unwrap();

        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(output.pda, expected);
    }

    #[tokio::test]
    async fn test_pubkey_seed() {
        let program_id = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let expected =
            Pubkey::find_program_address(&[b"seed", mint.as_ref()], &program_id).0;

        let output = build()
            .unwrap()
            .run(
                CommandContext::default(),
                value::map! {
                    "program_id" => program_id,
                    "seed_1" => Value::String("seed".to_owned()),
                    "seed_2" => mint,
                },
            )
            .await
            .unwrap();

        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(output.pda, expected);
    }

    #[tokio::test]
    async fn test_no_seeds() {
        let program_id = Pubkey::new_unique();
        let expected = Pubkey::find_program_address(&[], &program_id).0;

        let output = build()
            .unwrap()
            .run(
                CommandContext::default(),
                value::map! {
                    "program_id" => program_id,
                },
            )
            .await
            .unwrap();

        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(output.pda, expected);
    }

    #[tokio::test]
    async fn test_seeds_array() {
        let program_id = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let expected =
            Pubkey::find_program_address(&[b"token", mint.as_ref()], &program_id).0;

        let output = build()
            .unwrap()
            .run(
                CommandContext::default(),
                value::map! {
                    "program_id" => program_id,
                    "seeds" => Value::Array(vec![
                        Value::String("token".to_owned()),
                        Value::B32(mint.to_bytes()),
                    ]),
                },
            )
            .await
            .unwrap();

        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(output.pda, expected);
    }
}
