use ethers::{
    prelude::{k256::ecdsa::SigningKey, rand},
    signers::{coins_bip39::English, MnemonicBuilder},
};
use expanded_pathbuf::ExpandedPathBuf;

pub struct Wallet {
    pub signer: ethers::signers::Wallet<SigningKey>,
}

impl Wallet {
    pub fn new(output_path: ExpandedPathBuf) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            signer: MnemonicBuilder::<English>::default()
                .write_to(output_path.to_path_buf())
                .build_random(&mut rng)
                .unwrap(),
        }
    }

    pub fn from_file(input_path: ExpandedPathBuf) -> Self {
        Self {
            signer: MnemonicBuilder::<English>::default()
                .phrase(input_path.to_path_buf())
                .build()
                .unwrap(),
        }
    }
}
