use crate::*;
use pretty_assertions::assert_eq;
use serde_json::{json, Value as Json};

pub fn module(name: &str) -> Result<Vec<u8>> {
    let base = env!("CARGO_MANIFEST_DIR");
    let path = format!("{base}/tests/{name}/target/wasm32-wasi/release/{name}.wasm");
    Ok(std::fs::read(path)?)
}

#[test]
fn manual() -> Result<()> {
    let wasm = Wasm::new(&module("manual")?, <_>::default())?;
    let input = json! {{
        "value": 100,
        "name": "Space Operator",
    }};
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {{
            "value": 200,
            "name": "rotarepO ecapS",
        }}
    );
    Ok(())
}

#[test]
fn automatic() -> Result<()> {
    let wasm = Wasm::new(&module("automatic")?, <_>::default())?;
    let input = json! {{
        "value": 100,
        "name": "Space Operator",
    }};
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {{
            "value": 200,
            "name": "rotarepO ecapS",
        }}
    );
    Ok(())
}

#[test]
fn simple() -> Result<()> {
    let wasm = Wasm::new(&module("simple")?, <_>::default())?;
    let input = json! {
        "This is my string".repeat(10000)
    };
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {
            "gnirts ym si sihT".repeat(10000)
        }
    );
    Ok(())
}

#[test]
fn env() -> Result<()> {
    let wasm = Wasm::new(
        &module("env")?,
        [("RUST_LOG".to_owned(), "info".to_owned())].into(),
    )?;
    let input = json!("RUST_LOG");
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(output, json!("info"));
    Ok(())
}

#[test]
fn number() -> Result<()> {
    let wasm = Wasm::new(&module("number")?, <_>::default())?;
    let input = json! {
        100
    };
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {
            200
        }
    );
    Ok(())
}

#[test]
fn float() -> Result<()> {
    let wasm = Wasm::new(&module("float")?, <_>::default())?;
    let input = json! {
        5.4321
    };
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {
            17.065445453565115
        }
    );
    Ok(())
}

#[test]
fn http() -> Result<()> {
    let wasm = Wasm::new(&module("http")?, <_>::default())?;
    let input = json! {{
        "url": "https://dummyjson.com/products/1",
    }};
    let output = wasm.run::<_, Json>("main", &input)?;
    assert_eq!(
        output,
        json! {{
            "Ok": {
                "id": 1,
                "title": "iPhone 9",
                "description": "An apple mobile which is nothing like apple",
            },
        }}
    );
    Ok(())
}
