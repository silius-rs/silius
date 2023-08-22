use crate::proto::bundler::*;
use crate::proto::uopool::{GetSortedRequest, HandlePastEventRequest};
use crate::uo_pool_client::UoPoolClient;
use async_trait::async_trait;
use ethers::types::{Address, H256, U256};
use parking_lot::Mutex;
use silius_bundler::{Bundler, SendBundleMode};
use silius_primitives::{Chain, UserOperation, Wallet};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

pub struct BundlerService {
    pub bundlers: Vec<Bundler>,
    pub running: Arc<Mutex<bool>>,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

fn is_running(running: Arc<Mutex<bool>>) -> bool {
    let r = running.lock();
    *r
}

impl BundlerService {
    pub fn new(
        bundlers: Vec<Bundler>,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    ) -> Self {
        Self {
            bundlers,
            running: Arc::new(Mutex::new(false)),
            uopool_grpc_client,
        }
    }

    async fn get_user_operations(
        uopool_grpc_client: &UoPoolClient<tonic::transport::Channel>,
        ep: &Address,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let req = Request::new(GetSortedRequest {
            ep: Some((*ep).into()),
        });
        let res = uopool_grpc_client
            .clone()
            .get_sorted_user_operations(req)
            .await?;

        let uos: Vec<UserOperation> = res.into_inner().uos.into_iter().map(|u| u.into()).collect();
        Ok(uos)
    }

    pub async fn send_bundles(&self) -> anyhow::Result<H256> {
        let mut tx_hashes: Vec<H256> = vec![];

        for bundler in self.bundlers.iter() {
            let uos =
                Self::get_user_operations(&self.uopool_grpc_client, &bundler.entry_point).await?;
            let tx_hash = bundler.send_next_bundle(&uos).await?;

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
        ep: &Address,
    ) -> anyhow::Result<()> {
        let req = Request::new(HandlePastEventRequest {
            ep: Some((*ep).into()),
        });

        if let Some(e) = uopool_grpc_client
            .clone()
            .handle_past_events(req)
            .await
            .err()
        {
            warn!("Failed to handle past events: {:?}", e)
        };

        Ok(())
    }

    pub fn start_bundling(&self, int: u64) {
        if !self.is_running() {
            info!("Starting auto bundling");

            let mut r = self.running.lock();
            *r = true;

            for bundler in self.bundlers.iter() {
                let bundler_own = bundler.clone();
                let running_lock = self.running.clone();
                let uopool_grpc_client = self.uopool_grpc_client.clone();

                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(int));
                    loop {
                        if !is_running(running_lock.clone()) {
                            break;
                        }
                        interval.tick().await;

                        match Self::get_user_operations(
                            &uopool_grpc_client,
                            &bundler_own.entry_point,
                        )
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
    async fn set_bundler_mode(
        &self,
        req: Request<SetModeRequest>,
    ) -> Result<Response<SetModeResponse>, Status> {
        let req = req.into_inner();

        match req.mode() {
            Mode::Manual => {
                self.stop_bundling();
                Ok(Response::new(SetModeResponse {
                    res: SetModeResult::Ok.into(),
                }))
            }
            Mode::Auto => {
                let int = req.interval;
                self.start_bundling(int);
                Ok(Response::new(SetModeResponse {
                    res: SetModeResult::Ok.into(),
                }))
            }
        }
    }

    async fn send_bundle_now(
        &self,
        _req: Request<()>,
    ) -> Result<Response<SendBundleNowResponse>, Status> {
        let res = self
            .send_bundles()
            .await
            .map_err(|e| tonic::Status::internal(format!("Send bundle now with error: {e:?}")))?;
        Ok(Response::new(SendBundleNowResponse {
            res: Some(res.into()),
        }))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn bundler_service_run(
    grpc_listen_address: SocketAddr,
    wallet: Wallet,
    eps: Vec<Address>,
    eth_client_address: String,
    chain: Chain,
    beneficiary: Address,
    min_balance: U256,
    bundle_interval: u64,
    uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    send_bundle_mode: SendBundleMode,
    relay_endpoints: Option<Vec<String>>,
) {
    let bundlers: Vec<Bundler> = eps
        .iter()
        .map(|ep| {
            Bundler::new(
                wallet.clone(),
                eth_client_address.clone(),
                beneficiary,
                *ep,
                chain,
                send_bundle_mode.clone(),
                relay_endpoints.clone(),
                min_balance,
            )
            .expect("Failed to create bundler")
        })
        .collect();

    let bundler_service = BundlerService::new(bundlers, uopool_grpc_client);

    info!("Bundler gRPC server starting on {}", grpc_listen_address);

    bundler_service.start_bundling(bundle_interval);

    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = bundler_server::BundlerServer::new(bundler_service);
        builder.add_service(svc).serve(grpc_listen_address).await
    });
}
