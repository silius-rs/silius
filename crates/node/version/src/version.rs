pub const APP_NAME: &str = "silius";

/// The latest git commit hash of the build.
pub const SILIUS_FULL_COMMIT: &str = env!("VERGEN_GIT_SHA");
pub const SILIUS_SHORT_COMMIT: &str = env!("VERGEN_GIT_SHA_SHORT");

/// Silius's version is the same as the git tag.
pub const SILIUS_VERSION: &str = env!("SILIUS_VERSION");

/// The operating system of the build, linux, macos, windows etc.
pub const BUILD_OPERATING_SYSTEM: &str = env!("SILIUS_BUILD_OPERATING_SYSTEM");

/// The architecture of the build, x86_64, aarch64, etc.
pub const BUILD_ARCHITECTURE: &str = env!("SILIUS_BUILD_ARCHITECTURE");

/// The version of the programming language used to build the binary.
pub const PROGRAMMING_LANGUAGE_VERSION: &str = env!("VERGEN_RUSTC_SEMVER");

pub const FULL_VERSION: &str = env!("SILIUS_FULL_VERSION");

/// Returns the silius version and git revision.
pub const fn get_silius_version_short_commit() -> &'static str {
    SILIUS_SHORT_COMMIT
}

/// Information about the client.
/// example: silius/v0.0.1-892ad575/linux-x86_64/rustc1.85.0
pub fn silius_node_version() -> String {
    format!(
        "{APP_NAME}/{SILIUS_VERSION}-{SILIUS_SHORT_COMMIT}/{BUILD_OPERATING_SYSTEM}-{BUILD_ARCHITECTURE}/rustc{PROGRAMMING_LANGUAGE_VERSION}"
    )
}
