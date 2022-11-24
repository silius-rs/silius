use crate::models::wallet::Wallet;

pub struct Bundler {
    pub wallet: Wallet,
}

impl Bundler {
    pub fn new(wallet: Wallet) -> Self {
        Self { wallet }
    }
}
