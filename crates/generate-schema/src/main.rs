fn main() {
    use utoipa::OpenApi;

    let mut schema = schemars::schema_for!(flow_server::Config);
    schema.as_object_mut().unwrap()["$schema"] = "http://json-schema.org/draft-07/schema#".into();
    std::fs::write(
        "./schema/flow-server-config.schema.json",
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();

    std::fs::write(
        "./schema/flow-server.openapi.json",
        flow_server::openapi::FlowServerOpenApiDoc::openapi()
            .to_pretty_json()
            .unwrap(),
    )
    .unwrap();
}
