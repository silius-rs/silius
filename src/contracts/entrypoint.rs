use std::fmt::Display;
use std::sync::Arc;

use super::gen::entry_point_api::{
    EntryPointAPIErrors, FailedOp, SenderAddressResult, UserOperation, ValidationResult,
    ValidationResultWithAggregation,
};
use super::gen::stake_manager_api::DepositInfo;
use super::gen::{EntryPointAPI, StakeManagerAPI};
use super::tracer::JS_TRACER;
use ethers::abi::AbiDecode;
use ethers::prelude::ContractError;
use ethers::providers::{Middleware, ProviderError};
use ethers::types::{
    Address, Bytes, GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
    GethTrace, TransactionRequest, U256,
};
use ethers_providers::{JsonRpcError, MiddlewareError};
use thiserror::Error;
use tracing::trace;

pub struct EntryPoint<M: Middleware> {
    provider: Arc<M>,
    address: Address,
    entry_point_api: EntryPointAPI<M>,
    stake_manager_api: StakeManagerAPI<M>,
}

impl<M: Middleware + 'static> EntryPoint<M> {
    pub fn new(provider: Arc<M>, address: Address) -> Self {
        let entry_point_api = EntryPointAPI::new(address, provider.clone());
        let stake_manager_api = StakeManagerAPI::new(address, provider.clone());
        Self {
            provider,
            address,
            entry_point_api,
            stake_manager_api,
        }
    }

    pub fn provider(&self) -> Arc<M> {
        self.provider.clone()
    }

    pub fn address(&self) -> Address {
        self.address
    }

    fn deserialize_error_msg(
        err_msg: ContractError<M>,
    ) -> Result<EntryPointAPIErrors, EntryPointErr> {
        match err_msg {
            ContractError::DecodingError(e) => Err(EntryPointErr::DecodeErr(format!(
                "Decoding error on msg: {e:?}"
            ))),
            ContractError::AbiError(e) => Err(EntryPointErr::UnknownErr(format!(
                "Contract call with abi error: {e:?} ",
            ))),
            ContractError::MiddlewareError { e } => Err(EntryPointErr::from_middleware_err::<M>(e)),
            ContractError::ProviderError { e } => Err(e.into()),
            ContractError::Revert(data) => AbiDecode::decode(data).map_err(|e| {
                EntryPointErr::DecodeErr(format!(
                    "{e:?} data field could not be deserialize to EntryPointAPIErrors",
                ))
            }),
            _ => Err(EntryPointErr::UnknownErr(format!(
                "Unkown error: {err_msg:?}",
            ))),
        }
    }

    pub async fn simulate_validation<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<SimulateValidationResult, EntryPointErr> {
        let request_result = self
            .entry_point_api
            .simulate_validation(user_operation.into())
            .await;
        match request_result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Simulate validation should expect revert".to_string(),
            )),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::FailedOp(failed_op) => Err(EntryPointErr::FailedOp(failed_op)),
                EntryPointAPIErrors::ValidationResult(res) => {
                    Ok(SimulateValidationResult::ValidationResult(res))
                }
                EntryPointAPIErrors::ValidationResultWithAggregation(res) => Ok(
                    SimulateValidationResult::ValidationResultWithAggregation(res),
                ),
                _ => Err(EntryPointErr::UnknownErr(format!(
                    "Simulate validation with invalid error: {op:?}"
                ))),
            }),
        }
    }

    pub async fn simulate_validation_trace<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<GethTrace, EntryPointErr> {
        let call = self
            .entry_point_api
            .simulate_validation(user_operation.into());
        let request_result = self
            .provider
            .debug_trace_call(
                call.tx,
                None,
                GethDebugTracingCallOptions {
                    tracing_options: GethDebugTracingOptions {
                        disable_storage: None,
                        disable_stack: None,
                        enable_memory: None,
                        enable_return_data: None,
                        tracer: Some(GethDebugTracerType::JsTracer(JS_TRACER.to_string())),
                        tracer_config: None,
                        timeout: None,
                    },
                },
            )
            .await
            .map_err(|e| EntryPointErr::from_middleware_err::<M>(e))?;
        Ok(request_result)
    }

    pub async fn handle_ops<U: Into<UserOperation>>(
        &self,
        ops: Vec<U>,
        beneficiary: Address,
    ) -> Result<(), EntryPointErr> {
        self.entry_point_api
            .handle_ops(ops.into_iter().map(|u| u.into()).collect(), beneficiary)
            .call()
            .await
            .or_else(|e| {
                Self::deserialize_error_msg(e).and_then(|op| match op {
                    EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Handle ops with invalid error: {op:?}"
                    ))),
                })
            })
    }

    pub async fn get_deposit_info(&self, address: &Address) -> Result<DepositInfo, EntryPointErr> {
        let result = self
            .stake_manager_api
            .get_deposit_info(*address)
            .call()
            .await;

        match result {
            Ok(deposit_info) => Ok(deposit_info),
            _ => Err(EntryPointErr::UnknownErr(
                "Error calling get deposit info".to_string(),
            )),
        }
    }

    pub async fn get_sender_address(
        &self,
        initcode: Bytes,
    ) -> Result<SenderAddressResult, EntryPointErr> {
        let result = self
            .entry_point_api
            .get_sender_address(initcode)
            .call()
            .await;

        match result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Get sender address should expect revert".to_string(),
            )),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::SenderAddressResult(res) => Ok(res),
                EntryPointAPIErrors::FailedOp(failed_op) => Err(EntryPointErr::FailedOp(failed_op)),
                _ => Err(EntryPointErr::UnknownErr(format!(
                    "Simulate validation with invalid error: {op:?}"
                ))),
            }),
        }
    }

    pub async fn estimate_call_gas<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<U256, EntryPointErr> {
        let user_operation = user_operation.into();

        if user_operation.call_data.is_empty() {
            Ok(U256::zero())
        } else {
            let result = self
                .provider
                .estimate_gas(
                    &TransactionRequest::new()
                        .from(self.address)
                        .to(user_operation.sender)
                        .data(user_operation.call_data.clone())
                        .into(),
                    None,
                )
                .await;
            trace!("Estimate call gas on {user_operation:?} returned {result:?}");
            match result {
                Ok(gas) => Ok(gas),
                Err(e) => Err(EntryPointErr::from_middleware_err::<M>(e)),
            }
        }
    }

    pub async fn handle_aggregated_ops<U: Into<UserOperation>>(
        &self,
        _ops_per_aggregator: Vec<U>,
        _beneficiary: Address,
    ) -> Result<(), EntryPointErr> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum EntryPointErr {
    FailedOp(FailedOp),
    JsonRpcError(JsonRpcError),
    NetworkErr(String),
    DecodeErr(String),
    UnknownErr(String), // describe impossible error. We should fix the codes here(or contract codes) if this occurs.
}

impl Display for EntryPointErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<ProviderError> for EntryPointErr {
    fn from(e: ProviderError) -> Self {
        Self::from_provider_err(&e)
    }
}

impl EntryPointErr {
    fn from_provider_err(e: &ProviderError) -> Self {
        match e {
            ProviderError::JsonRpcClientError(err) => err
                .as_error_response()
                .map(|e| EntryPointErr::JsonRpcError(e.clone()))
                .unwrap_or(EntryPointErr::UnknownErr(format!(
                    "Unknown json rpc client error: {err:?}"
                ))),
            ProviderError::HTTPError(err) => {
                EntryPointErr::NetworkErr(format!("Entrypoint HTTP error: {err:?}"))
            }
            _ => EntryPointErr::UnknownErr(format!("Unknown error in provider: {e:?}")),
        }
    }

    fn from_middleware_err<M: Middleware>(value: M::Error) -> Self {
        if let Some(json_err) = value.as_error_response() {
            return EntryPointErr::JsonRpcError(json_err.clone());
        }

        if let Some(provider_err) = value.as_provider_error() {
            return EntryPointErr::from_provider_err(provider_err);
        }

        EntryPointErr::UnknownErr(format!("Unknown middlerware error: {value:?}"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    ValidationResult(ValidationResult),
    ValidationResultWithAggregation(ValidationResultWithAggregation),
}

#[cfg(test)]
mod tests {
    use ethers::{
        providers::{Http, Middleware, Provider},
        types::{Address, Bytes, GethTrace, U256},
    };

    use crate::types::user_operation::UserOperation;

    use super::*;
    use std::{str::FromStr, sync::Arc};

    #[tokio::test]
    #[ignore]
    async fn simulate_validation() {
        let eth_provider = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let entry_point = EntryPoint::<Provider<Http>>::new(
            eth_provider.clone(),
            Address::from_str("0x0576a174D229E3cFA37253523E645A78A0C91B57").unwrap(),
        );

        let max_priority_fee_per_gas = U256::from(1500000000_u64);
        let max_fee_per_gas =
            max_priority_fee_per_gas + eth_provider.get_gas_price().await.unwrap();

        let user_operation = UserOperation {
            sender: "0xBBe6a3230Ef8abC44EF61B3fBf93Cd0394D1d21f".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a80000000000000000000000000000000000000000000000000000018661be6ed7").unwrap(),
            call_data: Bytes::from_str("0xb61d27f6000000000000000000000000bbe6a3230ef8abc44ef61b3fbf93cd0394d1d21f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(22016),
            verification_gas_limit: U256::from(413910),
            pre_verification_gas: U256::from(48480),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xeb99f2f72c16b3eb5bdeadb243dd38a6e54771f1dd9b3d1d08e99e3e0840717331e6c8c83457c6c33daa3aa30a238197dbf7ea1f17d02aa57c3fa9e9ce3dc1731c").unwrap(),
        };

        let simulate_validation = entry_point
            .simulate_validation(user_operation.clone())
            .await
            .unwrap();

        assert!(matches!(
            simulate_validation,
            SimulateValidationResult::ValidationResult { .. },
        ));

        let simulate_validation_trace = entry_point
            .simulate_validation_trace(user_operation)
            .await
            .unwrap();

        assert!(matches!(
            simulate_validation_trace,
            GethTrace::Unknown { .. },
        ));
    }
}
