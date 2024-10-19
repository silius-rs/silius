use crate::{
    proto::{
        bundler::*,
        uopool::{GetSortedRequest, RemoveRequest},
    },
    uo_pool_client::UoPoolClient,
};
use alloy_chains::Chain;
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, H256, U256},
};
use parking_lot::Mutex;
use silius_bundler::{Bundler, SendBundleOp};
use silius_metrics::grpc::MetricsLayer;
use silius_primitives::{simulation::StorageMap, UserOperation, Wallet};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub struct BundlerService<M, S>
where
    M: Middleware + Clone + 'static,
    S: SendBundleOp + Clone + 'static,
{
    pub bundlers: Vec<Bundler<M, S>>,
    pub running: Arc<Mutex<bool>>,
    pub uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
}

fn is_running(running: Arc<Mutex<bool>>) -> bool {
    let r = running.lock();
    *r
}

impl<M, S> BundlerService<M, S>
where
    M: Middleware + Clone + 'static,
    S: SendBundleOp + Clone + 'static,
{
    pub fn new(
        bundlers: Vec<Bundler<M, S>>,
        uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    ) -> Self {
        Self { bundlers, running: Arc::new(Mutex::new(false)), uopool_grpc_client }
    }

    async fn get_user_operations(
        uopool_grpc_client: &UoPoolClient<tonic::transport::Channel>,
        ep: &Address,
    ) -> eyre::Result<(Vec<UserOperation>, StorageMap)> {
        let req = Request::new(GetSortedRequest { ep: Some((*ep).into()) });
        let res = uopool_grpc_client.clone().get_sorted_user_operations(req).await?;

        let res = res.into_inner();

        let uos: Vec<UserOperation> = res.uos.into_iter().map(|u| u.into()).collect();
        let map = match res.storage_map {
            Some(map) => map.into(),
            None => StorageMap::default(),
        };

        Ok((uos, map))
    }

    pub async fn send_bundles(&self) -> eyre::Result<(Vec<UserOperation>, Option<H256>)> {
        let mut tx_hashes: Vec<Option<H256>> = vec![];
        let mut user_operations: Vec<Vec<UserOperation>> = vec![];

        for bundler in self.bundlers.iter() {
            let (uos, map) =
                Self::get_user_operations(&self.uopool_grpc_client, &bundler.entry_point).await?;
            let tx_hash = bundler.send_bundle(&uos, map).await?;

            tx_hashes.push(tx_hash);
            user_operations.push(uos);
        }

        // FIXME: Because currently the bundler support multiple bundler and
        // we don't have a way to know which bundler is the one that is
        Ok((
            user_operations.first().expect("At least one bundler must be present").to_vec(),
            tx_hashes.into_iter().next().expect("At least one bundler must be present"),
        ))
    }

    pub fn stop_bundling(&self) {
        info!("Stopping auto bundling");
        let mut r = self.running.lock();
        *r = false;
    }

    pub fn is_running(&self) -> bool {
        is_running(self.running.clone())
    }

    pub fn start_bundling(&self, int: u64) {
        if !self.is_running() {
            info!("Starting auto bundling");

            {
                let mut r = self.running.lock();
                *r = true;
            }

            for bundler in self.bundlers.iter() {
                let bundler_own = bundler.clone();
                let running_lock = self.running.clone();
                let uopool_grpc_client = self.uopool_grpc_client.clone();

                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(int));
                    loop {
                        interval.tick().await;

                        if !is_running(running_lock.clone()) {
                            break;
                        }

                        match Self::get_user_operations(
                            &uopool_grpc_client,
                            &bundler_own.entry_point,
                        )
                        .await
                        {
                            Ok((bundle, map)) => {
                                if let Err(e) = bundler_own.send_bundle(&bundle, map).await {
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

#[async_trait]
impl<M, S> bundler_server::Bundler for BundlerService<M, S>
where
    M: Middleware + Clone + 'static,
    S: SendBundleOp + Clone + 'static,
{
    async fn set_bundle_mode(
        &self,
        req: Request<SetBundleModeRequest>,
    ) -> Result<Response<SetBundleModeResponse>, Status> {
        let req = req.into_inner();

        match req.mode() {
            Mode::Manual => {
                self.stop_bundling();
                Ok(Response::new(SetBundleModeResponse { res: SetBundleModeResult::Ok.into() }))
            }
            Mode::Auto => {
                let int = req.interval;
                self.start_bundling(int);
                Ok(Response::new(SetBundleModeResponse { res: SetBundleModeResult::Ok.into() }))
            }
        }
    }

    async fn send_bundle_now(
        &self,
        _req: Request<()>,
    ) -> Result<Response<SendBundleNowResponse>, Status> {
        let (uos, tx_hash) = self
            .send_bundles()
            .await
            .map_err(|e| tonic::Status::internal(format!("Send bundle now with error: {e:?}")))?;

        if let Some(tx_hash) = tx_hash {
            // wait for the tx to be mined
            loop {
                let tx_receipt = self
                    .bundlers
                    .first()
                    .expect("Must have at least one bundler")
                    .eth_client
                    .get_transaction_receipt(tx_hash)
                    .await;
                if let Ok(tx_receipt) = tx_receipt {
                    if tx_receipt.is_some() {
                        self.uopool_grpc_client
                            .clone()
                            .remove(Request::new(RemoveRequest {
                                uos: uos.into_iter().map(|uo| uo.into()).collect(),
                                ep: Some(
                                    self.bundlers
                                        .first()
                                        .expect("Must have at least one bundler")
                                        .entry_point
                                        .into(),
                                ),
                            }))
                            .await?;
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        Ok(Response::new(SendBundleNowResponse { res: Some(tx_hash.unwrap_or_default().into()) }))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn bundler_service_run<M, S>(
    addr: SocketAddr,
    wallet: Wallet,
    eps: Vec<Address>,
    chain: Chain,
    beneficiary: Address,
    min_balance: U256,
    bundle_interval: Option<u64>,
    eth_client: Arc<M>,
    client: Arc<S>,
    uopool_grpc_client: UoPoolClient<tonic::transport::Channel>,
    enable_metrics: bool,
    enable_access_list: bool,
) where
    M: Middleware + Clone + 'static,
    S: SendBundleOp + Clone + 'static,
{
    let bundlers: Vec<Bundler<M, S>> = eps
        .into_iter()
        .map(|ep| {
            Bundler::new(
                wallet.clone(),
                beneficiary,
                ep,
                chain,
                min_balance,
                eth_client.clone(),
                client.clone(),
                enable_access_list,
            )
        })
        .collect();

    let bundler_service = BundlerService::new(bundlers, uopool_grpc_client);
    if let Some(bundle_interval) = bundle_interval {
        bundler_service.start_bundling(bundle_interval);
    }

    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();
        let svc = bundler_server::BundlerServer::new(bundler_service);
        if enable_metrics {
            builder.layer(MetricsLayer).add_service(svc).serve(addr).await
        } else {
            builder.add_service(svc).serve(addr).await
        }
    });
}
