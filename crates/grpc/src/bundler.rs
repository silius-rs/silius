use std::{net::SocketAddr, sync::Arc, time::Duration};

use aa_bundler_bundler::Bundler as BundlerCore;
use aa_bundler_primitives::{parse_address, parse_u256, Chain, UserOperation, Wallet};
use async_trait::async_trait;
use clap::Parser;
use ethers::types::{Address, H256, U256};
use parking_lot::Mutex;
use tonic::Response;
use tracing::{error, info, warn};

use crate::proto::uopool::{GetSortedRequest, HandlePastEventRequest};
use crate::{GetChainIdResponse, GetSupportedEntryPointsResponse};

use crate::proto::bundler::*;
use crate::uo_pool_client::UoPoolClient;

#[derive(Clone, Copy, Debug, Parser, PartialEq)]
pub struct BundlerServiceOpts {
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

pub struct BundlerService {
    pub bundlers: Vec<BundlerCore>,
    pub running: Arc<Mutex<bool>>,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
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
        chain: Chain,
        eth_client_address: String,
    ) -> Self {
        let bundlers: Vec<BundlerCore> = entry_points
            .iter()
            .map(|entry_point| {
                BundlerCore::new(
                    wallet.clone(),
                    beneficiary,
                    *entry_point,
                    chain,
                    eth_client_address.clone(),
                )
            })
            .collect();

        Self {
            bundlers,
            running: Arc::new(Mutex::new(false)),
            uopool_grpc_client,
        }
    }

    async fn create_bundle(
        uopool_grpc_client: &UoPoolClient<tonic::transport::Channel>,
        entry_point: &Address,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let request = tonic::Request::new(GetSortedRequest {
            entry_point: Some((*entry_point).into()),
        });
        let response = uopool_grpc_client
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

    pub async fn send_bundles_now(&self) -> anyhow::Result<H256> {
        info!("Sending bundles now");
        let mut tx_hashes: Vec<H256> = vec![];
        for bundler in self.bundlers.iter() {
            info!("Sending bundle for entry point: {:?}", bundler.entry_point);

            let bundle =
                Self::create_bundle(&self.uopool_grpc_client, &bundler.entry_point).await?;
            let tx_hash = bundler.send_next_bundle(&bundle).await?;

            Self::handle_past_events(&self.uopool_grpc_client, &bundler.entry_point).await?;

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

    async fn handle_past_events(
        uopool_grpc_client: &UoPoolClient<tonic::transport::Channel>,
        entry_point: &Address,
    ) -> anyhow::Result<()> {
        info!("Send handlePastEvents request");

        let request = tonic::Request::new(HandlePastEventRequest {
            entry_point: Some((*entry_point).into()),
        });

        if let Some(e) = uopool_grpc_client
            .clone()
            .handle_past_events(request)
            .await
            .err()
        {
            warn!("Failed to handle past events: {:?}", e)
        };

        Ok(())
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
                let uopool_grpc_client = self.uopool_grpc_client.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(interval));
                    loop {
                        if !is_running(running_lock.clone()) {
                            break;
                        }
                        interval.tick().await;

                        match Self::create_bundle(&uopool_grpc_client, &bundler_own.entry_point)
                            .await
                        {
                            Ok(bundle) => {
                                if let Err(e) = bundler_own.send_next_bundle(&bundle).await {
                                    error!("Error while sending bundle: {e:?}");
                                }
                                if let Err(e) = Self::handle_past_events(
                                    &uopool_grpc_client,
                                    &bundler_own.entry_point,
                                )
                                .await
                                {
                                    error!("Error while handling past events: {e:?}");
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

#[async_trait]
impl bundler_server::Bundler for BundlerService {
    async fn chain_id(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetChainIdResponse>, tonic::Status> {
        todo!()
    }

    async fn supported_entry_points(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, tonic::Status> {
        todo!()
    }

    async fn set_bundler_mode(
        &self,
        request: tonic::Request<SetModeRequest>,
    ) -> Result<Response<SetModeResponse>, tonic::Status> {
        let req = request.into_inner();
        match req.mode() {
            Mode::Manual => {
                info!("Stopping auto bundling");
                self.stop_bundling();
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
            Mode::Auto => {
                let interval = req.interval;
                self.start_bundling(interval);
                Ok(Response::new(SetModeResponse {
                    result: SetModeResult::Ok.into(),
                }))
            }
        }
    }

    async fn send_bundle_now(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<SendBundleNowResponse>, tonic::Status> {
        let res = self.send_bundles_now().await.map_err(|e| {
            error!("Send bundle manually with response {e:?}");
            tonic::Status::internal(format!("Send bundle now with error: {e:?}"))
        })?;
        Ok(Response::new(SendBundleNowResponse {
            result: Some(res.into()),
        }))
    }
}

pub fn bundler_service_run(
    opts: BundlerServiceOpts,
    wallet: Wallet,
    entry_points: Vec<Address>,
    chain: Chain,
    eth_client_address: String,
    uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
) {
    let bundler_service = BundlerService::new(
        wallet,
        opts.beneficiary,
        uopool_grpc_client,
        entry_points,
        chain,
        eth_client_address,
    );

    info!("Starting bundler manager");

    bundler_service.start_bundling(opts.bundle_interval);

    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = bundler_server::BundlerServer::new(bundler_service);
        builder
            .add_service(svc)
            .serve(opts.bundler_grpc_listen_address)
            .await
    });
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
            BundlerServiceOpts {
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
            BundlerServiceOpts::try_parse_from(args).unwrap()
        );
    }
}
