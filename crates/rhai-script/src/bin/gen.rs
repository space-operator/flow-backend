use serde::Serialize;
use serde_json::{json, ser::PrettyFormatter};

const BASE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../node-definition.json"
));

fn main() {
    let mut base: serde_json::Value = serde_json::from_str(BASE).unwrap();
    base["targets"] = vec![json!(
        {
            "name": "source",
            "type_bounds": [
                "string"
            ],
            "required": true,
            "passthrough": false,
            "defaultValue": null,
            "tooltip": "Script's source code"
        }
    )]
    .into();
    base["sources"] = serde_json::Value::Array(Vec::new());
    base["targets_form.json_schema"] = json!({
      "type": "object",
      "title": "RHAI script",
      "properties": {
        "source": {
          "title": "source",
          "type": "string"
        }
      }
    });
    base["targets_form.ui_schema"] = json!({
      "source": {
        "ui:widget": "textarea"
      },
      "ui:order": [
        "source"
      ]
    });
    for i in 0..=5 {
        for o in 1..=5 {
            let mut def = base.clone();
            def["data"]["node_id"] = format!("rhai_script_{i}x{o}").into();
            def["data"]["display_name"] = format!("RHAI Script {i}x{o}").into();
            let targets = (0..i).map(|i| {
                json!({
                    "name": b"abcde"[i] as char,
                    "type_bounds": [
                        "free"
                    ],
                    "required": false,
                    "passthrough": false,
                    "defaultValue": null,
                    "tooltip": ""
                })
            });
            def["targets"].as_array_mut().unwrap().extend(targets);
            let sources = (0..o).map(|o| {
                json!({
                    "name": b"uvxyz"[o] as char,
                    "type": "free",
                    "optional": true,
                    "defaultValue": "",
                    "tooltip": ""
                })
            });
            def["sources"].as_array_mut().unwrap().extend(sources);
            let mut pretty = Vec::<u8>::new();
            def.serialize(&mut serde_json::ser::Serializer::with_formatter(
                &mut pretty,
                PrettyFormatter::with_indent(b"  "),
            ))
            .unwrap();
            std::fs::write(
                &(concat!(env!("CARGO_MANIFEST_DIR"), "/node-definitions/",).to_owned()
                    + &format!("rhai_script_{i}x{o}.json")),
                &pretty,
            )
            .unwrap();
        }
    }
}
