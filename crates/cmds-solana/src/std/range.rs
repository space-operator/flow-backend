use crate::prelude::*;

// Command Name
const NAME: &str = "range";

const DEFINITION: &str = include_str!("../../../../node-definitions/std/range.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::decimal")]
    pub start: Decimal,
    #[serde(with = "value::decimal")]
    pub end: Decimal,
    #[serde(default, with = "value::decimal::opt")]
    pub step_by: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: Vec<Value>,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    const MAX_LENGTH: usize = 10_000_000;
    let mut start = input.start;
    let end = input.end;
    let step = input.step_by.unwrap_or(Decimal::ONE);
    let length: usize = ((end - start).abs() / step).floor().try_into()?;
    if length > MAX_LENGTH {
        return Err(anyhow::anyhow!(
            "too large, maximum length is {}",
            MAX_LENGTH,
        ));
    }
    let mut result = Vec::with_capacity(length);
    for _ in 0..length {
        result.push(Value::Decimal(start));
        start += step;
    }
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
}
