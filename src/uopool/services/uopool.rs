use crate::{
    chain::gas::Overhead,
    contracts::{EntryPoint, EntryPointErr, SimulateValidationResult},
    types::{
        reputation::{ReputationEntry, ReputationStatus},
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
        MempoolBox, MempoolId, ReputationBox,
    },
};
use async_trait::async_trait;
use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
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

impl<M: Middleware + 'static> UoPoolService<M>
where
    EntryPointErr<M>: From<<M as Middleware>::Error>,
{
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
fn get_addr(bytes: &Bytes) -> Option<Address> {
    if bytes.len() >= 20 {
        Some(Address::from_slice(&bytes[0..20]))
    } else {
        None
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
                                res.data =
                                    serde_json::to_string(&SimulationError::from(match error {
                                        EntryPointErr::JsonRpcError(err) => {
                                            SimulateValidationError::<M>::UserOperationExecution {
                                                message: err.message,
                                            }
                                        }
                                        _ => SimulateValidationError::UnknownError {
                                            error: format!("{error:?}"),
                                        },
                                    }))
                                    .map_err(|_| {
                                        tonic::Status::internal(
                                            "error estimating user operation gas",
                                        )
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
            entrypoint: Some(entry_point),
            ..
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
            for uo in uos.iter() {
                let (paymaster_status, deployer_status) = {
                    let reputation_lock = self.reputations.read();
                    match reputation_lock.get(&mempool_id) {
                        Some(reputation_entries) => {
                            let paymaster_status =
                                if let Some(paymaster) = get_addr(&uo.paymaster_and_data) {
                                    reputation_entries.get_status(&paymaster)
                                } else {
                                    ReputationStatus::OK
                                };
                            let deployer_status = if let Some(factory) = get_addr(&uo.init_code) {
                                reputation_entries.get_status(&factory)
                            } else {
                                ReputationStatus::OK
                            };
                            (paymaster_status, deployer_status)
                        }
                        None => {
                            return Err(tonic::Status::unavailable("mempool could not be found"))
                        }
                    }
                };

                match (paymaster_status, deployer_status) {
                    (ReputationStatus::BANNED, _) | (_, ReputationStatus::BANNED) => {
                        remove_user_op(uo)?;
                        continue;
                    }
                    (ReputationStatus::THROTTLED, _) => {
                        debug!("skipping throttled paymaster {} {}", uo.sender, uo.nonce);
                        continue;
                    }
                    (_, ReputationStatus::THROTTLED) => {
                        debug!("skipping throttled factory {} {}", uo.sender, uo.nonce);
                        continue;
                    }
                    _ => (),
                };

                // TODO
                // For each paymaster used in the batch, keep track of the balance while adding UserOps.
                // Ensure that it has sufficient deposit to pay for all the UserOps that use it.
                let _res = match self.validate_user_operation(uo, &entry_point).await {
                    Ok(res) => res,
                    Err(_) => {
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

    #[cfg(debug_assertions)]
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

    #[cfg(debug_assertions)]
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

    #[cfg(debug_assertions)]
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

    #[cfg(debug_assertions)]
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
