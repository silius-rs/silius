use discv5::Enr;
use eyre::Result;
use libp2p::identity::Keypair;
use silius_primitives::{constants::p2p::IPFS_GATEWAY, MempoolConfig};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, str::FromStr};

/// Load ENR from file
pub fn load_enr_from_file(path: &PathBuf) -> Option<Enr> {
    if path.exists() {
        let content = std::fs::read_to_string(path).expect("enr file currupted");
        return Some(Enr::from_str(&content).expect("enr file currupted"));
    }

    None
}

/// Save ENR to file
pub fn save_enr_to_file(enr: &Enr, path: &PathBuf) {
    std::fs::create_dir_all(path.parent().expect("Key file path error"))
        .expect("Creating key file directory failed");
    std::fs::write(path, enr.to_base64()).expect("enr writing failed");
}

/// Load combined key from file
pub fn load_private_key_from_file(path: &PathBuf) -> Option<Keypair> {
    if path.exists() {
        let content = std::fs::read(path).expect("discovery secret file currupted");
        return Some(
            Keypair::from_protobuf_encoding(&content).expect("discovery secret file currupted"),
        );
    }

    None
}

/// Save combined key to file
pub fn save_private_key_to_file(key: &Keypair, path: &PathBuf) {
    std::fs::create_dir_all(path.parent().expect("Key file path error"))
        .expect("Creating key file directory failed");
    std::fs::write(
        path.clone(),
        key.to_protobuf_encoding().expect("Discovery secret encoding failed"),
    )
    .expect("Discovery secret writing failed");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .expect("Setting key file permission failed");
}

/// Fetch mempool configuration from IPFS.
pub async fn fetch_mempool_config(cid: String) -> Result<MempoolConfig> {
    let body =
        reqwest::Client::new().get(format!("{IPFS_GATEWAY}/{cid}")).send().await?.text().await?;
    let mempool_config: MempoolConfig = serde_yml::from_str(&body)?;
    Ok(mempool_config)
}

#[cfg(test)]
pub mod tests {
    use super::fetch_mempool_config;
    use ethers::types::U256;

    #[tokio::test]
    async fn mempool_config_ethereum_sepolia() {
        let cid = "QmdDwVFoEEcgv5qnaTB8ncnXGMnqrhnA5nYpRr4ouWe4AT";
        let mempool_config = fetch_mempool_config(cid.to_string()).await.unwrap();
        assert_eq!(mempool_config.min_stake, 0.into());
    }

    #[tokio::test]
    #[ignore]
    async fn mempool_config_arbitrum_sepolia() {
        let cid = "QmVwhF77aVNzRUkMJNLDkeF9BtQMHLnfDY5ePpZ81uKLzA";
        let mempool_config = fetch_mempool_config(cid.to_string()).await.unwrap();
        assert_eq!(mempool_config.min_stake, 0.into());
    }

    #[tokio::test]
    async fn mempool_config_polygon_amoy() {
        let cid = "QmQfRyE9iVTBqZ17hPSP4tuMzaez83Y5wD874ymyRtj9VE";
        let mempool_config = fetch_mempool_config(cid.to_string()).await.unwrap();
        assert_eq!(mempool_config.min_stake, 0.into());
    }

    #[tokio::test]
    async fn mempool_config_polygon() {
        let cid = "QmRJ1EPhmRDb8SKrPLRXcUBi2weUN8VJ8X9zUtXByC7eJg";
        let mempool_config = fetch_mempool_config(cid.to_string()).await.unwrap();
        assert_eq!(mempool_config.min_stake, U256::from(500000000000000000000_u128));

        let cid = "QmaHG3xiRYhxTth7vSTyZCyodBDrtj5hmEMz5DuzaJVKHH";
        let mempool_config = fetch_mempool_config(cid.to_string()).await.unwrap();
        assert_eq!(mempool_config.min_stake, U256::from(1000000000000000000000_u128));
    }
}
