use std::{env, path::PathBuf};

use ethers::solc::{Project, ProjectPathsConfig};

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

fn compile_aa_interfaces() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("thirdparty/account-abstraction");
    let target_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_path_config = ProjectPathsConfig::builder()
        // only interfaces are needed
        .sources(root.join("contracts").join("interfaces"))
        .artifacts(target_path)
        .build_infos(root.join("contracts").join("build-info"))
        .root(root)
        .build()?;
    let project = Project::builder().paths(build_path_config).build()?;
    project.rerun_if_sources_changed();
    let compiled = project.compile()?;
    assert!(
        !compiled.has_compiler_errors(),
        "Compiling EIP-4337 interfaces failed: {:?}",
        compiled.output().errors
    );
    Ok(())
}

fn main() {
    std::env::set_var("PROTOC", protobuf_src::protoc());

    let protos = vec![
        "types/types.proto",
        "uopool/uopool.proto",
        "bundler/bundler.proto",
    ];

    make_protos(&protos);

    compile_aa_interfaces().expect("Compiling EIP-4337 interfaces should pass.");
}
