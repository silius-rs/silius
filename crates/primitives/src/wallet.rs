use crate::UserOperation;
use ethers::{
    prelude::{k256::ecdsa::SigningKey, rand, LocalWallet},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use std::fs;

/// Wrapper around ethers wallet
#[derive(Clone)]
pub struct Wallet {
    /// Signing key of the wallet
    pub signer: ethers::signers::Wallet<SigningKey>,
    pub fb_signer: Option<ethers::signers::Wallet<SigningKey>>,
}

impl Wallet {
    /// Create a new wallet and outputs the mnemonic to the given path
    pub fn build_random(
        path: ExpandedPathBuf,
        chain_id: &U256,
        fb_path: Option<ExpandedPathBuf>,
    ) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();

        fs::create_dir_all(&path)?;

        let wallet = MnemonicBuilder::<English>::default()
            .write_to(path.to_path_buf())
            .build_random(&mut rng)?;

        if fb_path.is_some() {
            let mut rng = rand::thread_rng();
            fs::create_dir_all(
                fb_path
                    .clone()
                    .expect("Failed to open Flashbots' mnemonic file"),
            )?;
            let fb_wallet = MnemonicBuilder::<English>::default()
                .write_to(
                    fb_path
                        .expect("Failed to open Flashbots' mnemonic file")
                        .to_path_buf(),
                )
                .build_random(&mut rng)?;

            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: Some(fb_wallet.with_chain_id(chain_id.as_u64())),
            })
        } else {
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: None,
            })
        }
    }

    /// Create a new wallet from the given file containing the mnemonic phrase
    pub fn from_file(
        path: ExpandedPathBuf,
        chain_id: &U256,
        fb_path: Option<ExpandedPathBuf>,
    ) -> anyhow::Result<Self> {
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(path.to_path_buf())
            .build()?;

        if fb_path.is_some() {
            let fb_wallet = MnemonicBuilder::<English>::default()
                .phrase(
                    fb_path
                        .expect("Failed to open Flashbots' mnemonic file")
                        .to_path_buf(),
                )
                .build()?;
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: Some(fb_wallet.with_chain_id(chain_id.as_u64())),
            })
        } else {
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: None,
            })
        }
    }

    /// Create a new wallet from the given mnemonic phrase
    pub fn from_phrase(
        phrase: &str,
        chain_id: &U256,
        fb_phrase: Option<&str>,
    ) -> anyhow::Result<Self> {
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(phrase)
            .build()?;

        if fb_phrase.is_some() {
            let fb_wallet = MnemonicBuilder::<English>::default()
                .phrase(fb_phrase.expect("Failed to read Flashbots' mnemonic file"))
                .build()?;
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: Some(fb_wallet.with_chain_id(chain_id.as_u64())),
            })
        } else {
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: None,
            })
        }
    }

    /// Create a new wallet from the given private key
    pub fn from_key(key: &str, chain_id: &U256, fb_key: Option<&str>) -> anyhow::Result<Self> {
        let wallet = key.parse::<LocalWallet>()?;

        if fb_key.is_some() {
            let fb_wallet = fb_key
                .expect("Failed to read Flashbot's mnemonic file")
                .parse::<LocalWallet>()?;
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: Some(fb_wallet.with_chain_id(chain_id.as_u64())),
            })
        } else {
            Ok(Self {
                signer: wallet.with_chain_id(chain_id.as_u64()),
                fb_signer: None,
            })
        }
    }

    /// Signs the user operation
    pub async fn sign_uo(
        &self,
        uo: &UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> anyhow::Result<UserOperation> {
        let h = uo.hash(ep, chain_id);
        let sig = self.signer.sign_message(h.0.as_bytes()).await?;
        Ok(UserOperation {
            signature: sig.to_vec().into(),
            ..uo.clone()
        })
    }
}
