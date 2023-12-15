use ethers_solc::{Project, ProjectPathsConfig};
use std::{env, path::PathBuf};

fn compile(
    root: &PathBuf,
    source: &PathBuf,
    build_info: &PathBuf,
    target: &PathBuf,
) -> eyre::Result<()> {
    let build_path_config = ProjectPathsConfig::builder()
        .sources(source)
        .artifacts(target)
        .build_infos(build_info)
        .root(root)
        .build()?;

    let project = Project::builder().paths(build_path_config).build()?;
    project.rerun_if_sources_changed();
    let compiled = project.compile()?;

    assert!(
        !compiled.has_compiler_errors(),
        "Compiling ERC-4337 smart contracts failed: {:?}.",
        compiled.output().errors
    );

    Ok(())
}

fn compile_aa_smart_contracts() -> eyre::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("thirdparty/account-abstraction");
    let target = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let build_info = root.join("contracts").join("build-info");

    // compile interfaces
    compile(&root, &root.join("contracts").join("interfaces"), &build_info, &target)?;

    // compile sender creator smart contract
    compile(
        &root,
        &root.join("contracts").join("core").join("SenderCreator.sol"),
        &build_info,
        &target,
    )?;

    Ok(())
}

fn main() {
    compile_aa_smart_contracts().expect("Compiling ERC-4337 smart contracts should pass.");
}
