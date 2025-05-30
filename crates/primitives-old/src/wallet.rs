//! Wrapper around an ethers wallet with an optional field for Flashbots bundle identifier

use crate::{UserOperation, UserOperationSigned};
use ethers::{
    prelude::{k256::ecdsa::SigningKey, rand, LocalWallet},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::Address,
};
use expanded_pathbuf::ExpandedPathBuf;
use std::fs;

/// Wrapper around ethers wallet
#[derive(Clone, Debug)]
pub struct Wallet {
    /// Signing key of the wallet
    pub signer: ethers::signers::Wallet<SigningKey>,
    /// Flashbots signing key of the wallet
    pub flashbots_signer: Option<ethers::signers::Wallet<SigningKey>>,
}

impl Wallet {
    /// Builds a `Wallet` and construct using a randomly generated number
    /// if `flashbots_key` is true, then a Flashbots key is also created, otherwise it is set to
    /// None
    ///
    /// # Arguments
    /// * `path` - The path to the file where the mnemonic phrase will be written
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `flashbots_key` - Whether to create a Flashbots key
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn build_random(
        path: ExpandedPathBuf,
        chain_id: u64,
        flashbots_key: bool,
    ) -> eyre::Result<Self> {
        let mut rng = rand::thread_rng();

        fs::create_dir_all(&path)?;

        let wallet_builder = MnemonicBuilder::<English>::default().write_to(path.to_path_buf());

        let wallet = wallet_builder
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build_random(&mut rng)?;

        if flashbots_key {
            let mut entries = fs::read_dir(&path)?;
            let entry = entries.next().expect("No file found")?;

            let flashbots_wallet = MnemonicBuilder::<English>::default()
                .phrase(entry.path())
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
                .build()?;

            Ok(Self {
                signer: wallet.with_chain_id(chain_id),
                flashbots_signer: Some(flashbots_wallet.with_chain_id(chain_id)),
            })
        } else {
            Ok(Self { signer: wallet.with_chain_id(chain_id), flashbots_signer: None })
        }
    }

    /// Create a new wallet from the given file containing the mnemonic phrase
    /// if `flashbots_key` is true, then a Flashbots key is also created, otherwise it is set to
    /// None
    ///
    /// # Arguments
    /// * `path` - The path to the file where the mnemonic phrase is stored
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `flashbots_key` - Whether to create a Flashbots key
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn from_file(
        path: ExpandedPathBuf,
        chain_id: u64,
        flashbots_key: bool,
    ) -> eyre::Result<Self> {
        let mnemonic_phrase = fs::read_to_string(path)?;

        let wallet_builder = MnemonicBuilder::<English>::default().phrase(mnemonic_phrase.trim());

        let wallet = wallet_builder
            .clone()
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build()?;

        if flashbots_key {
            let flashbots_wallet = wallet_builder
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
                .build()?;

            Ok(Self {
                signer: wallet.with_chain_id(chain_id),
                flashbots_signer: Some(flashbots_wallet.with_chain_id(chain_id)),
            })
        } else {
            Ok(Self { signer: wallet.with_chain_id(chain_id), flashbots_signer: None })
        }
    }

    /// Create a new wallet from the given mnemonic phrase
    /// if `flashbots_key` is true, then a Flashbots key is also created, otherwise it is set to
    /// None
    ///
    /// # Arguments
    /// * `phrase` - The mnemonic phrase
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `flashbots_key` - Whether to create a Flashbots key
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn from_phrase(phrase: &str, chain_id: u64, flashbots_key: bool) -> eyre::Result<Self> {
        let wallet_builder = MnemonicBuilder::<English>::default().phrase(phrase);

        let wallet = wallet_builder
            .clone()
            .derivation_path("m/44'/60'/0'/0/0")
            .expect("Failed to derive wallet")
            .build()?;

        if flashbots_key {
            let flashbots_wallet = wallet_builder
                .derivation_path("m/44'/60'/0'/0/1")
                .expect("Failed to derive wallet")
                .build()?;
            Ok(Self {
                signer: wallet.with_chain_id(chain_id),
                flashbots_signer: Some(flashbots_wallet.with_chain_id(chain_id)),
            })
        } else {
            Ok(Self { signer: wallet.with_chain_id(chain_id), flashbots_signer: None })
        }
    }

    /// Create a new wallet from the given private key
    /// if `flashbots_key` is true, then `flashbots_private_key` must be provided
    ///
    /// # Arguments
    /// * `private_key` - The private key
    /// * `chain_id` - The chain id of the blockchain network to be used
    /// * `flashbots_key` - Whether to create a Flashbots key
    /// * `flashbots_private_key` - The private key for the Flashbots wallet
    ///
    /// # Returns
    /// * `Self` - A new `Wallet` instance
    pub fn from_private_key(
        private_key: &str,
        chain_id: u64,
        flashbots_key: bool,
        flashbots_private_key: Option<&str>,
    ) -> eyre::Result<Self> {
        let wallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        if flashbots_key {
            let flashbots_wallet = flashbots_private_key
                .expect("Flashbots private key is required")
                .parse::<LocalWallet>()?
                .with_chain_id(chain_id);
            Ok(Self { signer: wallet, flashbots_signer: Some(flashbots_wallet) })
        } else {
            Ok(Self { signer: wallet, flashbots_signer: None })
        }
    }

    /// Signs the user operation
    ///
    /// # Arguments
    /// * `uo` - The [UserOperationSigned](UserOperationSigned) to be signed
    /// * `ep` - The entry point contract address
    /// * `chain_id` - The chain id of the blockchain network to be used
    ///
    /// # Returns
    /// * `UserOperation` - The signed [UserOperation](UserOperation)
    pub async fn sign_user_operation(
        &self,
        uo: &UserOperationSigned,
        ep: &Address,
        chain_id: u64,
    ) -> eyre::Result<UserOperation> {
        let h = uo.hash(ep, chain_id);
        let sig = self.signer.sign_message(h.0.as_bytes()).await?;
        Ok(UserOperation {
            hash: h,
            user_operation: UserOperationSigned { signature: sig.to_vec().into(), ..uo.clone() },
        })
    }
}
