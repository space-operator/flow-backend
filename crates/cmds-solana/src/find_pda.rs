use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct FindPDA;

const FIND_PDA: &str = "find_pda";
#[derive(Serialize, Deserialize, Debug)]

pub struct Input {
    #[serde(with = "value::pubkey")]
    pub program_id: Pubkey,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::pubkey")]
    pub pda: Pubkey,
}

const PROGRAM_ID: &str = "program_id";
const SEED_1: &str = "seed_1";
const SEED_2: &str = "seed_2";
const SEED_3: &str = "seed_3";
const SEED_4: &str = "seed_4";
const SEED_5: &str = "seed_5";

const PDA: &str = "pda";

#[async_trait]
impl CommandTrait for FindPDA {
    fn name(&self) -> Name {
        FIND_PDA.into()
    }

    fn inputs(&self) -> Vec<CmdInput> {
        [
            CmdInput {
                name: PROGRAM_ID.into(),
                type_bounds: [ValueType::Pubkey].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: SEED_1.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: SEED_2.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: SEED_3.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: SEED_4.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: false,
                passthrough: false,
            },
            CmdInput {
                name: SEED_5.into(),
                type_bounds: [ValueType::Free].to_vec(),
                required: false,
                passthrough: false,
            },
        ]
        .to_vec()
    }

    fn outputs(&self) -> Vec<CmdOutput> {
        [CmdOutput {
            name: PDA.into(),
            r#type: ValueType::Pubkey,
        }]
        .to_vec()
    }

    async fn run(&self, _: Context, mut inputs: ValueSet) -> Result<ValueSet, CommandError> {
        let Input { program_id } = value::from_map(inputs.clone())?;

        let seed_1: Option<Value> = inputs.remove(SEED_1);
        let seed_1 = match seed_1 {
            Some(Value::B32(v)) => v.to_vec(),
            Some(Value::String(v)) => v.as_bytes().to_vec(),
            _ => vec![],
        };

        let seed_2: Option<Value> = inputs.remove(SEED_2);
        let seed_2 = match seed_2 {
            Some(Value::B32(v)) => v.to_vec(),
            Some(Value::String(v)) => v.as_bytes().to_vec(),
            _ => vec![],
        };
        let seed_3: Option<Value> = inputs.remove(SEED_3);
        let seed_3 = match seed_3 {
            Some(Value::B32(v)) => v.to_vec(),
            Some(Value::String(v)) => v.as_bytes().to_vec(),
            _ => vec![],
        };
        let seed_4: Option<Value> = inputs.remove(SEED_4);
        let seed_4 = match seed_4 {
            Some(Value::B32(v)) => v.to_vec(),
            Some(Value::String(v)) => v.as_bytes().to_vec(),
            _ => vec![],
        };
        let seed_5: Option<Value> = inputs.remove(SEED_5);
        let seed_5 = match seed_5 {
            Some(Value::B32(v)) => v.to_vec(),
            Some(Value::String(v)) => v.as_bytes().to_vec(),
            _ => vec![],
        };

        let seeds = vec![seed_1, seed_2, seed_3, seed_4, seed_5];

        let seeds = seeds
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<Vec<u8>>>();

        let seeds = seeds.iter().map(|s| &s[..]).collect::<Vec<&[u8]>>();

        let seeds = &seeds[..];

        let pda = Pubkey::find_program_address(seeds, &program_id).0;

        Ok(value::to_map(&Output { pda })?)
    }
}

inventory::submit!(CommandDescription::new(FIND_PDA, |_| Ok(Box::new(FindPDA))));
