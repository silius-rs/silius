use crate::proto::uopool::*;
use crate::{
    builder::UoPoolBuiler,
    proto::types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
    utils::{parse_addr, parse_hash, parse_uo},
};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use silius_contracts::entry_point::EntryPointErr;
use silius_primitives::reputation::ReputationEntry;
use silius_primitives::{uopool::AddError, Chain, UoPoolMode};
use silius_uopool::{
    mempool_id, validate::validator::StandardUserOperationValidator, MemoryMempool,
    MemoryReputation, MempoolId, Reputation, UoPool as UserOperationPool,
};
use silius_uopool::{Mempool, VecCh, VecUo};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Code, Request, Response, Status};
use tracing::{info, warn};

pub const MAX_UOS_PER_UNSTAKED_SENDER: usize = 4;
pub const GAS_INCREASE_PERC: u64 = 10;

type StandardUserPool<M, P, R> =
    UserOperationPool<M, StandardUserOperationValidator<M, P, R>, P, R>;

pub struct UoPoolService<M, P, R>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    pub uo_pools: Arc<DashMap<MempoolId, UoPoolBuiler<M, P, R>>>,
    pub chain: Chain,
}

impl<M, P, R> UoPoolService<M, P, R>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    pub fn new(uo_pools: Arc<DashMap<MempoolId, UoPoolBuiler<M, P, R>>>, chain: Chain) -> Self {
        Self { uo_pools, chain }
    }

    fn get_uo_pool(&self, ep: &Address) -> tonic::Result<StandardUserPool<M, P, R>> {
        let m_id = mempool_id(ep, &U256::from(self.chain.id()));
        self.uo_pools
            .get(&m_id)
            .map(|b| b.uo_pool())
            .ok_or(Status::new(
                Code::Unavailable,
                "User operation pool is not available",
            ))
    }
}

#[async_trait]
impl<M, P, R> uo_pool_server::UoPool for UoPoolService<M, P, R>
where
    EntryPointErr: From<<M as Middleware>::Error>,
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error>
        + Send
        + Sync
        + 'static,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error>
        + Send
        + Sync
        + 'static,
{
    async fn add(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let res = {
            let uo_pool = self.get_uo_pool(&ep)?;
            match uo_pool.validate_user_operation(&uo).await {
                Ok(res) => res,
                Err(err) => {
                    return Ok(Response::new(AddResponse {
                        res: AddResult::NotAdded as i32,
                        data: serde_json::to_string(&err).map_err(|err| {
                            Status::internal(format!("Failed to serialize error: {err}"))
                        })?,
                    }))
                }
            }
        };

        let mut uo_pool = self.get_uo_pool(&ep)?;

        match uo_pool.add_user_operation(uo, res).await {
            Ok(uo_hash) => Ok(Response::new(AddResponse {
                res: AddResult::Added as i32,
                data: serde_json::to_string(&uo_hash)
                    .map_err(|err| Status::internal(format!("Failed to serialize hash: {err}")))?,
            })),
            Err(err) => match err {
                AddError::Verification(err) => Ok(Response::new(AddResponse {
                    res: AddResult::NotAdded as i32,
                    data: serde_json::to_string(&err).map_err(|err| {
                        Status::internal(format!("Failed to serialize error: {err}"))
                    })?,
                })),
                AddError::MempoolError { message } => {
                    Err(Status::internal(format!("Internal error: {message}")))
                }
            },
        }
    }

    async fn remove(&self, req: Request<RemoveRequest>) -> Result<Response<()>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uo_pool = self.get_uo_pool(&ep)?;

        uo_pool.remove_user_operations(req.hashes.into_iter().map(Into::into).collect());

        Ok(Response::new(()))
    }

    async fn get_chain_id(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetChainIdResponse>, Status> {
        Ok(Response::new(GetChainIdResponse {
            chain_id: self.chain.id(),
        }))
    }

    async fn get_supported_entry_points(
        &self,
        _req: Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, Status> {
        Ok(Response::new(GetSupportedEntryPointsResponse {
            eps: self
                .uo_pools
                .iter()
                .map(|mempool| mempool.uo_pool().entry_point_address().into())
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

        let uo_pool = self.get_uo_pool(&ep)?;

        Ok(Response::new(
            match uo_pool.estimate_user_operation_gas(&uo).await {
                Ok(gas) => EstimateUserOperationGasResponse {
                    res: EstimateUserOperationGasResult::Estimated as i32,
                    data: serde_json::to_string(&gas).map_err(|err| {
                        Status::internal(format!("Failed to serialize gas: {err}"))
                    })?,
                },
                Err(err) => EstimateUserOperationGasResponse {
                    res: EstimateUserOperationGasResult::NotEstimated as i32,
                    data: serde_json::to_string(&err).map_err(|err| {
                        Status::internal(format!("Failed to serialize error: {err}"))
                    })?,
                },
            },
        ))
    }

    async fn get_sorted_user_operations(
        &self,
        req: Request<GetSortedRequest>,
    ) -> Result<Response<GetSortedResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;

        let uos = {
            let uo_pool = self.get_uo_pool(&ep)?;
            uo_pool.get_sorted_user_operations().map_err(|e| {
                tonic::Status::internal(format!("Get sorted uos internal error: {e}"))
            })?
        };

        let uos_valid = {
            let mut uo_pool = self.get_uo_pool(&ep)?;
            uo_pool
                .bundle_user_operations(uos)
                .await
                .map_err(|e| tonic::Status::internal(format!("Bundle uos internal error: {e}")))?
        };

        Ok(Response::new(GetSortedResponse {
            uos: uos_valid.into_iter().map(Into::into).collect(),
        }))
    }

    async fn handle_past_events(
        &self,
        req: Request<HandlePastEventRequest>,
    ) -> Result<Response<()>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uo_pool = self.get_uo_pool(&ep)?;

        uo_pool
            .handle_past_events()
            .await
            .map_err(|e| tonic::Status::internal(format!("Failed to handle past events: {e:?}")))?;

        Ok(Response::new(()))
    }

    async fn get_user_operation_by_hash(
        &self,
        req: Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationByHashResponse>, Status> {
        let req = req.into_inner();

        let uo_hash = parse_hash(req.hash)?;

        for uo_pool in self.uo_pools.iter() {
            if let Ok(uo_by_hash) = uo_pool
                .uo_pool()
                .get_user_operation_by_hash(&uo_hash.into())
                .await
            {
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

        for uo_pool in self.uo_pools.iter() {
            if let Ok(uo_receipt) = uo_pool
                .uo_pool()
                .get_user_operation_receipt(&uo_hash.into())
                .await
            {
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
        let uo_pool = self.get_uo_pool(&ep)?;

        Ok(Response::new(GetAllResponse {
            uos: uo_pool.get_all().into_iter().map(Into::into).collect(),
        }))
    }

    async fn clear(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uo_pools.iter_mut().for_each(|uo_pool| {
            uo_pool.uo_pool().clear();
        });
        Ok(Response::new(()))
    }

    async fn get_all_reputation(
        &self,
        req: Request<GetAllReputationRequest>,
    ) -> Result<Response<GetAllReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uo_pool = self.get_uo_pool(&ep)?;

        Ok(Response::new(GetAllReputationResponse {
            rep: uo_pool
                .get_reputation()
                .into_iter()
                .map(Into::into)
                .collect(),
        }))
    }

    async fn set_reputation(
        &self,
        req: Request<SetReputationRequest>,
    ) -> Result<Response<SetReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let mut uo_pool = self.get_uo_pool(&ep)?;

        let res = Response::new(SetReputationResponse {
            res: match uo_pool.set_reputation(req.rep.iter().map(|re| re.clone().into()).collect())
            {
                Ok(_) => SetReputationResult::SetReputation as i32,
                Err(_) => SetReputationResult::NotSetReputation as i32,
            },
        });

        Ok(res)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn uopool_service_run(
    grpc_listen_address: SocketAddr,
    eps: Vec<Address>,
    eth_client: Arc<Provider<Http>>,
    chain: Chain,
    max_verification_gas: U256,
    min_stake: U256,
    min_unstake_delay: U256,
    min_priority_fee_per_gas: U256,
    whitelist: Vec<Address>,
    uo_pool_mode: UoPoolMode,
) -> Result<()> {
    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let m_map = Arc::new(DashMap::<
            MempoolId,
            UoPoolBuiler<Provider<Http>, MemoryMempool, MemoryReputation>,
        >::new());

        for ep in eps {
            let id = mempool_id(&ep, &U256::from(chain.id()));
            let builder = UoPoolBuiler::new(
                uo_pool_mode == UoPoolMode::Unsafe,
                eth_client.clone(),
                ep,
                chain,
                max_verification_gas,
                min_stake,
                min_unstake_delay,
                min_priority_fee_per_gas,
                whitelist.clone(),
                MemoryMempool::default(),
                MemoryReputation::default(),
            );
            m_map.insert(id, builder);
        }

        let svc = uo_pool_server::UoPoolServer::new(UoPoolService::<
            Provider<Http>,
            MemoryMempool,
            MemoryReputation,
        >::new(m_map.clone(), chain));

        tokio::spawn(async move {
            loop {
                m_map.iter_mut().for_each(|m| {
                    let _ = m
                        .uo_pool()
                        .reputation
                        .update_hourly()
                        .map_err(|e| warn!("Failed to update hourly reputation: {:?}", e));
                });
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });

        info!("UoPool gRPC server starting on {}", grpc_listen_address);

        builder.add_service(svc).serve(grpc_listen_address).await
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
