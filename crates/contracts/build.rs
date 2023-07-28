use ethers_solc::{Project, ProjectPathsConfig};
use std::{env, path::PathBuf};

fn compile_aa_interfaces() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("thirdparty/account-abstraction");
    let target_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
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
        "Compiling ERC-4337 interfaces failed: {:?}.",
        compiled.output().errors
    );
    Ok(())
}

fn main() {
    compile_aa_interfaces().expect("Compiling ERC-4337 interfaces should pass.");
}
