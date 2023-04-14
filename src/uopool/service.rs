use crate::{
    chain::gas::Overhead,
    contracts::{
        gen::EntryPointAPIEvents, gen::UserOperationEventFilter, EntryPointErr,
        SimulateValidationResult,
    },
    types::{
        reputation::{ReputationStatus, THROTTLED_MAX_INCLUDE},
        simulation::{SimulateValidationError, SimulationError},
        user_operation::{
            parse_from_input_data, UserOperation, UserOperationGasEstimation, UserOperationHash,
        },
    },
    uopool::{
        mempool_id,
        server::{
            types::{GetChainIdResponse, GetSupportedEntryPointsResponse},
            uopool::{
                uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, ClearResponse,
                ClearResult, EstimateUserOperationGasRequest, EstimateUserOperationGasResponse,
                EstimateUserOperationGasResult, GetAllReputationRequest, GetAllReputationResponse,
                GetAllReputationResult, GetAllRequest, GetAllResponse, GetAllResult,
                GetSortedRequest, GetSortedResponse, GetUserOperationByHashRequest,
                GetUserOperationByHashResponse, HandlePastEventRequest, RemoveRequest,
                RemoveResponse, RemoveResult, SetReputationRequest, SetReputationResponse,
                SetReputationResult,
            },
        },
        utils::get_addr,
        MempoolId,
    },
};
use async_trait::async_trait;
use dashmap::DashMap;
use ethers::{
    prelude::LogMeta,
    providers::Middleware,
    types::{Address, H256, U256, U64},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tonic::Response;
use tracing::{debug, trace, warn};

const LATEST_SCAN_DEPTH: u64 = 1000;

use super::UoPool as UserOperationPool;

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

    pub fn remove_user_operation(
        &self,
        uopool: &mut UserOperationPool<M>,
        user_operation_hash: &UserOperationHash,
    ) -> Option<()> {
        uopool.mempool.remove(user_operation_hash).ok();
        None
    }

    pub fn include_address(&self, uopool: &mut UserOperationPool<M>, addr: Address) -> Option<()> {
        uopool.reputation.increment_included(&addr);
        Some(())
    }
}

#[async_trait]
impl<M: Middleware + 'static> UoPool for UoPoolService<M>
where
    EntryPointErr: From<<M as Middleware>::Error>,
{
    async fn add(
        &self,
        request: tonic::Request<AddRequest>,
    ) -> Result<Response<AddResponse>, tonic::Status> {
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
                .map_err(|_| tonic::Status::invalid_argument("invalid user operation"))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let verification_result = {
                let uopool = self
                    .mempools
                    .get(&mempool_id)
                    .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;
                uopool.verify_user_operation(&user_operation).await
            };

            match verification_result {
                Ok(verification_result) => {
                    let mut uopool = self.mempools.get_mut(&mempool_id).ok_or_else(|| {
                        tonic::Status::invalid_argument("entry point not supported")
                    })?;

                    if let Some(user_operation_hash) =
                        verification_result.sanity_check_result.user_operation_hash
                    {
                        self.remove_user_operation(&mut uopool, &user_operation_hash)
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
                            .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                        }
                        Err(error) => {
                            res.set_result(AddResult::NotAdded);
                            res.data = serde_json::to_string(&error.to_string()).map_err(|_| {
                                tonic::Status::internal("error adding user operation")
                            })?;
                        }
                    }
                }
                Err(error) => {
                    res.set_result(AddResult::NotAdded);
                    res.data = serde_json::to_string(&error)
                        .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                }
            }

            return Ok(Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing user operation"))
    }

    async fn remove(
        &self,
        request: tonic::Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, tonic::Status> {
        let req = request.into_inner();

        if let RemoveRequest {
            hashes,
            ep: Some(entry_point),
        } = req
        {
            let entry_point = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;
            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let mut uopool = self
                .mempools
                .get_mut(&mempool_id)
                .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

            for hash in hashes {
                let hash: H256 = hash
                    .try_into()
                    .map_err(|_| tonic::Status::invalid_argument("invalid user operation hash"))?;

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

        Err(tonic::Status::invalid_argument(
            "missing user operations or entry point",
        ))
    }

    async fn get_chain_id(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetChainIdResponse>, tonic::Status> {
        Ok(tonic::Response::new(GetChainIdResponse {
            chain_id: self.chain_id.as_u64(),
        }))
    }

    async fn get_supported_entry_points(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<GetSupportedEntryPointsResponse>, tonic::Status> {
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
    ) -> Result<Response<EstimateUserOperationGasResponse>, tonic::Status> {
        let req = request.into_inner();
        let mut res = EstimateUserOperationGasResponse::default();

        if let EstimateUserOperationGasRequest {
            uo: Some(user_operation),
            ep: Some(entry_point),
        } = req
        {
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid user operation"))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

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
                                tonic::Status::internal("error estimating user operation gas")
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
                                tonic::Status::internal("error estimating user operation gas")
                            })?;
                        }
                    }
                }
                Err(error) => {
                    res.set_result(EstimateUserOperationGasResult::NotEstimated);
                    res.data =
                        serde_json::to_string(&SimulationError::from(error)).map_err(|_| {
                            tonic::Status::internal("error estimating user operation gas")
                        })?;
                }
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing user operation"))
    }

    async fn get_sorted_user_operations(
        &self,
        request: tonic::Request<GetSortedRequest>,
    ) -> Result<Response<GetSortedResponse>, tonic::Status> {
        let req = request.into_inner();
        if let GetSortedRequest {
            entry_point: Some(entry_point),
        } = req
        {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uos = {
                let uopool = self
                    .mempools
                    .get(&mempool_id)
                    .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;
                uopool.mempool.get_sorted().map_err(|e| {
                    tonic::Status::internal(format!("Get sorted uos internal error: {e:?}"))
                })?
            };

            let remove_user_op = |uo: &UserOperation| -> Result<(), tonic::Status> {
                let user_op_hash = uo.hash(&entry_point, &self.chain_id);
                let mut uopool = self
                    .mempools
                    .get_mut(&mempool_id)
                    .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;
                uopool.mempool.remove(&user_op_hash).map_err(|e| {
                    tonic::Status::unknown(format!(
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

                let paymaster_opt = get_addr(&uo.paymaster_and_data);
                let factory_opt = get_addr(&uo.init_code);
                let (paymaster_status, factory_status) = {
                    let uopool = self.mempools.get(&mempool_id).ok_or_else(|| {
                        tonic::Status::invalid_argument("entry point not supported")
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
                        tonic::Status::invalid_argument("entry point not supported")
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
                                // The result of call_gas_limit is usesally higher and less user op would be included
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
                                                    tonic::Status::invalid_argument(
                                                        "entry point not supported",
                                                    )
                                                })?;
                                            uopool
                                            .eth_provider
                                            .get_balance(paymaster, None)
                                            .await
                                            .map_err(|e| {
                                                tonic::Status::internal(
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
            return Err(tonic::Status::invalid_argument(format!(
                "invalid GetSortedRequest {req:?}"
            )));
        }
    }

    async fn handle_past_events(
        &self,
        request: tonic::Request<HandlePastEventRequest>,
    ) -> Result<Response<()>, tonic::Status> {
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
                tonic::Status::internal(format!("Getting the latest block number error: {e:?}"))
            })?;
        let last_block = std::cmp::max(
            1u64,
            latest_block
                .checked_sub(U64::from(LATEST_SCAN_DEPTH))
                .unwrap_or(U64::from(0))
                .as_u64(),
        );
        let entry_point =
            entry_point_opt.ok_or(tonic::Status::invalid_argument("entry point is missing"))?;
        let mempool_id = mempool_id(&entry_point.into(), &self.chain_id);

        let mut uopool = self
            .mempools
            .get_mut(&mempool_id)
            .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

        let events_filter = uopool.entry_point.events().from_block(last_block);
        let events = events_filter.query().await.map_err(|e| {
            tonic::Status::internal(format!("Getting event logs with error: {e:?}"))
        })?;
        for event in events {
            match event {
                EntryPointAPIEvents::UserOperationEventFilter(user_operation_event) => {
                    self.remove_user_operation(
                        &mut uopool,
                        &user_operation_event.user_op_hash.into(),
                    )
                    .unwrap_or_else(|| {
                        // This could be possible when other bundler submit the user operations
                        trace!(
                            "Unable to remove user operation {:?} from mempool {:?}",
                            user_operation_event.user_op_hash,
                            mempool_id
                        )
                    });
                    self.include_address(&mut uopool, user_operation_event.sender);
                    self.include_address(&mut uopool, user_operation_event.paymaster);
                    // TODO: include event aggregator
                }
                EntryPointAPIEvents::AccountDeployedFilter(account_deploy_event) => {
                    self.include_address(&mut uopool, account_deploy_event.factory);
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
        request: tonic::Request<GetUserOperationByHashRequest>,
    ) -> Result<Response<GetUserOperationByHashResponse>, tonic::Status> {
        let req = request.into_inner();
        let user_operation_hash: H256 = req
            .hash
            .ok_or(tonic::Status::invalid_argument(
                "User operation hash is missing",
            ))?
            .into();
        let mut event: Option<(UserOperationEventFilter, LogMeta)> = None;

        for mempool in self.mempools.iter() {
            let filter = mempool
                .value()
                .entry_point
                .entry_point_api()
                .event::<UserOperationEventFilter>()
                .topic1(user_operation_hash);
            let res: Vec<(UserOperationEventFilter, LogMeta)> =
                filter.query_with_meta().await.map_err(|e| {
                    tonic::Status::internal(format!("Getting event logs with error: {e:?}"))
                })?;
            if res.len() >= 2 {
                warn!("There are duplicate user operations with the same hash: {user_operation_hash:x?}");
            }
            // It is possible have two same user operatation in same bundle
            // see https://twitter.com/leekt216/status/1636414866662785024
            for log_meta in res.iter() {
                event = Some(log_meta.clone());
            }
        }

        match event {
            Some((event, log_meta)) => {
                if let Some((user_operation, entry_point)) = self
                    .eth_provider
                    .get_transaction(log_meta.transaction_hash)
                    .await
                    .map_err(|e| {
                        tonic::Status::internal(format!(
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
                    Err(tonic::Status::not_found("User operation not found"))
                }
            }
            None => Err(tonic::Status::not_found("User operation not found")),
        }
    }

    async fn get_all(
        &self,
        request: tonic::Request<GetAllRequest>,
    ) -> Result<Response<GetAllResponse>, tonic::Status> {
        let req = request.into_inner();
        let mut res = GetAllResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

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

        Err(tonic::Status::invalid_argument("missing entry point"))
    }

    async fn clear(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<ClearResponse>, tonic::Status> {
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
    ) -> Result<Response<GetAllReputationResponse>, tonic::Status> {
        let req = request.into_inner();
        let mut res = GetAllReputationResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let uopool = self
                .mempools
                .get(&mempool_id)
                .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

            res.result = GetAllReputationResult::GotAllReputation as i32;
            res.res = uopool
                .reputation
                .get_all()
                .iter()
                .map(|re| (*re).into())
                .collect();

            return Ok(tonic::Response::new(res));
        };

        Err(tonic::Status::invalid_argument("missing entry point"))
    }

    async fn set_reputation(
        &self,
        request: tonic::Request<SetReputationRequest>,
    ) -> Result<Response<SetReputationResponse>, tonic::Status> {
        let req = request.into_inner();
        let mut res = SetReputationResponse::default();

        if let Some(entry_point) = req.ep {
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            let mut uopool = self
                .mempools
                .get_mut(&mempool_id)
                .ok_or_else(|| tonic::Status::invalid_argument("entry point not supported"))?;

            uopool
                .reputation
                .set(req.res.iter().map(|re| re.clone().into()).collect());
            res.result = SetReputationResult::SetReputation as i32;

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing entry point"))
    }
}
