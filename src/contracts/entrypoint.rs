use std::str::FromStr;
use std::sync::Arc;

use anyhow;
use ethers::abi::AbiDecode;
use ethers::providers::{FromErr, Middleware, ProviderError};
use ethers::types::{Address, Bytes, GethDebugTracingCallOptions, GethTrace};
use regex::Regex;
use serde::Deserialize;

use super::gen::entry_point_api::{
    EntryPointAPIErrors, FailedOp, SenderAddressResult, UserOperation, ValidationResult,
    ValidationResultWithAggregation,
};
use super::gen::stake_manager_api::DepositInfo;
use super::gen::{EntryPointAPI, StakeManagerAPI};

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
            .call()
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

    // TODO: change to javascript tracer
    pub async fn simulate_validation_trace<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<GethTrace, EntryPointErr<M>> {
        let call = self
            .entry_point_api
            .simulate_validation(user_operation.into());
        let options: GethDebugTracingCallOptions = GethDebugTracingCallOptions::default();
        let request_result = self
            .provider
            .debug_trace_call(call.tx, None, options)
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
    use super::JsonRpcError;
    use std::str::FromStr;

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
}
