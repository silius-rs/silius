use crate::{
    proto::{
        types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
        uopool::*,
    },
    utils::{parse_addr, parse_hash, parse_uo},
};
use alloy_chains::Chain;
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use eyre::Result;
use futures::{channel::mpsc::unbounded, StreamExt};
use parking_lot::RwLock;
use silius_mempool::{
    mempool_id, validate::validator::StandardUserOperationValidator, Mempool, MempoolErrorKind,
    MempoolId, Reputation, SanityCheck, SimulationCheck, SimulationTraceCheck,
    UoPool as UserOperationPool, UoPoolBuilder,
};
use silius_metrics::grpc::MetricsLayer;
use silius_p2p::{
    config::Config,
    service::{MempoolChannel, Network},
};
use silius_primitives::{p2p::NetworkMessage, provider::BlockStream, UoPoolMode};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Code, Request, Response, Status};
use tracing::{error, info};

type StandardUserPool<M, SanCk, SimCk, SimTrCk> =
    UserOperationPool<M, StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>>;

type UoPoolMaps<M, SanCk, SimCk, SimTrCk> =
    Arc<RwLock<HashMap<MempoolId, UoPoolBuilder<M, SanCk, SimCk, SimTrCk>>>>;

pub struct UoPoolService<M, SanCk, SimCk, SimTrCk>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M>,
    SimCk: SimulationCheck,
    SimTrCk: SimulationTraceCheck<M>,
{
    pub uopools: UoPoolMaps<M, SanCk, SimCk, SimTrCk>,
    pub chain: Chain,
}

impl<M, SanCk, SimCk, SimTrCk> UoPoolService<M, SanCk, SimCk, SimTrCk>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M> + Clone + 'static,
    SimCk: SimulationCheck + Clone + 'static,
    SimTrCk: SimulationTraceCheck<M> + Clone + 'static,
{
    pub fn new(uopools: UoPoolMaps<M, SanCk, SimCk, SimTrCk>, chain: Chain) -> Self {
        Self { uopools, chain }
    }

    #[allow(clippy::type_complexity)]
    fn get_uopool(
        &self,
        ep: &Address,
    ) -> tonic::Result<StandardUserPool<M, SanCk, SimCk, SimTrCk>> {
        let m_id = mempool_id(ep, self.chain.id());
        self.uopools
            .read()
            .get(&m_id)
            .map(|b| b.uopool())
            .ok_or(Status::new(Code::Unavailable, "User operation pool is not available"))
    }
}

#[async_trait]
impl<M, SanCk, SimCk, SimTrCk> uo_pool_server::UoPool for UoPoolService<M, SanCk, SimCk, SimTrCk>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M> + Clone + 'static,
    SimCk: SimulationCheck + Clone + 'static,
    SimTrCk: SimulationTraceCheck<M> + Clone + 'static,
{
    async fn add(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let res = {
            let uopool = self.get_uopool(&ep)?;
            uopool.validate_user_operation(&uo, None).await
        };

        let mut uopool = self.get_uopool(&ep)?;

        match uopool.add_user_operation(uo, res).await {
            Ok(uo_hash) => Ok(Response::new(AddResponse {
                res: AddResult::Added as i32,
                data: serde_json::to_string(&uo_hash)
                    .map_err(|err| Status::internal(format!("Failed to serialize hash: {err}")))?,
            })),
            Err(err) => match err.kind {
                MempoolErrorKind::InvalidUserOperation(_) => Ok(Response::new(AddResponse {
                    res: AddResult::NotAdded as i32,
                    data: serde_json::to_string(&err).map_err(|err| {
                        Status::internal(format!("Failed to serialize error: {err}"))
                    })?,
                })),
                _ => Err(Status::internal(format!("Internal error: {err}"))),
            },
        }
    }

    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uopool = self.get_uopool(&ep)?;

        uopool.remove_user_operations(req.uos.into_iter().map(|uo| uo.into()).collect());

        Ok(Response::new(()))
    }

    async fn get_chain_id(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetChainIdResponse>, Status> {
        Ok(Response::new(GetChainIdResponse { chain_id: self.chain.id() }))
    }

    async fn get_supported_entry_points(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, Status> {
        Ok(Response::new(GetSupportedEntryPointsResponse {
            eps: self
                .uopools
                .read()
                .values()
                .map(|mempool| mempool.uopool().entry_point.address().into())
                .collect(),
        }))
    }

    async fn estimate_user_operation_gas(
        &self,
        req: Request<EstimateUserOperationGasRequest>,
    ) -> Result<Response<EstimateUserOperationGasResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let uopool = self.get_uopool(&ep)?;

        Ok(Response::new(match uopool.estimate_user_operation_gas(&uo).await {
            Ok(gas) => EstimateUserOperationGasResponse {
                res: EstimateUserOperationGasResult::Estimated as i32,
                data: serde_json::to_string(&gas)
                    .map_err(|err| Status::internal(format!("Failed to serialize gas: {err}")))?,
            },
            Err(err) => EstimateUserOperationGasResponse {
                res: EstimateUserOperationGasResult::NotEstimated as i32,
                data: serde_json::to_string(&err)
                    .map_err(|err| Status::internal(format!("Failed to serialize error: {err}")))?,
            },
        }))
    }

    async fn get_sorted_user_operations(
        &self,
        req: Request<GetSortedRequest>,
    ) -> Result<Response<GetSortedResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;

        let uos = {
            let uopool = self.get_uopool(&ep)?;
            uopool.get_sorted_user_operations().map_err(|e| {
                tonic::Status::internal(format!("Get sorted uos internal error: {e:?}"))
            })?
        };

        let (uos_valid, storage_map) = {
            let mut uopool = self.get_uopool(&ep)?;
            uopool
                .bundle_user_operations(uos)
                .await
                .map_err(|e| tonic::Status::internal(format!("Bundle uos internal error: {e}")))?
        };

        Ok(Response::new(GetSortedResponse {
            uos: uos_valid.into_iter().map(Into::into).collect(),
            storage_map: Some(storage_map.into()),
        }))
    }

    async fn get_user_operation_by_hash(
        &self,
        req: Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationByHashResponse>, Status> {
        let req = req.into_inner();

        let uo_hash = parse_hash(req.hash)?;

        let keys: Vec<MempoolId> = self.uopools.read().keys().cloned().collect();
        for key in keys {
            let uopool = {
                let uopools_ref = self.uopools.read();
                let uopool_builder = uopools_ref.get(&key).expect("key must exist");
                uopool_builder.uopool()
            };
            if let Ok(uo_by_hash) = uopool.get_user_operation_by_hash(&uo_hash.into()).await {
                return Ok(Response::new(GetUserOperationByHashResponse {
                    user_operation: Some(uo_by_hash.user_operation.into()),
                    entry_point: Some(uo_by_hash.entry_point.into()),
                    transaction_hash: Some(uo_by_hash.transaction_hash.into()),
                    block_hash: Some(uo_by_hash.block_hash.into()),
                    block_number: uo_by_hash.block_number.as_u64(),
                }));
            }
        }

        Err(tonic::Status::not_found("User operation not found"))
    }

    async fn get_user_operation_receipt(
        &self,
        req: Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationReceiptResponse>, Status> {
        let req = req.into_inner();

        let uo_hash = parse_hash(req.hash)?;
        let keys: Vec<MempoolId> = self.uopools.read().keys().cloned().collect();
        for key in keys {
            let uopool = {
                let uopools_ref = self.uopools.read();
                let uopool_builder = uopools_ref.get(&key).expect("key must exist");
                uopool_builder.uopool()
            };
            if let Ok(uo_receipt) = uopool.get_user_operation_receipt(&uo_hash.into()).await {
                return Ok(Response::new(GetUserOperationReceiptResponse {
                    user_operation_hash: Some(uo_receipt.user_operation_hash.into()),
                    sender: Some(uo_receipt.sender.into()),
                    nonce: Some(uo_receipt.nonce.into()),
                    actual_gas_cost: Some(uo_receipt.actual_gas_cost.into()),
                    actual_gas_used: Some(uo_receipt.actual_gas_used.into()),
                    success: uo_receipt.success,
                    tx_receipt: Some(uo_receipt.tx_receipt.into()),
                    logs: uo_receipt.logs.into_iter().map(|log| log.into()).collect(),
                    paymaster: uo_receipt.paymaster.map(|p| p.into()),
                    reason: uo_receipt.reason,
                }));
            }
        }

        Err(tonic::Status::not_found("User operation receipt not found"))
    }

    async fn get_all(
        &self,
        req: Request<GetAllRequest>,
    ) -> Result<Response<GetAllResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uopool = self.get_uopool(&ep)?;
        match uopool.get_all() {
            Ok(uos) => {
                Ok(Response::new(GetAllResponse { uos: uos.into_iter().map(Into::into).collect() }))
            }
            Err(err) => Err(Status::unknown(format!("Internal error: {err:?}"))),
        }
    }

    async fn clear_mempool(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.read().values().for_each(|uopool| {
            uopool.uopool().clear_mempool();
        });
        Ok(Response::new(()))
    }

    async fn clear_reputation(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.read().values().for_each(|uopool| {
            uopool.uopool().clear_reputation();
        });
        Ok(Response::new(()))
    }

    async fn clear(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uopools.read().values().for_each(|uopool| {
            uopool.uopool().clear();
        });
        Ok(Response::new(()))
    }

    async fn get_all_reputation(
        &self,
        req: Request<GetAllReputationRequest>,
    ) -> Result<Response<GetAllReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uopool = self.get_uopool(&ep)?;

        Ok(Response::new(GetAllReputationResponse {
            rep: uopool.get_reputation().into_iter().map(Into::into).collect(),
        }))
    }

    async fn set_reputation(
        &self,
        req: Request<SetReputationRequest>,
    ) -> Result<Response<SetReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uopool = self.get_uopool(&ep)?;

        let res = Response::new(SetReputationResponse {
            res: match uopool.set_reputation(req.rep.iter().map(|re| re.clone().into()).collect()) {
                Ok(_) => SetReputationResult::Set as i32,
                Err(_) => SetReputationResult::NotSet as i32,
            },
        });

        Ok(res)
    }

    async fn add_mempool(
        &self,
        req: Request<AddMempoolRequest>,
    ) -> Result<Response<AddMempoolResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uopool = self.get_uopool(&ep)?;

        let res = Response::new(AddMempoolResponse {
            res: match uopool
                .add_user_operations(req.uos.into_iter().map(|uo| uo.into()).collect(), None)
                .await
            {
                Ok(_) => AddMempoolResult::AddedMempool as i32,
                Err(_) => AddMempoolResult::NotAddedMempool as i32,
            },
        });

        Ok(res)
    }

    async fn get_stake_info(
        &self,
        req: Request<GetStakeInfoRequest>,
    ) -> Result<Response<GetStakeInfoResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let addr = parse_addr(req.addr)?;
        let uopool = self.get_uopool(&ep)?;

        let res = uopool
            .get_stake_info(&addr)
            .await
            .map_err(|e| tonic::Status::internal(format!("Get stake info internal error: {e}")))?;
        Ok(Response::new(GetStakeInfoResponse {
            info: Some(res.stake_info.into()),
            is_staked: res.is_staked,
        }))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn uopool_service_run<M, SanCk, SimCk, SimTrCk>(
    addr: SocketAddr,
    mode: UoPoolMode,
    eps: Vec<Address>,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
    chain: Chain,
    max_verification_gas: U256,
    mempool: Mempool,
    reputation: Reputation,
    validator: StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>,
    p2p_config: Option<Config>,
    enable_metrics: bool,
) -> Result<()>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M> + Clone + 'static,
    SimCk: SimulationCheck + Clone + 'static,
    SimTrCk: SimulationTraceCheck<M> + Clone + 'static,
{
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let mut m_map = HashMap::<MempoolId, UoPoolBuilder<M, SanCk, SimCk, SimTrCk>>::new();

        // setup p2p
        if let Some(config) = p2p_config {
            let mut mempool_channels: Vec<MempoolChannel> = Vec::new();

            for (ep, block_stream) in eps.into_iter().zip(block_streams.into_iter()) {
                let id = mempool_id(&ep, chain.id());

                let (mempool_sender, mempool_receiver) = unbounded::<NetworkMessage>();

                let uo_builder = UoPoolBuilder::new(
                    mode,
                    eth_client.clone(),
                    ep,
                    chain,
                    max_verification_gas,
                    mempool.clone(),
                    reputation.clone(),
                    validator.clone(),
                    Some(mempool_sender),
                );
                uo_builder.register_block_updates(block_stream);
                uo_builder.register_reputation_updates();

                let (network_sender, mut network_receiver) = unbounded::<NetworkMessage>();
                let mut uo_pool = uo_builder.uopool();

                // spawn a task which would consume user operations received from p2p network
                tokio::spawn(async move {
                    while let Some(msg) = network_receiver.next().await {
                        if let NetworkMessage::Validate { user_operation, validation_config } = msg
                        {
                            let res = uo_pool
                                .validate_user_operation(&user_operation, Some(validation_config))
                                .await;
                            match uo_pool.add_user_operation(user_operation, res).await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("Failed to add user operation: {:?} from p2p", e)
                                }
                            }
                        }
                    }
                });

                m_map.insert(id, uo_builder);
                mempool_channels.push((ep, network_sender, mempool_receiver))
            }

            if config.bootnodes.is_empty() {
                info!("Starting p2p mode without bootnodes");
            }

            // fetch latest block information for p2p
            let latest_block_number = eth_client
                .get_block_number()
                .await
                .expect("get block number failed (needed for p2p)");
            let latest_block_hash = eth_client
                .get_block(latest_block_number)
                .await
                .expect("get block hash failed (needed for p2p)")
                .expect("get block hash failed (needed for p2p)");

            let mut p2p_network = Network::new(
                config.clone(),
                (latest_block_hash.hash.unwrap_or_default(), latest_block_number.as_u64()),
                mempool_channels,
            )
            .await
            .expect("p2p network init failed");

            tokio::spawn(async move {
                loop {
                    p2p_network.next_event().await;
                }
            });
        } else {
            for (ep, block_stream) in eps.into_iter().zip(block_streams.into_iter()) {
                let id = mempool_id(&ep, chain.id());
                let uo_builder = UoPoolBuilder::new(
                    mode,
                    eth_client.clone(),
                    ep,
                    chain,
                    max_verification_gas,
                    mempool.clone(),
                    reputation.clone(),
                    validator.clone(),
                    None,
                );
                uo_builder.register_block_updates(block_stream);
                uo_builder.register_reputation_updates();
                m_map.insert(id, uo_builder);
            }
        };

        let uopool_map = Arc::new(RwLock::new(m_map));
        let svc = uo_pool_server::UoPoolServer::new(
            UoPoolService::<M, SanCk, SimCk, SimTrCk>::new(uopool_map, chain),
        );

        if enable_metrics {
            builder.layer(MetricsLayer).add_service(svc).serve(addr).await
        } else {
            builder.add_service(svc).serve(addr).await
        }
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
