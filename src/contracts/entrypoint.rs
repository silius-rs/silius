use std::str::FromStr;
use std::sync::Arc;

use anyhow;
use ethers::abi::AbiDecode;
use ethers::providers::{FromErr, Middleware, ProviderError};
use ethers::types::{
    Address, Bytes, GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
    GethTrace,
};
use regex::Regex;
use serde::Deserialize;

use super::gen::entry_point_api::{
    EntryPointAPIErrors, FailedOp, SenderAddressResult, UserOperation, ValidationResult,
    ValidationResultWithAggregation,
};
use super::gen::stake_manager_api::DepositInfo;
use super::gen::{EntryPointAPI, StakeManagerAPI};
use super::tracer::JS_TRACER;

pub struct EntryPoint<M: Middleware> {
    provider: Arc<M>,
    address: Address,
    entry_point_api: EntryPointAPI<M>,
    stake_manager_api: StakeManagerAPI<M>,
}

impl<M: Middleware + 'static> EntryPoint<M>
where
    EntryPointErr<M>: From<<M as Middleware>::Error>,
{
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

    fn deserialize_error_msg(err_msg: &str) -> Result<EntryPointAPIErrors, EntryPointErr<M>> {
        JsonRpcError::from_str(err_msg)
            .map_err(|_| {
                EntryPointErr::DecodeErr(format!("{err_msg:?} is not a valid JsonRpcError message"))
            })
            .and_then(|json_error| {
                json_error.data.ok_or_else(|| {
                    EntryPointErr::DecodeErr(
                        "{json_error:?} doesn't have valid data field".to_string(),
                    )
                })
            })
            .and_then(|data: String| {
                AbiDecode::decode_hex(data).map_err(|_| {
                    EntryPointErr::DecodeErr(format!(
                        "{err_msg:?} data field could not be deserialize to EntryPointAPIErrors"
                    ))
                })
            })
    }

    pub async fn simulate_validation<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<SimulateValidationResult, EntryPointErr<M>> {
        let request_result = self
            .entry_point_api
            .simulate_validation(user_operation.into())
            .await;
        match request_result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Simulate validation should expect revert".to_string(),
            )),
            Err(e) => {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    EntryPointAPIErrors::ValidationResult(res) => {
                        Ok(SimulateValidationResult::ValidationResult(res))
                    }
                    EntryPointAPIErrors::ValidationResultWithAggregation(res) => Ok(
                        SimulateValidationResult::ValidationResultWithAggregation(res),
                    ),
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Simulate validation with invalid error: {op:?}"
                    ))),
                })
            }
        }
    }

    pub async fn simulate_validation_trace<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<GethTrace, EntryPointErr<M>> {
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
            .await?;
        Ok(request_result)
    }

    pub async fn handle_ops<U: Into<UserOperation>>(
        &self,
        ops: Vec<U>,
        beneficiary: Address,
    ) -> Result<(), EntryPointErr<M>> {
        self.entry_point_api
            .handle_ops(ops.into_iter().map(|u| u.into()).collect(), beneficiary)
            .call()
            .await
            .or_else(|e| {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Handle ops with invalid error: {op:?}"
                    ))),
                })
            })
    }

    pub async fn get_deposit_info(
        &self,
        address: &Address,
    ) -> Result<DepositInfo, EntryPointErr<M>> {
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
    ) -> Result<SenderAddressResult, EntryPointErr<M>> {
        let result = self
            .entry_point_api
            .get_sender_address(initcode)
            .call()
            .await;

        match result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Get sender address should expect revert".to_string(),
            )),
            Err(e) => {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    EntryPointAPIErrors::SenderAddressResult(res) => Ok(res),
                    EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Simulate validation with invalid error: {op:?}"
                    ))),
                })
            }
        }
    }

    pub async fn handle_aggregated_ops<U: Into<UserOperation>>(
        &self,
        _ops_per_aggregator: Vec<U>,
        _beneficiary: Address,
    ) -> Result<(), EntryPointErr<M>> {
        todo!()
    }
}

#[derive(Debug)]
pub enum EntryPointErr<M: Middleware> {
    FailedOp(FailedOp),
    ProviderErr(ProviderError),
    MiddlewareErr(M::Error),
    NetworkErr, // TODO
    DecodeErr(String),
    UnknownErr(String), // describe impossible error. We should fix the codes here(or contract codes) if this occurs.
}

impl<M: Middleware> From<ProviderError> for EntryPointErr<M> {
    fn from(e: ProviderError) -> Self {
        EntryPointErr::ProviderErr(e)
    }
}

impl<M: Middleware> FromErr<M::Error> for EntryPointErr<M> {
    fn from(src: M::Error) -> Self {
        EntryPointErr::MiddlewareErr(src)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    ValidationResult(ValidationResult),
    ValidationResultWithAggregation(ValidationResultWithAggregation),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct JsonRpcError {
    /// The error code
    pub code: u64,
    /// The error message
    pub message: String,
    /// Additional data
    pub data: Option<String>,
}

impl FromStr for JsonRpcError {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(
            r###"code: (\d+), message: ([^,]*), data: (None|Some\(String\("([^)]*)"\))"###,
        )?;
        let captures = re
            .captures(s)
            .ok_or_else(|| anyhow::anyhow!("The return error is not a valid JsonRpcError"))?;
        let code = captures[1].parse::<u64>()?;
        let message = &captures[2];
        let data = match &captures[3] {
            "None" => None,
            _ => Some(captures[4].to_string()),
        };
        Ok(JsonRpcError {
            code,
            message: message.to_string(),
            data,
        })
    }
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

    #[test]
    fn json_rpc_err_parse() {
        let some_data =
            "(code: 3, message: execution reverted: , data: Some(String(\"0x00fa072b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000001941413230206163636f756e74206e6f74206465706c6f79656400000000000000\")))";
        let err = JsonRpcError::from_str(some_data);

        assert_eq!(
            err.unwrap(),
            JsonRpcError {
                code: 3,
                message: "execution reverted: ".to_string(),
                data: Some("0x00fa072b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000001941413230206163636f756e74206e6f74206465706c6f79656400000000000000".to_string())
            }
        );

        let none_data = "(code: 3, message: execution reverted, data: None)";
        let err2 = JsonRpcError::from_str(none_data);
        assert_eq!(
            err2.unwrap(),
            JsonRpcError {
                code: 3,
                message: "execution reverted".to_string(),
                data: None
            }
        );
    }

    #[ignore]
    #[tokio::test]
    async fn simulate_validation() {
        let eth_provider = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let entry_point = EntryPoint::<Provider<Http>>::new(
            eth_provider.clone(),
            Address::from_str("0x1306b01bc3e4ad202612d3843387e94737673f53").unwrap(),
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
