pub mod service;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use clap::Parser;
use ethers::{
    prelude::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{transaction::eip2718::TypedTransaction, Address, H256, U256},
};
use parking_lot::Mutex;
use serde::Deserialize;
use tonic::Request;
use tracing::{error, info, trace, warn};

use crate::{
    contracts::gen::EntryPointAPI,
    models::wallet::Wallet,
    types::user_operation::UserOperation,
    uopool::server::{
        bundler::Mode as GrpcMode,
        uopool::{uo_pool_client::UoPoolClient, GetSortedRequest, HandlePastEventRequest},
    },
    utils::{parse_address, parse_u256},
};

pub const DEFAULT_INTERVAL: u64 = 10;

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
    pub chain_id: U256,
    pub eth_client_address: String,
}

impl Bundler {
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
        entry_point: Address,
        chain_id: U256,
        eth_client_address: String,
    ) -> Self {
        Self {
            wallet,
            beneficiary,
            uopool_grpc_client,
            entry_point,
            chain_id,
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

    async fn send_next_bundle(&self, bundle: &Vec<UserOperation>) -> anyhow::Result<H256> {
        info!(
            "Creating the next bundle, got {} user operations",
            bundle.len()
        );
        let provider = Provider::<Http>::try_from(self.eth_client_address.clone())?;
        let client = Arc::new(SignerMiddleware::new(
            provider.clone(),
            self.wallet.signer.clone(),
        ));
        let entry_point = EntryPointAPI::new(self.entry_point, client.clone());
        let nonce = client
            .clone()
            .get_transaction_count(self.wallet.signer.address(), None)
            .await?;
        let mut tx: TypedTransaction = entry_point
            .handle_ops(
                bundle.clone().into_iter().map(Into::into).collect(),
                self.beneficiary,
            )
            .tx
            .clone();
        tx.set_nonce(nonce).set_chain_id(self.chain_id.as_u64());

        trace!("Prepare the transaction {tx:?} send to execution client!");
        let tx = client.send_transaction(tx, None).await?;
        let tx_hash = tx.tx_hash();
        trace!("Send bundle with transaction: {tx:?}");

        info!("Send handlePastEvents request");
        if let Some(e) = self
            .uopool_grpc_client
            .clone()
            .handle_past_events(Request::new(HandlePastEventRequest {
                entry_point: Some(self.entry_point.into()),
            }))
            .await
            .err()
        {
            warn!("Failed to handle past events: {:?}", e)
        };
        Ok(tx_hash)
    }
}

pub struct BundlerService {
    pub bundlers: Vec<Bundler>,
    pub running: Arc<Mutex<bool>>,
}

fn is_running(running: Arc<Mutex<bool>>) -> bool {
    let r = running.lock();
    *r
}

impl BundlerService {
    pub fn new(
        wallet: Wallet,
        beneficiary: Address,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
        entry_points: Vec<Address>,
        chain_id: U256,
        eth_client_address: String,
    ) -> Self {
        let bundlers: Vec<Bundler> = entry_points
            .iter()
            .map(|entry_point| {
                Bundler::new(
                    wallet.clone(),
                    beneficiary,
                    uopool_grpc_client.clone(),
                    *entry_point,
                    chain_id,
                    eth_client_address.clone(),
                )
            })
            .collect();

        Self {
            bundlers,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn send_bundles_now(&self) -> anyhow::Result<H256> {
        info!("Sending bundles now");
        let mut tx_hashes: Vec<H256> = vec![];
        for bundler in self.bundlers.iter() {
            info!("Sending bundle for entry point: {:?}", bundler.entry_point);

            let bundle = bundler.create_bundle().await?;
            let tx_hash = bundler.send_next_bundle(&bundle).await?;

            tx_hashes.push(tx_hash)
        }

        // FIXME: Because currently the bundler support multiple bundler and
        // we don't have a way to know which bundler is the one that is
        Ok(tx_hashes
            .into_iter()
            .next()
            .expect("Must have at least one tx hash"))
    }

    pub fn stop_bundling(&self) {
        info!("Stopping auto bundling");
        let mut r = self.running.lock();
        *r = false;
    }

    pub fn is_running(&self) -> bool {
        is_running(self.running.clone())
    }

    pub fn start_bundling(&self, interval: u64) {
        if !self.is_running() {
            for bundler in self.bundlers.iter() {
                info!(
                    "Starting auto bundling process for entry point: {:?}",
                    bundler.entry_point
                );
                let bundler_own = bundler.clone();
                let running_lock = self.running.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(interval));
                    loop {
                        if !is_running(running_lock.clone()) {
                            break;
                        }
                        interval.tick().await;

                        match bundler_own.create_bundle().await {
                            Ok(bundle) => {
                                if let Err(e) = bundler_own.send_next_bundle(&bundle).await {
                                    error!("Error while sending bundle: {e:?}");
                                }
                            }
                            Err(e) => {
                                error!("Error while creating bundle: {e:?}");
                            }
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
