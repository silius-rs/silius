use crate::{
    proto::types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
    utils::{parse_addr, parse_hash, parse_uo, parse_uo_pool_mut},
};
use crate::{proto::uopool::*, utils::parse_uo_pool};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, H256, U256},
};
use silius_contracts::{entry_point::EntryPointErr, EntryPoint};
use silius_primitives::{
    reputation::{BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK},
    uopool::AddError,
    Chain, UoPoolMode,
};
use silius_uopool::{
    mempool_id,
    validate::{
        sanity::{
            call_gas::CallGas, max_fee::MaxFee, paymaster::Paymaster, sender::SenderOrInitCode,
            sender_uos::SenderUos, verification_gas::VerificationGas,
        },
        simulation::{signature::Signature, timestamp::Timestamp},
        simulation_trace::{
            call_stack::CallStack, code_hashes::CodeHashes, external_contracts::ExternalContracts,
            gas::Gas, opcodes::Opcodes, storage_access::StorageAccess,
        },
        validator::StandardUserOperationValidator,
        UserOperationValidator,
    },
    MemoryMempool, MemoryReputation, MempoolId, Reputation, UoPool as UserOperationPool,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tonic::{Request, Response, Status};
use tracing::{info, warn};

const MAX_UOS_PER_UNSTAKED_SENDER: usize = 4;
const GAS_INCREASE_PERC: u64 = 10;

pub struct UoPoolService<M: Middleware + 'static, V: UserOperationValidator> {
    pub uo_pools: Arc<DashMap<MempoolId, UserOperationPool<M, V>>>,
    pub chain: Chain,
}

impl<M: Middleware + 'static, V: UserOperationValidator> UoPoolService<M, V> {
    pub fn new(uo_pools: Arc<DashMap<MempoolId, UserOperationPool<M, V>>>, chain: Chain) -> Self {
        Self { uo_pools, chain }
    }

    fn get_uo_pool(&self, ep: &Address) -> Option<Ref<H256, UserOperationPool<M, V>>> {
        let m_id = mempool_id(ep, &U256::from(self.chain.id()));
        self.uo_pools.get(&m_id)
    }

    fn get_uo_pool_mut(&self, ep: &Address) -> Option<RefMut<H256, UserOperationPool<M, V>>> {
        let m_id = mempool_id(ep, &U256::from(self.chain.id()));
        self.uo_pools.get_mut(&m_id)
    }
}

#[async_trait]
impl<M: Middleware + 'static, V: UserOperationValidator + 'static> uo_pool_server::UoPool
    for UoPoolService<M, V>
where
    EntryPointErr: From<<M as Middleware>::Error>,
{
    async fn add(&self, req: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = req.into_inner();

        let uo = parse_uo(req.uo)?;
        let ep = parse_addr(req.ep)?;

        let res = {
            let uo_pool = parse_uo_pool(self.get_uo_pool(&ep))?;
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

        let mut uo_pool = parse_uo_pool_mut(self.get_uo_pool_mut(&ep))?;

        match uo_pool.add_user_operation(uo, Some(res)).await {
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
        let mut uo_pool = parse_uo_pool_mut(self.get_uo_pool_mut(&ep))?;

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
                .map(|mempool| mempool.entry_point_address().into())
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

        let uo_pool = parse_uo_pool(self.get_uo_pool(&ep))?;

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
            let uo_pool = parse_uo_pool(self.get_uo_pool(&ep))?;
            uo_pool.get_sorted_user_operations().map_err(|e| {
                tonic::Status::internal(format!("Get sorted uos internal error: {e}"))
            })?
        };

        let uos_valid = {
            let mut uo_pool = parse_uo_pool_mut(self.get_uo_pool_mut(&ep))?;
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
        let mut uo_pool = parse_uo_pool_mut(self.get_uo_pool_mut(&ep))?;

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
            if let Ok(uo_by_hash) = uo_pool.get_user_operation_by_hash(&uo_hash.into()).await {
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
            if let Ok(uo_receipt) = uo_pool.get_user_operation_receipt(&uo_hash.into()).await {
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
        let uo_pool = parse_uo_pool(self.get_uo_pool(&ep))?;

        Ok(Response::new(GetAllResponse {
            uos: uo_pool.get_all().into_iter().map(Into::into).collect(),
        }))
    }

    async fn clear(&self, _req: Request<()>) -> Result<Response<()>, Status> {
        self.uo_pools.iter_mut().for_each(|mut uo_pool| {
            uo_pool.clear();
        });
        Ok(Response::new(()))
    }

    async fn get_all_reputation(
        &self,
        req: Request<GetAllReputationRequest>,
    ) -> Result<Response<GetAllReputationResponse>, Status> {
        let req = req.into_inner();

        let ep = parse_addr(req.ep)?;
        let uo_pool = parse_uo_pool(self.get_uo_pool(&ep))?;

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
        let mut uo_pool = parse_uo_pool_mut(self.get_uo_pool_mut(&ep))?;

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
            UserOperationPool<Provider<Http>, StandardUserOperationValidator<Provider<Http>>>,
        >::new());

        for ep in eps {
            let id = mempool_id(&ep, &U256::from(chain.id()));

            let mut reputation = Box::<MemoryReputation>::default();
            reputation.init(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                min_stake,
                min_unstake_delay,
            );
            for addr in whitelist.iter() {
                reputation.add_whitelist(addr);
            }

            let entry_point = EntryPoint::<Provider<Http>>::new(eth_client.clone(), ep);

            let mut validator =
                StandardUserOperationValidator::new(eth_client.clone(), entry_point.clone(), chain)
                    .with_sanity_check(SenderOrInitCode)
                    .with_sanity_check(VerificationGas {
                        max_verification_gas,
                    })
                    .with_sanity_check(Paymaster)
                    .with_sanity_check(CallGas)
                    .with_sanity_check(MaxFee {
                        min_priority_fee_per_gas,
                    })
                    .with_sanity_check(SenderUos {
                        max_uos_per_unstaked_sender: MAX_UOS_PER_UNSTAKED_SENDER,
                        gas_increase_perc: GAS_INCREASE_PERC.into(),
                    })
                    .with_simulation_check(Signature)
                    .with_simulation_check(Timestamp);

            if uo_pool_mode != UoPoolMode::Unsafe {
                validator = validator
                    .with_simulation_trace_check(Gas)
                    .with_simulation_trace_check(Opcodes)
                    .with_simulation_trace_check(ExternalContracts)
                    .with_simulation_trace_check(StorageAccess)
                    .with_simulation_trace_check(CallStack)
                    .with_simulation_trace_check(CodeHashes);
            }

            m_map.insert(
                id,
                UserOperationPool::<Provider<Http>, StandardUserOperationValidator<Provider<Http>>>::new(
                    entry_point,
                    validator,
                    Box::<MemoryMempool>::default(),
                    reputation,
                    eth_client.clone(),
                    max_verification_gas,
                    chain,
                ),
            );
        }

        let svc = uo_pool_server::UoPoolServer::new(UoPoolService::new(m_map.clone(), chain));

        tokio::spawn(async move {
            loop {
                m_map.iter_mut().for_each(|mut m| {
                    let _ = m
                        .value_mut()
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
