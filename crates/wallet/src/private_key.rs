use crate::{KeySource, Wallet};

pub fn create_wallet_from_private_key(key_source: &KeySource) -> Result<Wallet, anyhow::Error> {
    let private_key = match key_source {
        KeySource::Argument(private_key) => private_key.clone(),
        KeySource::File(path) => std::fs::read_to_string(path)?,
    };

    let signer = private_key.trim().parse()?;

    Ok(Wallet { inner: signer })
}
