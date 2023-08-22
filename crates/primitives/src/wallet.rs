//! A `Wallet` is a wrapper around an ethers wallet with an optional field for Flashbots bundle identifier
use crate::UserOperation;
use ethers::{
    prelude::{k256::ecdsa::SigningKey, rand},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{Address, U256},
};
use expanded_pathbuf::ExpandedPathBuf;
use std::fs;

/// Wrapper around ethers wallet
#[derive(Clone, Debug)]
pub struct Wallet {
    /// Signing key of the wallet
    pub signer: ethers::signers::Wallet<SigningKey>,
    /// Flashbots signing key of the wallet
    pub fb_signer: Option<ethers::signers::Wallet<SigningKey>>,
}

impl Wallet {
    /// Builds a `Wallet` and construct using a randomly generated number
    /// if `build_fb_wallet` is true, then a Flashbots wallet is also built, otherwise it is set to None
    ///
    /// # Arguments
    /// * `path` - The path to the file where the mnemonic phrase will be written
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `build_fb_wallet` - Whether to build a Flashbots wallet
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn build_random(
        path: ExpandedPathBuf,
        chain_id: &U256,
        build_fb_wallet: bool,
    ) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();

        fs::create_dir_all(&path)?;

        let wallet_builder = MnemonicBuilder::<English>::default().write_to(path.to_path_buf());

        let wallet = wallet_builder
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build_random(&mut rng)?;

        if build_fb_wallet {
            let mut entries = fs::read_dir(&path)?;
            let entry = entries.next().expect("No file found")?;

            let fb_wallet = MnemonicBuilder::<English>::default()
                .phrase(entry.path())
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
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

    /// Create a new wallet from the given file containing the mnemonic phrase
    /// if `build_fb_wallet` is true, then a Flashbots wallet is also built, otherwise it is set to None
    ///
    /// # Arguments
    /// * `path` - The path to the file where the mnemonic phrase is stored
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `build_fb_wallet` - Whether to build a Flashbots wallet
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn from_file(
        path: ExpandedPathBuf,
        chain_id: &U256,
        build_fb_wallet: bool,
    ) -> anyhow::Result<Self> {
        let wallet_builder = MnemonicBuilder::<English>::default().phrase(path.to_path_buf());

        let wallet = wallet_builder
            .clone()
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build()?;

        if build_fb_wallet {
            let fb_wallet = wallet_builder
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
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
    /// if `build_fb_wallet` is true, then a Flashbots wallet is also built, otherwise it is set to None
    ///
    /// # Arguments
    /// * `phrase` - The mnemonic phrase
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `build_fb_wallet` - Whether to build a Flashbots wallet
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn from_phrase(
        phrase: &str,
        chain_id: &U256,
        build_fb_wallet: bool,
    ) -> anyhow::Result<Self> {
        let wallet_builder = MnemonicBuilder::<English>::default().phrase(phrase);

        let wallet = wallet_builder
            .clone()
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build()?;

        if build_fb_wallet {
            let fb_wallet = wallet_builder
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
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

    /// Signs the user operation
    ///
    /// # Arguments
    /// * `uo` - The [UserOperation](UserOperation) to be signed
    /// * `ep` - The entry point contract address
    /// * `chain_id` - The chain id of the blockchain network to be used
    ///
    /// # Returns
    /// * `UserOperation` - The signed [UserOperation](UserOperation)
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
