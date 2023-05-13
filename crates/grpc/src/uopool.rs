use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use aa_bundler_bundler::UoPoolError;
use aa_bundler_contracts::{
    parse_from_input_data, EntryPoint, EntryPointAPIEvents, EntryPointErr,
    SimulateValidationResult, UserOperationEventFilter,
};
use aa_bundler_primitives::{
    get_addr, parse_u256, ReputationStatus, SimulationError, UserOperation,
    UserOperationGasEstimation, BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLED_MAX_INCLUDE,
    THROTTLING_SLACK,
};
use aa_bundler_uopool::{
    canonical::simulation::SimulateValidationError, mempool_id, MemoryMempool, MemoryReputation,
    MempoolId, Overhead, Reputation, UoPool as UserOperationPool,
};
use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use dashmap::DashMap;
use ethers::{
    prelude::LogMeta,
    providers::{Http, Middleware, Provider},
    types::{Address, H256, U256, U64},
};
use tonic::Response;
use tracing::{debug, info, trace, warn};

const LATEST_SCAN_DEPTH: u64 = 1000;

use crate::proto::types::{GetChainIdResponse, GetSupportedEntryPointsResponse};
use crate::proto::uopool::*;
use crate::error::GrpcErrors;

#[derive(Clone, Copy, Debug, Parser, PartialEq)]
pub struct UoPoolServiceOpts {
    #[clap(long, default_value = "127.0.0.1:3001")]
    pub uopool_grpc_listen_address: SocketAddr,

    #[clap(long, value_parser=parse_u256, default_value = "1")]
    pub min_stake: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_unstake_delay: U256,

    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_priority_fee_per_gas: U256,
}

pub struct UoPoolService<M: Middleware> {
    pub mempools: Arc<DashMap<MempoolId, UserOperationPool<M>>>,
    pub eth_provider: Arc<M>,
    pub chain_id: U256,
}

impl<M: Middleware + 'static> UoPoolService<M> {
    pub fn new(
        mempools: Arc<DashMap<MempoolId, UserOperationPool<M>>>,
        eth_provider: Arc<M>,
        chain_id: U256,
    ) -> Self {
        Self {
            mempools,
            eth_provider,
            chain_id,
        }
    }

    pub async fn find_user_operation_event(
        &self,
        user_operation_hash: H256,
    ) -> anyhow::Result<Option<(UserOperationEventFilter, LogMeta)>> {
        let mut event: Option<(UserOperationEventFilter, LogMeta)> = None;
        for pool in self.mempools.iter() {
            if let Some(res) = pool
                .get_user_operation_event_meta(user_operation_hash)
                .await?
            {
                event = Some(res);
                break;
            }
        }
        Ok(event)
    }
}

#[async_trait]
impl<M: Middleware + 'static> uo_pool_server::UoPool for UoPoolService<M>
where
    EntryPointErr: From<<M as Middleware>::Error>,
{
    async fn add(
        &self,
        request: tonic::Request<AddRequest>,
    ) -> Result<Response<AddResponse>, GrpcErrors> {
        let req = request.into_inner();
        let mut res = AddResponse::default();

        if let AddRequest {
            uo: Some(user_operation),
            ep: Some(entry_point),
        } = req
        {
            trace!("Receive grpc request to add user operation: {user_operation:?} on entry point: {entry_point:?}");
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid user operaton".to_string()))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let verification_result = {
                let uopool = self
                    .mempools
                    .get(&mempool_id)
                    .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;
                uopool.verify_user_operation(&user_operation).await
            };

            match verification_result {
                Ok(verification_result) => {
                    let mut uopool = self.mempools.get_mut(&mempool_id).ok_or_else(|| {
                        GrpcError::InvalidArgument("entry point not supported".to_string())
                    })?;

                    if let Some(user_operation_hash) =
                        verification_result.sanity_check_result.user_operation_hash
                    {
                        uopool
                            .remove_user_operation(&user_operation_hash)
                            .unwrap_or_else(|| {
                                trace!(
                                    "Unable to remove user operation {:?} from mempool {:?}",
                                    user_operation_hash,
                                    mempool_id
                                )
                            });
                    }

                    match uopool
                        .mempool
                        .add(user_operation.clone(), &entry_point, &self.chain_id)
                    {
                        Ok(_) => {
                            // TODO: find better way to atomically store user operation and code hashes
                            match uopool.mempool.set_code_hashes(
                                &user_operation.hash(&entry_point, &self.chain_id),
                                &verification_result.simulation_result.code_hashes,
                            ) {
                                Ok(()) | Err(_) => {}
                            }

                            // TODO: update reputation

                            res.set_result(AddResult::Added);
                            res.data = serde_json::to_string(
                                &user_operation.hash(&entry_point, &self.chain_id),
                            )
                            .map_err(|_| GrpcErrors::Internal("error adding user operation".to_string()))?;
                        }
                        Err(error) => {
                            res.set_result(AddResult::NotAdded);
                            res.data = serde_json::to_string(&error.to_string()).map_err(|_| {
                                GrpcErrors::Internal("error adding user operation".to_string())
                            })?;
                        }
                    }
                }
                Err(error) => {
                    res.set_result(AddResult::NotAdded);
                    res.data = serde_json::to_string(&error)
                        .map_err(|_| GrpcErrors::Internal("error adding user operation".to_string()))?;
                }
            }

            return Ok(Response::new(res));
        }
        Err(GrpcErrors::InvalidArgument("missing user operation".to_string()))
    }

    async fn remove(
        &self,
        request: tonic::Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, GrpcErrors> {
        let req = request.into_inner();

        if let RemoveRequest {
            hashes,
            ep: Some(entry_point),
        } = req
        {
            let entry_point = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;
            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let mut uopool = self
                .mempools
                .get_mut(&mempool_id)
                .ok_or_else(|| GrpcErrors::InvalidArgument("entry point is not supported ".to_string()))?;

            for hash in hashes {
                let hash: H256 = hash
                    .try_into()
                    .map_err(|_| GrpcErrors::InvalidArgument("invalid user operation hash".to_string()))?;

                match uopool.mempool.remove(&hash.into()) {
                    Ok(_) => {}
                    Err(_) => {
                        return Ok(tonic::Response::new(RemoveResponse {
                            result: RemoveResult::NotRemoved as i32,
                        }));
                    }
                }
            }

            return Ok(tonic::Response::new(RemoveResponse {
                result: RemoveResult::Removed as i32,
            }));
        }

        Err(GrpcErrors::InvalidArgument(
            "missing user operations or entry point".to_string(),
        ))
    }

    async fn get_chain_id(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetChainIdResponse>, GrpcErrors> {
        Ok(tonic::Response::new(GetChainIdResponse {
            chain_id: self.chain_id.as_u64(),
        }))
    }

    async fn get_supported_entry_points(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, GrpcErrors> {
        Ok(tonic::Response::new(GetSupportedEntryPointsResponse {
            eps: self
                .mempools
                .iter()
                .map(|mempool| mempool.value().entry_point.address().into())
                .collect(),
        }))
    }

    async fn estimate_user_operation_gas(
        &self,
        request: tonic::Request<EstimateUserOperationGasRequest>,
    ) -> Result<Response<EstimateUserOperationGasResponse>, GrpcErrors> {
        let req = request.into_inner();
        let mut res = EstimateUserOperationGasResponse::default();

        if let EstimateUserOperationGasRequest {
            uo: Some(user_operation),
            ep: Some(entry_point),
        } = req
        {
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid user operation".to_string()))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;

            match uopool.simulate_user_operation(&user_operation).await {
                Ok(simulation_result) => {
                    let pre_verification_gas =
                        Overhead::default().calculate_pre_verification_gas(&user_operation);

                    let verification_gas_limit = match simulation_result.simulate_validation_result
                    {
                        SimulateValidationResult::ValidationResult(validation_result) => {
                            validation_result.return_info.0
                        }
                        SimulateValidationResult::ValidationResultWithAggregation(
                            validation_result_with_aggregation,
                        ) => validation_result_with_aggregation.return_info.0,
                    };

                    match uopool
                        .entry_point
                        .estimate_call_gas(user_operation.clone())
                        .await
                    {
                        Ok(call_gas_limit) => {
                            res.set_result(EstimateUserOperationGasResult::Estimated);
                            res.data = serde_json::to_string(&UserOperationGasEstimation {
                                pre_verification_gas,
                                verification_gas_limit,
                                call_gas_limit,
                            })
                            .map_err(|_| {
                                GrpcErrors::Internal("error estimating user operation gas".to_string())
                            })?;
                        }
                        Err(error) => {
                            res.set_result(EstimateUserOperationGasResult::NotEstimated);
                            res.data = serde_json::to_string(&SimulationError::from(match error {
                                EntryPointErr::JsonRpcError(err) => {
                                    SimulateValidationError::UserOperationExecution {
                                        message: err.message,
                                    }
                                }
                                _ => SimulateValidationError::UnknownError {
                                    error: format!("{error:?}"),
                                },
                            }))
                            .map_err(|_| {
                                GrpcErrors::Internal("error estimating user operation gas".to_string())
                            })?;
                        }
                    }
                }
                Err(error) => {
                    res.set_result(EstimateUserOperationGasResult::NotEstimated);
                    res.data =
                        serde_json::to_string(&SimulationError::from(error)).map_err(|_| {
                            GrpcErrors::Internal("error estimating user operation gas".to_string())
                        })?;
                }
            }

            return Ok(tonic::Response::new(res));
        }

        Err(GrpcErrors::InvalidArgument("missing user operation".to_string()))
    }

    async fn get_sorted_user_operations(
        &self,
        request: tonic::Request<GetSortedRequest>,
    ) -> Result<Response<GetSortedResponse>, GrpcErrors> {
        let req = request.into_inner();
        if let GetSortedRequest {
            entry_point: Some(entry_point),
        } = req
        {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uos = {
                let uopool = self
                    .mempools
                    .get(&mempool_id)
                    .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;
                uopool.mempool.get_sorted().map_err(|e| {
                    GrpcErrors::Internal(format!("Get sorted uos internal error: {e:?}"))
                })?
            };

            let remove_user_op = |uo: &UserOperation| -> Result<(), GrpcErrors> {
                let user_op_hash = uo.hash(&entry_point, &self.chain_id);
                let mut uopool = self
                    .mempools
                    .get_mut(&mempool_id)
                    .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;
                uopool.mempool.remove(&user_op_hash).map_err(|e| {
                    GrpcErrors::Unknown(format!(
                        "remove a banned user operation {user_op_hash:x?} failed with {e:?}."
                    ))
                })?;
                Ok(())
            };

            let mut valid_user_operations = vec![];
            let mut senders: HashSet<Address> = HashSet::new();
            let mut total_gas = U256::zero();
            let mut paymaster_deposit: HashMap<Address, U256> = HashMap::new();
            let mut staked_entity_count: HashMap<Address, u64> = HashMap::new();
            for uo in uos.iter() {
                if senders.contains(&uo.sender) {
                    continue;
                }

                let paymaster_opt = get_addr(uo.paymaster_and_data.0.as_ref());
                let factory_opt = get_addr(uo.init_code.0.as_ref());
                let (paymaster_status, factory_status) = {
                    let uopool = self.mempools.get(&mempool_id).ok_or_else(|| {
                        GrpcErrors::InvalidArgument("entry point not supported".to_string())
                    })?;

                    let paymaster_status = uopool
                        .reputation
                        .get_status_from_bytes(&uo.paymaster_and_data);
                    let deployer_status = uopool.reputation.get_status_from_bytes(&uo.init_code);
                    (paymaster_status, deployer_status)
                };
                let paymaster_count = paymaster_opt
                    .map(|p| staked_entity_count.get(&p).cloned().unwrap_or(0))
                    .unwrap_or(0);
                let factory_count = factory_opt
                    .map(|p| staked_entity_count.get(&p).cloned().unwrap_or(0))
                    .unwrap_or(0);

                match (paymaster_status, factory_status) {
                    (ReputationStatus::BANNED, _) | (_, ReputationStatus::BANNED) => {
                        remove_user_op(uo)?;
                        continue;
                    }
                    (ReputationStatus::THROTTLED, _) if paymaster_count > THROTTLED_MAX_INCLUDE => {
                        debug!("skipping throttled paymaster {} {}", uo.sender, uo.nonce);
                        continue;
                    }
                    (_, ReputationStatus::THROTTLED) if factory_count > THROTTLED_MAX_INCLUDE => {
                        debug!("skipping throttled factory {} {}", uo.sender, uo.nonce);
                        continue;
                    }
                    _ => (),
                };

                let (simulation_result, max_verification_gas) = {
                    let uopool = self.mempools.get(&mempool_id).ok_or_else(|| {
                        GrpcErrors::InvalidArgument("entry point not supported".to_string())
                    })?;
                    (
                        uopool.simulate_user_operation(uo).await,
                        uopool.max_verification_gas,
                    )
                };

                match simulation_result {
                    Ok(simulation_result) => {
                        match simulation_result.simulate_validation_result {
                            SimulateValidationResult::ValidationResult(res) => {
                                // TODO
                                // it would be better to use estimate_gas instead of call_gas_limit
                                // The result of call_gas_limit is usually higher and less user op would be included
                                let user_op_gas_cost =
                                    res.return_info.0.saturating_add(uo.call_gas_limit);
                                let new_total_gas = total_gas.saturating_add(user_op_gas_cost);
                                if new_total_gas.gt(&max_verification_gas) {
                                    break;
                                }
                                if let Some(paymaster) = paymaster_opt {
                                    let balance = match paymaster_deposit.get(&paymaster) {
                                        Some(n) => Ok(n.to_owned()),
                                        None => {
                                            let uopool = self
                                                .mempools
                                                .get(&mempool_id)
                                                .ok_or_else(|| {
                                                    GrpcErrors::InvalidArgument(
                                                        "entry point not supported".to_string(),
                                                    )
                                                })?;
                                            uopool
                                            .eth_provider
                                            .get_balance(paymaster, None)
                                            .await
                                            .map_err(|e| {
                                                GrpcErrors::Internal(
                                                    format!("Could not get paymaster {paymaster:?} balance because of {e:?}")
                                                )
                                            })
                                        }
                                    }?;

                                    if balance.lt(&res.return_info.1) {
                                        continue;
                                    }

                                    let update_balance = balance.saturating_sub(res.return_info.1);
                                    staked_entity_count
                                        .entry(paymaster)
                                        .and_modify(|c| *c += 1)
                                        .or_insert(1);
                                    paymaster_deposit.insert(paymaster, update_balance);
                                };
                                if let Some(factory) = factory_opt {
                                    staked_entity_count
                                        .entry(factory)
                                        .and_modify(|c| *c += 1)
                                        .or_insert(1);
                                };
                                total_gas = new_total_gas
                            }
                            SimulateValidationResult::ValidationResultWithAggregation(_) => {
                                todo!("Aggregation is not supported now.")
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed in 2nd simulation: {e:?} ");
                        remove_user_op(uo)?;
                        continue;
                    }
                }

                valid_user_operations.push(uo.to_owned());
                senders.insert(uo.sender);
            }

            let response = GetSortedResponse {
                user_operations: valid_user_operations
                    .into_iter()
                    .map(|u| u.into())
                    .collect(),
            };
            return Ok(tonic::Response::new(response));
        } else {
            return Err(GrpcErrors::InvalidArgument(format!(
                "invalid GetSortedRequest {req:?}"
            )));
        }
    }

    async fn handle_past_events(
        &self,
        request: tonic::Request<HandlePastEventRequest>,
    ) -> Result<Response<()>, GrpcErrors> {
        let req = request.into_inner();
        let HandlePastEventRequest {
            entry_point: entry_point_opt,
        } = req;
        let latest_block = self
            .eth_provider
            .clone()
            .get_block_number()
            .await
            .map_err(|e| {
                GrpcErrors::Internal(format!("Getting the latest block number error: {e:?}"))
            })?;
        let last_block = std::cmp::max(
            1u64,
            latest_block
                .checked_sub(U64::from(LATEST_SCAN_DEPTH))
                .unwrap_or(U64::from(0))
                .as_u64(),
        );
        let entry_point =
            entry_point_opt.ok_or(GrpcErrors::InvalidArgument("entry point is missing".to_string()))?;
        let mempool_id = mempool_id(&entry_point.into(), &self.chain_id);

        let mut uopool = self
            .mempools
            .get_mut(&mempool_id)
            .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;

        let events_filter = uopool.entry_point.events().from_block(last_block);
        let events = events_filter.query().await.map_err(|e| {
            GrpcErrors::Internal(format!("Getting event logs with error: {e:?}"))
        })?;
        for event in events {
            match event {
                EntryPointAPIEvents::UserOperationEventFilter(user_operation_event) => {
                    uopool
                        .remove_user_operation(&user_operation_event.user_op_hash.into())
                        .unwrap_or_else(|| {
                            // This could be possible when other bundler submit the user operations
                            trace!(
                                "Unable to remove user operation {:?} from mempool {:?}",
                                user_operation_event.user_op_hash,
                                mempool_id
                            )
                        });
                    uopool.include_address(user_operation_event.sender);
                    uopool.include_address(user_operation_event.paymaster);
                    // TODO: include event aggregator
                }
                EntryPointAPIEvents::AccountDeployedFilter(account_deploy_event) => {
                    uopool.include_address(account_deploy_event.factory);
                }
                EntryPointAPIEvents::SignatureAggregatorChangedFilter(_) => {
                    warn!("Aggregate signature is not supported currently");
                }
                _ => (),
            }
        }

        Ok(Response::new(()))
    }

    async fn get_user_operation_by_hash(
        &self,
        request: tonic::Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationByHashResponse>, GrpcErrors> {
        let req = request.into_inner();
        let user_operation_hash: H256 = req
            .hash
            .ok_or(GrpcErrors::InvalidArgument(
                "User operation hash is missing".to_string(),
            ))?
            .into();
        let event: Option<(UserOperationEventFilter, LogMeta)> = self
            .find_user_operation_event(user_operation_hash)
            .await
            .map_err(|e| {
                GrpcErrors::Internal(format!(
                    "Getting user operation event meta with error: {e:?}"
                ))
            })?;

        match event {
            Some((event, log_meta)) => {
                if let Some((user_operation, entry_point)) = self
                    .eth_provider
                    .get_transaction(log_meta.transaction_hash)
                    .await
                    .map_err(|e| {
                        GrpcErrors::Internal(format!(
                            "Getting transaction by hash with error: {e:?}"
                        ))
                    })?
                    .and_then(|tx| {
                        let user_ops = parse_from_input_data(tx.input)?;
                        let entry_point = tx.to?;
                        user_ops
                            .iter()
                            .find(|uo| uo.sender == event.sender && uo.nonce == event.nonce)
                            .map(|uo| (uo.clone(), entry_point))
                    })
                {
                    let response = Response::new(GetUserOperationByHashResponse {
                        user_operation: Some(user_operation.into()),
                        entry_point: Some(entry_point.into()),
                        transaction_hash: Some(log_meta.transaction_hash.into()),
                        block_hash: Some(log_meta.block_hash.into()),
                        block_number: log_meta.block_number.as_u64(),
                    });
                    Ok(response)
                } else {
                    Err(GrpcErrors::NotFound("User operation not found".to_string()))
                }
            }
            None => Err(GrpcErrors::NotFound("User operation not found".to_string())),
        }
    }

    async fn get_user_operation_receipt(
        &self,
        request: tonic::Request<UserOperationHashRequest>,
    ) -> Result<Response<GetUserOperationReceiptResponse>, GrpcErrors> {
        let req = request.into_inner();
        let user_operation_hash: H256 = req
            .clone()
            .hash
            .ok_or(GrpcErrors::InvalidArgument(
                "User operation hash is missing".to_string(),
            ))?
            .into();

        let event = self
            .find_user_operation_event(user_operation_hash)
            .await
            .map_err(|e| {
                GrpcErrors::Internal(format!(
                    "Getting user operation event meta with error: {e:?}"
                ))
            })?;

        match event {
            Some((event, log_meta)) => {
                if let Some(transaction_receipt) = self
                    .eth_provider
                    .get_transaction_receipt(log_meta.transaction_hash)
                    .await
                    .map_err(|e| {
                        GrpcErrors::Internal(format!(
                            "Getting transaction by hash with error: {e:?}"
                        ))
                    })?
                {
                    let user_operation = self
                        .get_user_operation_by_hash(tonic::Request::new(req))
                        .await?;

                    let response = Response::new(GetUserOperationReceiptResponse {
                        user_operation_hash: Some(user_operation_hash.into()),
                        sender: Some(event.sender.into()),
                        nonce: Some(event.nonce.into()),
                        actual_gas_cost: Some(event.actual_gas_cost.into()),
                        actual_gas_used: Some(event.actual_gas_used.into()),
                        success: event.success,
                        transaction_receipt: Some(transaction_receipt.clone().into()),
                        logs: transaction_receipt
                            .logs
                            .into_iter()
                            .map(|l| l.into())
                            .collect(),
                        paymaster: user_operation
                            .into_inner()
                            .user_operation
                            .and_then(|u| get_addr(u.paymaster_and_data.as_ref()))
                            .map(|p| p.into()),
                    });
                    Ok(response)
                } else {
                    Err(GrpcErrors::NotFound("User operation not found".to_string()))
                }
            }
            None => Err(GrpcErrors::NotFound("User operation not found".to_string())),
        }
    }

    async fn get_all(
        &self,
        request: tonic::Request<GetAllRequest>,
    ) -> Result<Response<GetAllResponse>, GrpcErrors> {
        let req = request.into_inner();
        let mut res = GetAllResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;

            res.result = GetAllResult::GotAll as i32;
            res.uos = uopool
                .mempool
                .get_all()
                .iter()
                .map(|uo| uo.clone().into())
                .collect();
            trace!("Get all user operations in the mempool: {:?}", res.uos);

            return Ok(tonic::Response::new(res));
        }

        Err(GrpcErrors::InvalidArgument("missing entry point".to_string()))
    }

    async fn clear(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<ClearResponse>, GrpcErrors> {
        self.mempools.iter_mut().for_each(|mut mempool| {
            let mempool = mempool.value_mut();
            mempool.mempool.clear();
            mempool.reputation.clear()
        });

        Ok(tonic::Response::new(ClearResponse {
            result: ClearResult::Cleared as i32,
        }))
    }

    async fn get_all_reputation(
        &self,
        request: tonic::Request<GetAllReputationRequest>,
    ) -> Result<Response<GetAllReputationResponse>, GrpcErrors> {
        let req = request.into_inner();
        let mut res = GetAllReputationResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;

            res.result = GetAllReputationResult::GotAllReputation as i32;
            res.res = uopool
                .reputation
                .get_all()
                .iter()
                .map(|re| (*re).into())
                .collect();

            return Ok(tonic::Response::new(res));
        };

        Err(GrpcErrors::InvalidArgument("missing entry point".to_string()))
    }

    async fn set_reputation(
        &self,
        request: tonic::Request<SetReputationRequest>,
    ) -> Result<Response<SetReputationResponse>, GrpcErrors> {
        let req = request.into_inner();
        let mut res = SetReputationResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| GrpcErrors::InvalidArgument("invalid entry point".to_string()))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let mut uopool = self
                .mempools
                .get_mut(&mempool_id)
                .ok_or_else(|| GrpcErrors::InvalidArgument("entry point not supported".to_string()))?;

            uopool
                .reputation
                .set(req.res.iter().map(|re| re.clone().into()).collect());
            res.result = SetReputationResult::SetReputation as i32;

            return Ok(tonic::Response::new(res));
        }

        Err(GrpcErrors::InvalidArgument("missing entry point".to_string()))
    }
}

pub async fn uopool_service_run(
    opts: UoPoolServiceOpts,
    entry_points: Vec<Address>,
    eth_provider: Arc<Provider<Http>>,
    max_verification_gas: U256,
) -> Result<()> {
    let chain_id = eth_provider.get_chainid().await?;

    tokio::spawn(async move {
        let mut builder = tonic::transport::Server::builder();

        let mempools_map = Arc::new(DashMap::<MempoolId, UserOperationPool<Provider<Http>>>::new());

        for entry_point in entry_points {
            let id = mempool_id(&entry_point, &chain_id);

            let mut reputation = Box::<MemoryReputation>::default();
            reputation.init(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                opts.min_stake,
                opts.min_unstake_delay,
            );

            mempools_map.insert(
                id,
                UserOperationPool::<Provider<Http>>::new(
                    EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
                    Box::<MemoryMempool>::default(),
                    reputation,
                    eth_provider.clone(),
                    max_verification_gas,
                    opts.min_priority_fee_per_gas,
                    chain_id,
                ),
            );
        }

        let svc = uo_pool_server::UoPoolServer::new(UoPoolService::new(
            mempools_map.clone(),
            eth_provider.clone(),
            chain_id,
        ));

        tokio::spawn(async move {
            loop {
                mempools_map
                    .iter_mut()
                    .for_each(|mut mempool| mempool.value_mut().reputation.update_hourly());
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });

        info!(
            "UoPool gRPC server starting on {}",
            opts.uopool_grpc_listen_address
        );

        builder
            .add_service(svc)
            .serve(opts.uopool_grpc_listen_address)
            .await
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
