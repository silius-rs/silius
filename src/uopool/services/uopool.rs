use crate::{
    chain::gas::Overhead,
    contracts::{EntryPoint, EntryPointErr, SimulateValidationResult},
    types::{
        reputation::{ReputationEntry, ReputationStatus, THROTTLED_MAX_INCLUDE},
        simulation::{SimulateValidationError, SimulationError},
        user_operation::{UserOperation, UserOperationGasEstimation},
    },
    uopool::{
        mempool_id,
        server::uopool::{
            uo_pool_server::UoPool, AddRequest, AddResponse, AddResult, ClearResponse, ClearResult,
            EstimateUserOperationGasRequest, EstimateUserOperationGasResponse,
            EstimateUserOperationGasResult, GetAllReputationRequest, GetAllReputationResponse,
            GetAllReputationResult, GetAllRequest, GetAllResponse, GetAllResult,
            GetChainIdResponse, GetSortedRequest, GetSortedResponse,
            GetSupportedEntryPointsResponse, RemoveRequest, RemoveResponse, SetReputationRequest,
            SetReputationResponse, SetReputationResult,
        },
        utils::get_addr,
        MempoolBox, MempoolId, ReputationBox,
    },
};
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use jsonrpsee::types::ErrorObject;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tonic::Response;
use tracing::debug;

pub type UoPoolError = ErrorObject<'static>;

pub struct UoPoolService<M: Middleware> {
    pub entry_points: Arc<HashMap<MempoolId, EntryPoint<M>>>,
    pub mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
    pub reputations: Arc<RwLock<HashMap<MempoolId, ReputationBox<Vec<ReputationEntry>>>>>,
    pub eth_provider: Arc<M>,
    pub max_verification_gas: U256,
    pub min_priority_fee_per_gas: U256,
    pub chain_id: U256,
}

impl<M: Middleware + 'static> UoPoolService<M> {
    pub fn new(
        entry_points: Arc<HashMap<MempoolId, EntryPoint<M>>>,
        mempools: Arc<RwLock<HashMap<MempoolId, MempoolBox<Vec<UserOperation>>>>>,
        reputations: Arc<RwLock<HashMap<MempoolId, ReputationBox<Vec<ReputationEntry>>>>>,
        eth_provider: Arc<M>,
        max_verification_gas: U256,
        min_priority_fee_per_gas: U256,
        chain_id: U256,
    ) -> Self {
        Self {
            entry_points,
            mempools,
            reputations,
            eth_provider,
            max_verification_gas,
            min_priority_fee_per_gas,
            chain_id,
        }
    }

    async fn verify_user_operation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<(), ErrorObject<'static>> {
        // sanity check
        self.validate_user_operation(user_operation, entry_point)
            .await?;

        // simulation
        self.simulate_user_operation(user_operation, entry_point)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl<M: Middleware + 'static> UoPool for UoPoolService<M>
where
    EntryPointErr<M>: From<<M as Middleware>::Error>,
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
            let user_operation: UserOperation = user_operation
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid user operation"))?;
            let entry_point: Address = entry_point
                .try_into()
                .map_err(|_| tonic::Status::invalid_argument("invalid entry point"))?;

            let mempool_id = mempool_id(&entry_point, &self.chain_id);

            if !self.entry_points.contains_key(&mempool_id) {
                return Err(tonic::Status::invalid_argument("entry point not supported"));
            }

            match self
                .verify_user_operation(&user_operation, &entry_point)
                .await
            {
                Ok(_) => {
                    // TODO: update reputation
                    // TODO: add to mempool

                    res.set_result(AddResult::Added);
                    res.data =
                        serde_json::to_string(&user_operation.hash(&entry_point, &self.chain_id))
                            .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                }
                Err(error) => {
                    res.set_result(AddResult::NotAdded);
                    res.data = serde_json::to_string(&error)
                        .map_err(|_| tonic::Status::internal("error adding user operation"))?;
                }
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing user operation"))
    }

    async fn remove(
        &self,
        _request: tonic::Request<RemoveRequest>,
    ) -> Result<Response<RemoveResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("todo"))
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
                .entry_points
                .values()
                .map(|entry_point| entry_point.address().into())
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

            if let Some(entry_point) = self.entry_points.get(&mempool_id) {
                match self
                    .simulate_user_operation(&user_operation, &entry_point.address())
                    .await
                {
                    Ok(simulate_validation_result) => {
                        let pre_verification_gas =
                            Overhead::default().calculate_pre_verification_gas(&user_operation);

                        let verification_gas_limit = match simulate_validation_result {
                            SimulateValidationResult::ValidationResult(validation_result) => {
                                validation_result.return_info.0
                            }
                            SimulateValidationResult::ValidationResultWithAggregation(
                                validation_result_with_aggregation,
                            ) => validation_result_with_aggregation.return_info.0,
                        };

                        match entry_point.estimate_call_gas(user_operation.clone()).await {
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
                                res.data = serde_json::to_string(&SimulationError::from(
                                    SimulateValidationError::<M>::UnknownError {
                                        error: format!("{error:?}"),
                                    },
                                ))
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
            } else {
                return Err(tonic::Status::invalid_argument("entry point not supported"));
            }
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
            if !self.entry_points.contains_key(&mempool_id) {
                return Err(tonic::Status::invalid_argument("entry point not supported"));
            }
            let mempool = self.mempools.clone();
            let uos = {
                let lock = mempool.read();
                if let Some(pool) = lock.get(&mempool_id) {
                    pool.get_sorted().map_err(|e| {
                        tonic::Status::internal(format!("Get Sored internal error: {e:?}"))
                    })
                } else {
                    return Err(tonic::Status::unavailable("mempool could not be found"));
                }
            }?;

            let remove_user_op = |uo: &UserOperation| -> Result<(), tonic::Status> {
                let mut memory_pool = self.mempools.write();
                if let Some(pool) = memory_pool.get_mut(&mempool_id) {
                    let user_op_hash = uo.hash(&entry_point, &self.chain_id);
                    pool.remove(&user_op_hash).map_err(|e| {
                        tonic::Status::unknown(format!(
                            "remove a banned user operation {user_op_hash:x?} failed with {e:?}."
                        ))
                    })?;
                }
                Ok(())
            };

            let mut valid_user_operations = vec![];
            let mut total_gas = U256::zero();
            let mut paymaster_deposit: HashMap<Address, U256> = HashMap::new();
            let mut staked_entity_count: HashMap<Address, u64> = HashMap::new();
            for uo in uos.iter() {
                let paymaster_opt = get_addr(&uo.paymaster_and_data);
                let factory_opt = get_addr(&uo.init_code);
                let (paymaster_status, factory_status) = {
                    let reputation_lock = self.reputations.read();
                    match reputation_lock.get(&mempool_id) {
                        Some(reputation_entries) => {
                            let paymaster_status =
                                reputation_entries.get_status_from_bytes(&uo.paymaster_and_data);
                            let deployer_status =
                                reputation_entries.get_status_from_bytes(&uo.init_code);
                            (paymaster_status, deployer_status)
                        }
                        None => {
                            return Err(tonic::Status::unavailable("mempool could not be found"))
                        }
                    }
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

                match self.simulate_user_operation(uo, &entry_point).await {
                    Ok(res) => match res {
                        SimulateValidationResult::ValidationResult(res) => {
                            // TODO
                            // it would be better to use estimate_gas instead of call_gas_limit
                            // The result of call_gas_limit is usesally higher and less user op would be included
                            let user_op_gas_cost =
                                res.return_info.0.saturating_add(uo.call_gas_limit);
                            let new_total_gas = total_gas.saturating_add(user_op_gas_cost);
                            if new_total_gas.gt(&self.max_verification_gas) {
                                break;
                            }
                            if let Some(paymaster) = paymaster_opt {
                                let balance = match paymaster_deposit.get(&paymaster) {
                                    Some(n) => Ok(n.to_owned()),
                                    None => self
                                        .eth_provider
                                        .get_balance(paymaster, None)
                                        .await
                                        .map_err(|e| {
                                            tonic::Status::internal(
                                                format!("Could not get paymaster {paymaster:?} balance because of {e:?}")
                                            )
                                        }),
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
                    },
                    Err(e) => {
                        debug!("Failed in 2nd validation: {e:?} ");
                        remove_user_op(uo)?;
                        continue;
                    }
                };

                valid_user_operations.push(uo.to_owned())
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

            if let Some(mempool) = self
                .mempools
                .read()
                .get(&mempool_id(&entry_point, &self.chain_id))
            {
                res.result = GetAllResult::GotAll as i32;
                res.uos = mempool
                    .get_all()
                    .iter()
                    .map(|uo| uo.clone().into())
                    .collect();
            } else {
                res.result = GetAllResult::NotGotAll as i32;
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing entry point"))
    }

    async fn clear(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<Response<ClearResponse>, tonic::Status> {
        for mempool in self.mempools.write().values_mut() {
            mempool.clear();
        }

        for reputation in self.reputations.write().values_mut() {
            reputation.clear();
        }

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

            if let Some(reputation) = self
                .reputations
                .read()
                .get(&mempool_id(&entry_point, &self.chain_id))
            {
                res.result = GetAllReputationResult::GotAllReputation as i32;
                res.res = reputation.get_all().iter().map(|re| (*re).into()).collect();
            } else {
                res.result = GetAllReputationResult::NotGotAllReputation as i32;
            }

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

            if let Some(reputation) = self
                .reputations
                .write()
                .get_mut(&mempool_id(&entry_point, &self.chain_id))
            {
                reputation.set(req.res.iter().map(|re| re.clone().into()).collect());
                res.result = SetReputationResult::SetReputation as i32;
            } else {
                res.result = SetReputationResult::NotSetReputation as i32;
            }

            return Ok(tonic::Response::new(res));
        }

        Err(tonic::Status::invalid_argument("missing entry point"))
    }
}
