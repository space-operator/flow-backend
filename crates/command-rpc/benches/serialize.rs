use criterion::{Criterion, black_box, criterion_group, criterion_main};
use flow_lib::{
    Value, ValueType,
    config::client::{Extra, NodeData, Source, Target, TargetsForm},
    value,
};

fn value_serialize(nd: &NodeData) -> Value {
    value::to_value(nd).unwrap()
}

fn bincode_serialize(nd: &NodeData) -> Vec<u8> {
    value::to_value(nd).unwrap().to_bincode().unwrap()
}

fn json_serialize(nd: &NodeData) -> String {
    serde_json::to_string(nd).unwrap()
}

fn simd_json_serialize(nd: &NodeData) -> String {
    simd_json::to_string(nd).unwrap()
}

fn bincode_serialize_value(value: &Value) -> Vec<u8> {
    value.to_bincode().unwrap()
}

fn json_serialize_value(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

fn bench_ser_node_data(c: &mut Criterion) {
    let nd = NodeData {
        r#type: flow_lib::CommandType::Native,
        node_id: "add".to_owned(),
        sources: [Source {
            id: <_>::default(),
            name: "c".to_owned(),
            r#type: ValueType::Decimal,
            optional: false,
        }]
        .into(),
        targets: [
            Target {
                id: <_>::default(),
                name: "a".to_owned(),
                type_bounds: [ValueType::Decimal].into(),
                required: true,
                passthrough: false,
            },
            Target {
                id: <_>::default(),
                name: "b".to_owned(),
                type_bounds: [ValueType::Decimal].into(),
                required: true,
                passthrough: false,
            },
        ]
        .into(),
        targets_form: TargetsForm {
            form_data: serde_json::Value::Null,
            extra: Extra {
                ..Default::default()
            },
            wasm_bytes: None,
        },
        instruction_info: None,
    };

    let mut g = c.benchmark_group("ser_NodeData");
    g.bench_function("bincode", |b| b.iter(|| bincode_serialize(black_box(&nd))));
    g.bench_function("json", |b| b.iter(|| json_serialize(black_box(&nd))));
    g.bench_function("simd_json", |b| {
        b.iter(|| simd_json_serialize(black_box(&nd)))
    });
    g.bench_function("value", |b| b.iter(|| value_serialize(black_box(&nd))));
    g.finish();

    let value: Value = serde_json::from_str(
r#"
{
  "M": {
    "uri": {
      "S": "https://base.spaceoperator.com/storage/v1/object/public/blinks/ef00d6c7-7f76-4554-bd93-1b197849584f/1c9a0c7b-4264-44ff-bbb9-245bf03360cb.json"
    },
    "name": {
      "S": "Test 1"
    },
    "payer": {
      "B3": "2gdutJtCz1f9P3NJGP4HbBYFCHMh8rVAhmT2QDSb9dN9"
    },
    "collection": {
      "S": "9HFpVi6zuCDKjbGfQopWZxWpCnHpjLusrNsCheg44fsz"
    },
    "mint_amount": {
      "S": "0.001"
    },
    "basis_points": {
      "D": "500"
    },
    "payee_address": {
      "S": "2gdutJtCz1f9P3NJGP4HbBYFCHMh8rVAhmT2QDSb9dN9"
    },
    "collection_creator": {
      "S": "tFC6Zmb9rss6xKEptqpSrjiP69pBiqGshzbPE8ZX9BG"
    }
  }
}
"#
    ).unwrap();

    let mut g = c.benchmark_group("ser_Value");
    g.bench_function("bincode", |b| {
        b.iter(|| bincode_serialize_value(black_box(&value)))
    });
    g.bench_function("json", |b| {
        b.iter(|| json_serialize_value(black_box(&value)))
    });
    g.finish();
}

criterion_group!(benches, bench_ser_node_data);
criterion_main!(benches);
