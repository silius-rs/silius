use super::gen::entry_point_api::{
    EntryPointAPIErrors, FailedOp, SenderAddressResult, UserOperation, ValidationResult,
    ValidationResultWithAggregation,
};
use super::gen::stake_manager_api::DepositInfo;
pub use super::gen::{
    EntryPointAPI, EntryPointAPIEvents, StakeManagerAPI, UserOperationEventFilter,
    ValidatePaymasterUserOpReturn, CONTRACTS_FUNCTIONS,
};
use super::tracer::JS_TRACER;
use crate::gen::ExecutionResult;
use ethers::abi::AbiDecode;
use ethers::prelude::{ContractError, Event};
use ethers::providers::{JsonRpcError, Middleware, MiddlewareError, ProviderError};
use ethers::types::{
    Address, Bytes, GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
    GethTrace, TransactionRequest, U256,
};
use std::fmt::Display;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    ValidationResult(ValidationResult),
    ValidationResultWithAggregation(ValidationResultWithAggregation),
}

#[derive(Clone)]
pub struct EntryPoint<M: Middleware + 'static> {
    eth_client: Arc<M>,
    address: Address,
    entry_point_api: EntryPointAPI<M>,
    stake_manager_api: StakeManagerAPI<M>,
}

impl<M: Middleware + 'static> EntryPoint<M> {
    pub fn new(eth_client: Arc<M>, address: Address) -> Self {
        let entry_point_api = EntryPointAPI::new(address, eth_client.clone());
        let stake_manager_api = StakeManagerAPI::new(address, eth_client.clone());
        Self {
            eth_client,
            address,
            entry_point_api,
            stake_manager_api,
        }
    }

    pub fn entry_point_api(&self) -> &EntryPointAPI<M> {
        &self.entry_point_api
    }

    pub fn events(&self) -> Event<Arc<M>, M, EntryPointAPIEvents> {
        self.entry_point_api.events()
    }

    pub fn eth_client(&self) -> Arc<M> {
        self.eth_client.clone()
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
            ContractError::Revert(data) => {
                let decoded = EntryPointAPIErrors::decode(data.as_ref());
                match decoded {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        // ethers-rs could not handle `require (true, "reason")` well in this case
                        // revert with `require` error would ends up with error event signature `0x08c379a0`
                        // we need to handle it manually
                        let (error_sig, reason) = data.split_at(4);
                        if error_sig == [0x08, 0xc3, 0x79, 0xa0] {
                            return <String as AbiDecode>::decode(reason)
                                .map(EntryPointAPIErrors::RevertString)
                                .map_err(|e| {
                                    EntryPointErr::DecodeErr(format!(
                                        "{e:?} data field could not be deserialize to revert error",
                                    ))
                                });
                        }
                        Err(EntryPointErr::DecodeErr(format!(
                            "{e:?} data field could not be deserialize to EntryPointAPIErrors",
                        )))
                    }
                }
            }
            _ => Err(EntryPointErr::UnknownErr(format!(
                "Unkown error: {err_msg:?}",
            ))),
        }
    }

    pub async fn simulate_validation<U: Into<UserOperation>>(
        &self,
        uo: U,
    ) -> Result<SimulateValidationResult, EntryPointErr> {
        let res = self.entry_point_api.simulate_validation(uo.into()).await;

        match res {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Simulate validation should expect revert".to_string(),
            )),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::FailedOp(err) => Err(EntryPointErr::FailedOp(err)),
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
        uo: U,
    ) -> Result<GethTrace, EntryPointErr> {
        let call = self.entry_point_api.simulate_validation(uo.into());

        let res = self
            .eth_client
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
                    state_overrides: None,
                },
            )
            .await
            .map_err(|e| EntryPointErr::from_middleware_err::<M>(e))?;

        Ok(res)
    }

    pub async fn handle_ops<U: Into<UserOperation>>(
        &self,
        uos: Vec<U>,
        beneficiary: Address,
    ) -> Result<(), EntryPointErr> {
        self.entry_point_api
            .handle_ops(uos.into_iter().map(|u| u.into()).collect(), beneficiary)
            .call()
            .await
            .or_else(|e| {
                Self::deserialize_error_msg(e).and_then(|op| match op {
                    EntryPointAPIErrors::FailedOp(err) => Err(EntryPointErr::FailedOp(err)),
                    _ => Err(EntryPointErr::UnknownErr(format!(
                        "Handle ops with invalid error: {op:?}"
                    ))),
                })
            })
    }

    pub async fn get_deposit_info(&self, addr: &Address) -> Result<DepositInfo, EntryPointErr> {
        let res = self.stake_manager_api.get_deposit_info(*addr).call().await;

        match res {
            Ok(deposit_info) => Ok(deposit_info),
            _ => Err(EntryPointErr::UnknownErr(
                "Error calling get deposit info".to_string(),
            )),
        }
    }

    pub async fn balance_of(&self, addr: &Address) -> Result<U256, EntryPointErr> {
        let res = self.stake_manager_api.balance_of(*addr).call().await;

        match res {
            Ok(balance) => Ok(balance),
            _ => Err(EntryPointErr::UnknownErr(
                "Error calling balance of".to_string(),
            )),
        }
    }

    pub async fn get_sender_address(
        &self,
        init_code: Bytes,
    ) -> Result<SenderAddressResult, EntryPointErr> {
        let res = self
            .entry_point_api
            .get_sender_address(init_code)
            .call()
            .await;

        match res {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Get sender address should expect revert".to_string(),
            )),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::SenderAddressResult(res) => Ok(res),
                EntryPointAPIErrors::FailedOp(err) => Err(EntryPointErr::FailedOp(err)),
                _ => Err(EntryPointErr::UnknownErr(format!(
                    "Simulate validation with invalid error: {op:?}"
                ))),
            }),
        }
    }

    pub async fn simulate_execution<U: Into<UserOperation>>(
        &self,
        uo: U,
    ) -> Result<(), EntryPointErr> {
        let uo: UserOperation = uo.into();

        let res = self
            .eth_client
            .call(
                &TransactionRequest::new()
                    .from(self.address)
                    .to(uo.sender)
                    .data(uo.call_data.clone())
                    .into(),
                None,
            )
            .await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(EntryPointErr::from_middleware_err::<M>(e)),
        }
    }

    pub async fn simulate_handle_op<U: Into<UserOperation>>(
        &self,
        uo: U,
    ) -> Result<ExecutionResult, EntryPointErr> {
        let res = self
            .entry_point_api
            .simulate_handle_op(uo.into(), Address::zero(), Bytes::default())
            .await;

        match res {
            Ok(_) => Err(EntryPointErr::UnknownErr(
                "Simulate handle op should expect revert".to_string(),
            )),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::FailedOp(err) => Err(EntryPointErr::FailedOp(err)),
                EntryPointAPIErrors::ExecutionResult(res) => Ok(res),
                _ => Err(EntryPointErr::UnknownErr(format!(
                    "Simulate handle op with invalid error: {op:?}"
                ))),
            }),
        }
    }

    pub async fn handle_aggregated_ops<U: Into<UserOperation>>(
        &self,
        _uos_per_aggregator: Vec<U>,
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
    fn from_provider_err(err: &ProviderError) -> Self {
        match err {
            ProviderError::JsonRpcClientError(err) => err
                .as_error_response()
                .map(|e| EntryPointErr::JsonRpcError(e.clone()))
                .unwrap_or(EntryPointErr::UnknownErr(format!(
                    "Unknown JSON-RPC client error: {err:?}"
                ))),
            ProviderError::HTTPError(err) => {
                EntryPointErr::NetworkErr(format!("Entry point HTTP error: {err:?}"))
            }
            _ => EntryPointErr::UnknownErr(format!("Unknown error in provider: {err:?}")),
        }
    }

    fn from_middleware_err<M: Middleware>(err: M::Error) -> Self {
        if let Some(err) = err.as_error_response() {
            return EntryPointErr::JsonRpcError(err.clone());
        }

        if let Some(err) = err.as_provider_error() {
            return EntryPointErr::from_provider_err(err);
        }

        EntryPointErr::UnknownErr(format!("Unknown middleware error: {err:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::{
        providers::{Http, Middleware, Provider},
        types::{Bytes, GethTrace, U256},
    };
    use silius_primitives::UserOperation;
    use std::{str::FromStr, sync::Arc};

    #[tokio::test]
    #[ignore]
    async fn simulate_validation() {
        let eth_client = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let ep = EntryPoint::<Provider<Http>>::new(
            eth_client.clone(),
            "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .unwrap(),
        );

        let max_priority_fee_per_gas = 1500000000_u64.into();
        let max_fee_per_gas = max_priority_fee_per_gas + eth_client.get_gas_price().await.unwrap();

        let uo = UserOperation {
            sender: "0xBBe6a3230Ef8abC44EF61B3fBf93Cd0394D1d21f".parse().unwrap(),
            nonce: U256::zero(),
            init_code: "0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a80000000000000000000000000000000000000000000000000000018661be6ed7".parse().unwrap(),
            call_data: "0xb61d27f6000000000000000000000000bbe6a3230ef8abc44ef61b3fbf93cd0394d1d21f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_gas_limit: 22016.into(),
            verification_gas_limit: 413910.into(),
            pre_verification_gas: 48480.into(),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            paymaster_and_data: Bytes::default(),
            signature: "0xeb99f2f72c16b3eb5bdeadb243dd38a6e54771f1dd9b3d1d08e99e3e0840717331e6c8c83457c6c33daa3aa30a238197dbf7ea1f17d02aa57c3fa9e9ce3dc1731c".parse().unwrap(),
        };

        let res = ep.simulate_validation(uo.clone()).await.unwrap();

        assert!(matches!(
            res,
            SimulateValidationResult::ValidationResult { .. },
        ));

        let trace = ep.simulate_validation_trace(uo).await.unwrap();

        assert!(matches!(trace, GethTrace::Unknown { .. },));
    }

    #[test]
    fn deserialize_error_msg() -> anyhow::Result<()> {
        let err_msg = Bytes::from_str("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001841413934206761732076616c756573206f766572666c6f770000000000000000")?;
        let res = EntryPointAPIErrors::decode(err_msg)?;
        println!("res: {:?}", res);
        match res {
            EntryPointAPIErrors::RevertString(s) => {
                assert_eq!(s, "AA94 gas values overflow")
            }
            _ => panic!("Invalid error message"),
        }

        let err_msg = Bytes::from_str("0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001841413934206761732076616c756573206f766572666c6f770000000000000000")?;
        let res = EntryPointAPIErrors::decode(err_msg);
        assert!(
            matches!(res, Err(_)),
            "ethers-rs derivatives could not handle revert error correctly"
        );
        Ok(())
    }
}
