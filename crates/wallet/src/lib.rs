use std::path::PathBuf;

use alloy_signer_local::PrivateKeySigner;
use home::home_dir;

pub mod mnemonic;
pub mod private_key;

pub const DEFAULT_DIR: &str = ".silius";

pub enum KeySource {
    Argument(String),
    File(PathBuf),
}

pub struct Wallet {
    inner: PrivateKeySigner,
}

impl Wallet {
    pub fn new() -> Result<Self, anyhow::Error> {
        let path = home_dir().unwrap_or_default().join(DEFAULT_DIR);
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {e}"))?;
        }
        mnemonic::create_wallet_from_new_mnemonic(path)
    }

    pub fn from_key_source(key_source: &KeySource) -> Result<Self, anyhow::Error> {
        private_key::create_wallet_from_private_key(key_source)
            .or_else(|_| mnemonic::create_wallet_from_mnemonic(key_source))
            .map_err(|e| anyhow::anyhow!("Failed to create wallet from key or mnemonic: {}", e))
    }

    pub fn signer(&self) -> &PrivateKeySigner {
        &self.inner
    }
}
