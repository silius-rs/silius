use std::path::PathBuf;

use alloy_signer_local::{MnemonicBuilder, coins_bip39::English};
use tracing::info;

use crate::{KeySource, Wallet};

pub const PRIVATE_KEY_INDEX: u32 = 0;

pub fn create_wallet_from_new_mnemonic(path: PathBuf) -> Result<Wallet, anyhow::Error> {
    info!(
        "No wallet provided... Creating new mnemonic and saving to: {:?}",
        path
    );

    let signer = MnemonicBuilder::<English>::default()
        .word_count(24)
        .derivation_path("m/44'/60'/0'/2/1")?
        .write_to(path)
        .index(PRIVATE_KEY_INDEX)?
        .build_random()?;

    Ok(Wallet { inner: signer })
}

pub fn create_wallet_from_mnemonic(key_source: &KeySource) -> Result<Wallet, anyhow::Error> {
    let mnemonic = match key_source {
        KeySource::Argument(mnemonic) => mnemonic.clone(),
        KeySource::File(path) => std::fs::read_to_string(path)?,
    };

    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic.trim())
        .index(PRIVATE_KEY_INDEX)?
        .build()?;

    Ok(Wallet { inner: signer })
}
