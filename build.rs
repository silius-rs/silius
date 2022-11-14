use std::{env, path::PathBuf};

fn config() -> prost_build::Config {
    let mut config = prost_build::Config::new();
    config.bytes(&["."]);
    config
}

fn make_protos(protos: &[&str]) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile_with_config(config(), protos, &["./src/proto"])
        .unwrap();
}

fn main() {
    std::env::set_var("PROTOC", protobuf_src::protoc());

    let protos = vec![
        "types/types.proto",
        "uopool/uopool.proto",
        "bundler/bundler.proto",
    ];

    make_protos(&protos);
}
