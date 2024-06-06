use std::{env, path::PathBuf};

fn config() -> prost_build::Config {
    let mut config = prost_build::Config::new();
    config.bytes(["."]);
    config
}

fn make_protos(protos: &[&str]) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIT not set"));
    tonic_build::configure()
        .server_mod_attribute(
            "uopool",
            r#"#[allow(clippy::unwrap_used, clippy::mixed_attributes_style)]"#,
        )
        .server_mod_attribute(
            "bundler",
            r#"#[allow(clippy::unwrap_used, clippy::mixed_attributes_style)]"#,
        )
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile_with_config(config(), protos, &["./src/protos"])
        .expect("Failed to compile protos.");
}

fn main() {
    std::env::set_var("PROTOC", protobuf_src::protoc());

    let protos = vec![
        "src/protos/types/types.proto",
        "src/protos/uopool/uopool.proto",
        "src/protos/bundler/bundler.proto",
    ];

    make_protos(&protos);
}
