pub mod server;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use clap::Parser;
use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, U256},
};
use parking_lot::Mutex;
use serde::Deserialize;
use tracing::{debug, error};

use crate::{
    contracts::gen::EntryPointAPI,
    models::wallet::Wallet,
    types::user_operation::UserOperation,
    uopool::server::{
        bundler::Mode as GrpcMode,
        uopool::{uo_pool_client::UoPoolClient, GetSortedRequest},
    },
    utils::{parse_address, parse_u256},
};

#[derive(Debug, Deserialize)]
pub enum Mode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "manual")]
    Manual,
}

impl From<Mode> for GrpcMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Auto => Self::Auto,
            Mode::Manual => Self::Manual,
        }
    }
}

#[derive(Debug, Parser, PartialEq)]
pub struct BundlerOpts {
    #[clap(long, value_parser=parse_address)]
    pub beneficiary: Address,

    #[clap(long, default_value = "1", value_parser=parse_u256)]
    pub gas_factor: U256,

    #[clap(long, value_parser=parse_u256)]
    pub min_balance: U256,

    #[clap(long, default_value = "127.0.0.1:3002")]
    pub bundler_grpc_listen_address: SocketAddr,

    #[clap(long, default_value = "10")]
    pub bundle_interval: u64,
}

#[derive(Clone)]
pub struct Bundler {
    pub wallet: Wallet,
    pub beneficiary: Address,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    pub entry_point: Address,
    pub eth_client_address: String,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
        entry_point: Address,
        eth_client_address: String,
    ) -> Self {
        Self {
            wallet,
            beneficiary,
            uopool_grpc_client,
            entry_point,
            eth_client_address,
        }
    }

    async fn create_bundle(&self) -> anyhow::Result<Vec<UserOperation>> {
        let request = tonic::Request::new(GetSortedRequest {
            entry_point: Some(self.entry_point.into()),
        });
        let response = self
            .uopool_grpc_client
            .clone()
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

    async fn send_next_bundle(&self) -> anyhow::Result<()> {
        let bundles = self.create_bundle().await?;
        let provider = Provider::<Http>::try_from(self.eth_client_address.clone())?;
        let client = Arc::new(SignerMiddleware::new(provider, self.wallet.signer.clone()));
        let entry_point = EntryPointAPI::new(self.entry_point, client.clone());
        let nonce = client
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await?;
        let (max_fee_per_gas, max_priority_fee_per_gas) =
            client.clone().estimate_eip1559_fees(None).await?;
        let mut tx: TypedTransaction = entry_point
            .handle_ops(
                bundles.into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx
            .clone();
        tx.set_gas(U256::from(1000000)).set_nonce(nonce);
        match tx {
            TypedTransaction::Eip1559(ref mut inner) => {
                inner.max_fee_per_gas = Some(max_fee_per_gas);
                inner.max_priority_fee_per_gas = Some(max_priority_fee_per_gas)
            }
            _ => {
                tx.set_gas_price(max_fee_per_gas);
            }
        };
        let res = client.send_transaction(tx, None).await?.await?;

        debug!("Send bundles with ret: {res:?}");
        Ok(())
    }
}

pub struct BundlerManager {
    pub bundlers: Vec<Bundler>,
    pub bundle_interval: u64,
    pub running: Arc<Mutex<bool>>,
}

fn is_running(running: Arc<Mutex<bool>>) -> bool {
    let r = running.lock();
    *r
}

impl BundlerManager {
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
        entry_points: Vec<Address>,
        eth_client_address: String,
        bundle_interval: u64,
    ) -> Self {
        let bundlers: Vec<Bundler> = entry_points
            .iter()
            .map(|entry_point| {
                Bundler::new(
                    wallet.clone(),
                    beneficiary,
                    uopool_grpc_client.clone(),
                    *entry_point,
                    eth_client_address.clone(),
                )
            })
            .collect();

        Self {
            bundlers,
            bundle_interval,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start_server(&self) {}

    pub fn stop_bundling(&self) {
        let mut r = self.running.lock();
        *r = false;
    }

    pub fn is_running(&self) -> bool {
        is_running(self.running.clone())
    }

    pub fn start_bundling(&self) {
        if !self.is_running() {
            for bundler in self.bundlers.iter() {
                let bundler_own = bundler.clone();
                let interval = self.bundle_interval;
                let running_lock = self.running.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(interval));
                    loop {
                        if !is_running(running_lock.clone()) {
                            break;
                        }
                        interval.tick().await;

                        if let Err(e) = bundler_own.send_next_bundle().await {
                            error!("Error while sending bundle: {e:?}");
                        }
                    }
                });
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        net::{IpAddr, Ipv4Addr},
        str::FromStr,
    };

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
            "--bundler-grpc-listen-address",
            "127.0.0.1:3002",
            "--bundle-interval",
            "10",
        ];
        assert_eq!(
            BundlerOpts {
                beneficiary: Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990")
                    .unwrap(),
                gas_factor: U256::from(600),
                min_balance: U256::from(1),
                bundler_grpc_listen_address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    3002
                ),
                bundle_interval: 10,
            },
            BundlerOpts::try_parse_from(args).unwrap()
        );
    }
}
