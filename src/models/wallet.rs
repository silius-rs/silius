use ethers::{
    prelude::{k256::ecdsa::SigningKey, rand},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::U256,
};
use expanded_pathbuf::ExpandedPathBuf;
use std::fs;

pub struct Wallet {
    pub signer: ethers::signers::Wallet<SigningKey>,
}

impl Wallet {
    pub fn new(output_path: ExpandedPathBuf, chain_id: U256) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();

        fs::create_dir_all(&output_path)?;

        let wallet = MnemonicBuilder::<English>::default()
            .write_to(output_path.to_path_buf())
            .build_random(&mut rng)?;

        Ok(Self {
            signer: wallet.with_chain_id(chain_id.as_u64()),
        })
    }

    pub fn from_file(input_path: ExpandedPathBuf, chain_id: U256) -> anyhow::Result<Self> {
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(input_path.to_path_buf())
            .build()?;

        Ok(Self {
            signer: wallet.with_chain_id(chain_id.as_u64()),
        })
    }
}
