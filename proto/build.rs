fn main() {
    capnpc::CompilerCommand::new()
        .file("proto/to_edge.capnp")
        .file("proto/from_edge.capnp")
        .run()
        .expect("compiling schema");
}
