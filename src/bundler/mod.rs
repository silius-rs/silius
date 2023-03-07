use std::time::Duration;

use clap::Parser;
use ethers::types::{Address, U256};
use tokio::time;

use crate::{
    models::wallet::Wallet,
    types::user_operation::UserOperation,
    uopool::server::uopool::{uo_pool_client::UoPoolClient, GetSortedRequest},
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

    #[clap(long, default_value = "127.0.0.1:3002")]
    pub bundler_grpc_listen_address: String,

    #[clap(long, default_value = "10")]
    pub bundle_interval: u64,

    #[clap(long, default_value = "10")]
    pub max_bundle_limit: u64,

    #[clap(long, value_parser=parse_address)]
    pub entrypoint: Address,
}

pub struct Bundler {
    pub wallet: Wallet,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    pub bundle_interval: u64,
    pub max_bundle_limit: u64,
    pub entrypoint: Address,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
        bundle_interval: u64,
        max_bundle_limit: u64,
        entrypoint: Address,
    ) -> Self {
        Self {
            wallet,
            uopool_grpc_client,
            bundle_interval,
            max_bundle_limit,
            entrypoint,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut interval = time::interval(Duration::from_secs(self.bundle_interval));

        loop {
            interval.tick().await;
            self.send_next_bundle().await?;
        }
    }

    async fn create_bundle(&mut self) -> anyhow::Result<Vec<UserOperation>> {
        let request = tonic::Request::new(GetSortedRequest {
            entrypoint: Some(self.entrypoint.into()),
            max_limit: self.max_bundle_limit,
        });
        let response = self
            .uopool_grpc_client
            .get_sorted_user_operations(request)
            .await?;
        let user_operations: Vec<UserOperation> = response
            .into_inner()
            .user_operations
            .into_iter()
            .map(|u| u.into())
            .collect();
        Ok(user_operations)
    }

    async fn send_next_bundle(&mut self) -> anyhow::Result<()> {
        let _bundles = self.create_bundle().await?;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn bundler_opts() {
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
            "127.0.0.1:3002",
            "--bundle-interval",
            "10",
            "--max-bundle-limit",
            "10",
            "--entrypoint",
            "0x0000000000000000000000000000000000000000",
        ];
        assert_eq!(
            BundlerOpts {
                beneficiary: Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990")
                    .unwrap(),
                gas_factor: U256::from(600),
                min_balance: U256::from(1),
                helper: Address::from([0; 20]),
                bundler_grpc_listen_address: String::from("127.0.0.1:3002"),
                bundle_interval: 10,
                max_bundle_limit: 10,
                entrypoint: Address::from([0; 20])
            },
            BundlerOpts::try_parse_from(args).unwrap()
        );
    }
}
