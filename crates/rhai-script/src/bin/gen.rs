use rhai_script::COMMAND_ID_PREFIX;
use serde::Serialize;
use serde_json::{json, ser::PrettyFormatter};

fn main() {
    for i in 0..=5 {
        for o in 1..=5 {
            let name = format!("{COMMAND_ID_PREFIX}{i}x{o}");

            let mut inputs = vec![json!({
                "name": "source",
                "type_bounds": ["string"],
                "required": true,
                "passthrough": false,
                "tooltip": "Rhai script source code"
            })];
            inputs.extend((0..i).map(|idx| {
                json!({
                    "name": String::from(b"abcde"[idx] as char),
                    "type_bounds": ["free"],
                    "required": false,
                    "passthrough": false,
                    "tooltip": format!("Script input {}", idx + 1)
                })
            }));

            let outputs: Vec<_> = (0..o)
                .map(|idx| {
                    json!({
                        "name": String::from(b"uvxyz"[idx] as char),
                        "type": "free",
                        "optional": true,
                        "tooltip": format!("Script output {}", idx + 1)
                    })
                })
                .collect();

            let def = json!({
                "$schema": "https://schema.spaceoperator.com/node-v2.schema.json",
                "version": "0.1",
                "name": name,
                "description": "",
                "type": "native",
                "author_handle": "spo",
                "source_code": "crates/rhai-script/src/lib.rs",
                "ports": {
                    "inputs": inputs,
                    "outputs": outputs,
                },
                "config_schema": {
                    "type": "object",
                    "title": "RHAI script",
                    "properties": {
                        "source": {
                            "title": "source",
                            "type": "string"
                        }
                    }
                },
                "config": {}
            });

            let mut pretty = Vec::<u8>::new();
            def.serialize(&mut serde_json::ser::Serializer::with_formatter(
                &mut pretty,
                PrettyFormatter::with_indent(b"  "),
            ))
            .unwrap();

            let base_path: String =
                concat!(env!("CARGO_MANIFEST_DIR"), "/node-definitions/").to_owned();
            std::fs::write(
                base_path + format!("{name}.jsonc").as_str(),
                &pretty,
            )
            .unwrap();
        }
    }
}
