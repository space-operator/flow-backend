fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/command.capnp")
        .run().expect("schema compiler command");
}
