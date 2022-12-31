use std::str::FromStr;
use std::sync::Arc;

use anyhow;
use ethers::abi::AbiDecode;
use ethers::providers::{Middleware, ProviderError};
use ethers::types::{Address, Bytes, GethDebugTracingCallOptions, GethTrace};
use regex::Regex;
use serde::Deserialize;

use super::gen::entry_point_api::{self, UserOperation};
use super::gen::EntryPointAPI;

pub struct EntryPoint<M: Middleware> {
    provider: Arc<M>,
    entry_point_address: Address,
    api: EntryPointAPI<M>,
}

impl<M: Middleware + 'static> EntryPoint<M> {
    pub fn new(provider: Arc<M>, entry_point_address: Address) -> Self {
        let api = EntryPointAPI::new(entry_point_address, provider.clone());
        Self {
            provider,
            entry_point_address,
            api,
        }
    }

    pub fn provider(&self) -> Arc<M> {
        self.provider.clone()
    }

    pub fn entry_point_address(&self) -> Address {
        self.entry_point_address
    }

    fn deserialize_error_msg(
        err_msg: &str,
    ) -> Result<entry_point_api::EntryPointAPIErrors, EntryPointErr> {
        JsonRpcError::from_str(err_msg)
            .map_err(|_| {
                EntryPointErr::DecodeErr(format!(
                    "{:?} is not a valid JsonRpcError message",
                    err_msg
                ))
            })
            .and_then(|json_error| {
                json_error.data.ok_or_else(|| {
                    EntryPointErr::DecodeErr("{:?} doesn't have valid data field".to_string())
                })
            })
            .and_then(|data: String| {
                AbiDecode::decode_hex(data).map_err(|_| {
                    EntryPointErr::DecodeErr(format!(
                        "{:?} data field could not be deserialize to EntryPointAPIErrors",
                        err_msg
                    ))
                })
            })
    }

    pub async fn simulate_validation<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<SimulateValidationResult, EntryPointErr> {
        let request_result = self.api.simulate_validation(user_operation.into()).await;
        match request_result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Simulate validation should expect revert".to_string(),
            )),
            Err(e) => {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    entry_point_api::EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    entry_point_api::EntryPointAPIErrors::SimulationResult(res) => {
                        Ok(SimulateValidationResult::SimulationResult(res))
                    }
                    entry_point_api::EntryPointAPIErrors::SimulationResultWithAggregation(res) => {
                        Ok(SimulateValidationResult::SimulationResultWithAggregation(
                            res,
                        ))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Simulate validation with invalid error: {:?}",
                        op
                    ))),
                })
            }
        }
    }

    pub async fn simulate_validation_trace<U: Into<UserOperation>>(
        &self,
        user_operation: U,
    ) -> Result<GethTrace, EntryPointErr> {
        let call = self.api.simulate_validation(user_operation.into());
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
    ) -> Result<(), EntryPointErr> {
        self.api
            .handle_ops(ops.into_iter().map(|u| u.into()).collect(), beneficiary)
            .await
            .or_else(|e| {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    entry_point_api::EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Handle ops with invalid error: {:?}",
                        op
                    ))),
                })
            })
    }

    pub async fn get_sender_address<U: Into<UserOperation>>(
        &self,
        initcode: Bytes,
    ) -> Result<entry_point_api::SenderAddressResult, EntryPointErr> {
        let result = self.api.get_sender_address(initcode).await;

        match result {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Get sender address should expect revert".to_string(),
            )),
            Err(e) => {
                let err_msg = e.to_string();
                Self::deserialize_error_msg(&err_msg).and_then(|op| match op {
                    entry_point_api::EntryPointAPIErrors::SenderAddressResult(res) => Ok(res),
                    entry_point_api::EntryPointAPIErrors::FailedOp(failed_op) => {
                        Err(EntryPointErr::FailedOp(failed_op))
                    }
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Simulate validation with invalid error: {:?}",
                        op
                    ))),
                })
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

#[derive(Debug)]
pub enum EntryPointErr {
    FailedOp(entry_point_api::FailedOp),
    ProviderErr(ProviderError),
    NetworkErr, // TODO
    DecodeErr(String),
    UnknownErr(String), // describe impossible error. We should fix the codes here(or contract codes) if this occurs.
}

impl From<ProviderError> for EntryPointErr {
    fn from(e: ProviderError) -> Self {
        EntryPointErr::ProviderErr(e)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    SimulationResult(entry_point_api::SimulationResult),
    SimulationResultWithAggregation(entry_point_api::SimulationResultWithAggregation),
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
        let captures = re.captures(s).unwrap();
        let code = captures[1].parse::<u64>().unwrap();
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
