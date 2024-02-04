use discv5::Enr;
use libp2p::identity::Keypair;
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
