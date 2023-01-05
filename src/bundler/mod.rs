use clap::Parser;
use ethers::types::{Address, U256};

use crate::{
    models::wallet::Wallet,
    utils::{parse_address, parse_u256},
};

#[derive(Debug, Parser, PartialEq)]
pub struct BundlerOpts {
    #[clap(long, value_parser=parse_address)]
    pub beneficiary: Address,

    #[clap(long, default_value = "1", value_parser=parse_u256)]
    pub gas_factor: U256,

    #[clap(long, value_parser=parse_u256)]
    pub min_balance: U256,

    #[clap(long, value_parser=parse_address)]
    pub helper: Address,

    #[clap(long, default_value = "127.0.0.1:3000")]
    pub bundler_grpc_listen_address: String,
}

pub struct Bundler {
    pub wallet: Wallet,
}

impl Bundler {
    pub fn new(wallet: Wallet) -> Self {
        Self { wallet }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn bundle_opt() {
        let args = vec![
            "bundleropts",
            "--beneficiary",
            "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990",
            "--gas-factor",
            "600",
            "--min-balance",
            "1",
            "--helper",
            "0x0000000000000000000000000000000000000000",
            "--bundler-grpc-listen-address",
            "127.0.0.1:3000",
        ];
        assert_eq!(
            BundlerOpts {
                beneficiary: Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990")
                    .unwrap(),
                gas_factor: U256::from(600),
                min_balance: U256::from(1),
                helper: Address::from([0; 20]),
                bundler_grpc_listen_address: String::from("127.0.0.1:3000")
            },
            BundlerOpts::try_parse_from(args).unwrap()
        );
    }
}
