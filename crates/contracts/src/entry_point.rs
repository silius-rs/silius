pub use super::{
    error::EntryPointError,
    gen::{
        EntryPointAPI, EntryPointAPIEvents, ValidatePaymasterUserOpReturn, SELECTORS_INDICES,
        SELECTORS_NAMES,
    },
};
use super::{
    gen::entry_point_api::{
        DepositInfo, EntryPointAPIErrors, PackedUserOperation, SenderAddressResult,
    },
    tracer::JS_TRACER,
};
use crate::{
    error::decode_revert_error, executor_tracer::EXECUTOR_TRACER, gen::ExecutionResult,
    simulation_codes::ENTRYPOINT_SIMULATION_CODE_STR,
};
use ethers::{
    abi::{AbiDecode, AbiError},
    contract::{EthAbiCodec, EthAbiType},
    prelude::{ContractError, Event},
    providers::{CallBuilder, Middleware, ProviderError, RawCall},
    types::{
        spoof::State, transaction::eip2718::TypedTransaction, Address, Bytes, GethDebugTracerType,
        GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TransactionRequest, U256,
    },
};
use silius_primitives::UserOperationSigned;
use std::{fmt::Debug, str::FromStr, sync::Arc};

const UINT96_MAX: u128 = 5192296858534827628530496329220095;

#[derive(EthAbiCodec, EthAbiType, Debug, Clone, PartialEq, Eq)]
pub struct StakeInfo {
    pub stake: U256,
    pub unstake_delay_sec: U256,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnInfo {
    pub pre_op_gas: U256,
    pub prefund: U256,
    pub account_validation_data: U256,
    pub paymaster_validation_data: U256,
    pub paymaster_context: Bytes,
}

impl ReturnInfo {
    pub fn decode(data: &[u8]) -> Result<Self, AbiError> {
        let pre_op_gas = <U256 as AbiDecode>::decode(&data[..32])?;
        let prefund = <U256 as AbiDecode>::decode(&data[32..64])?;
        let account_validation_data = <U256 as AbiDecode>::decode(&data[64..96])?;
        let paymaster_validation_data = <U256 as AbiDecode>::decode(&data[96..128])?;
        let bytes_length = <U256 as AbiDecode>::decode(&data[160..192])?;
        let paymaster_context = Bytes::from_iter(&data[192..192 + bytes_length.as_usize()]);
        Ok(Self {
            pre_op_gas,
            prefund,
            account_validation_data,
            paymaster_validation_data,
            paymaster_context,
        })
    }
}

#[derive(EthAbiCodec, EthAbiType, Debug, Clone, PartialEq, Eq)]
pub struct AggregatorStakeInfo {
    pub aggregator: Address,
    pub stake_info: StakeInfo,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub sender_info: StakeInfo,
    pub factory_info: StakeInfo,
    pub paymaster_info: StakeInfo,
    pub aggresgator_info: AggregatorStakeInfo,
    pub return_info: ReturnInfo,
}

impl AbiDecode for ValidationResult {
    fn decode(data: impl AsRef<[u8]>) -> Result<Self, AbiError> {
        let data = data.as_ref();
        let sender_info = <StakeInfo as AbiDecode>::decode(&data[64..128])?;
        let factory_info = <StakeInfo as AbiDecode>::decode(&data[128..192])?;
        let paymaster_info = <StakeInfo as AbiDecode>::decode(&data[192..256])?;
        let aggresgator_info = <AggregatorStakeInfo as AbiDecode>::decode(&data[256..352])?;
        let return_info = ReturnInfo::decode(&data[352..])?;
        Ok(Self { sender_info, factory_info, paymaster_info, aggresgator_info, return_info })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulateValidationResult {
    ValidationResult(ValidationResult),
}

#[derive(Clone)]
pub struct EntryPoint<M: Middleware + 'static> {
    eth_client: Arc<M>,
    address: Address,
    entry_point_api: EntryPointAPI<M>,
}

impl<M: Middleware + 'static> EntryPoint<M> {
    pub fn new(eth_client: Arc<M>, address: Address) -> Self {
        let entry_point_api = EntryPointAPI::new(address, eth_client.clone());
        Self { eth_client, address, entry_point_api }
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
        err: ContractError<M>,
    ) -> Result<EntryPointAPIErrors, EntryPointError> {
        match err {
            ContractError::DecodingError(e) => {
                Err(EntryPointError::Decode { inner: e.to_string() })
            }
            ContractError::AbiError(e) => Err(EntryPointError::ABI { inner: e.to_string() }),
            ContractError::MiddlewareError { e } => EntryPointError::from_middleware_error::<M>(e),
            ContractError::ProviderError { e } => EntryPointError::from_provider_error(&e),
            ContractError::Revert(data) => decode_revert_error(data),
            _ => Err(EntryPointError::Other { inner: err.to_string() }),
        }
    }

    fn handle_contract_call_result<T: AbiDecode + Debug>(
        result: Result<Bytes, ProviderError>,
    ) -> Result<T, EntryPointError> {
        match result {
            Ok(b) => {
                let result = <T>::decode(b.as_ref());
                match result {
                    Ok(v) => Ok(v),
                    Err(e) => Err(EntryPointError::Decode {
                        inner: format!("decode failed : {e:?} with data {b:?}"),
                    }),
                }
            }
            Err(e) => match e {
                ProviderError::JsonRpcClientError(err_opt) => {
                    let err = {
                        err_opt
                            .as_error_response()
                            .and_then(|e| e.as_revert_data())
                            .and_then(|e| decode_revert_error(e).ok())
                    };
                    match err {
                        Some(e) => Err(e.into()),
                        None => Err(EntryPointError::Other {
                            inner: format!("json rpc error with: {err:?}"),
                        }),
                    }
                }
                _ => Err(EntryPointError::Other { inner: format!("failed with {e:?}") }),
            },
        }
    }

    pub async fn simulate_validation<U: Into<PackedUserOperation>>(
        &self,
        uo: U,
    ) -> Result<SimulateValidationResult, EntryPointError> {
        let uo = uo.into();
        let call = self.entry_point_api.simulate_validation(uo);
        let mut tx = call.tx;
        tx.set_gas(100000000);
        let call_builder = CallBuilder::new(self.eth_client.provider(), &tx);
        let mut state = State::default();
        state.account(self.address).code(
            Bytes::from_str(ENTRYPOINT_SIMULATION_CODE_STR)
                .expect("hard coded code should be correct"),
        );
        let call_builder = call_builder.state(&state);
        let res = call_builder.await;
        Self::handle_contract_call_result::<ValidationResult>(res)
            .map(SimulateValidationResult::ValidationResult)
    }

    pub async fn simulate_validation_trace<U: Into<PackedUserOperation>>(
        &self,
        uo: U,
    ) -> Result<GethTrace, EntryPointError> {
        let call = self.entry_point_api.simulate_validation(uo.into());
        let mut state = State::default();
        state.account(self.address).code(
            Bytes::from_str(ENTRYPOINT_SIMULATION_CODE_STR)
                .expect("hard coded code should be correct"),
        );
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
                        tracer: Some(GethDebugTracerType::JsTracer(JS_TRACER.into())),
                        tracer_config: None,
                        timeout: None,
                    },
                    state_overrides: Some(state),
                    block_overrides: None,
                },
            )
            .await
            .map_err(|e| {
                EntryPointError::from_middleware_error::<M>(e).expect_err("trace err is expected")
            })?;

        Ok(res)
    }

    pub async fn simulate_handle_op_trace(
        &self,
        uo: UserOperationSigned,
    ) -> Result<GethTrace, EntryPointError> {
        let max_fee_per_gas = uo.max_fee_per_gas;
        let uo: PackedUserOperation = uo.into();
        let call = self.entry_point_api.simulate_handle_op(uo, Address::zero(), Bytes::default());
        let mut tx: TypedTransaction = call.tx;
        tx.set_from(Address::zero());
        tx.set_gas_price(max_fee_per_gas);
        tx.set_gas(u64::MAX);
        let mut state = State::default();
        state.account(self.address).code(
            Bytes::from_str(ENTRYPOINT_SIMULATION_CODE_STR)
                .expect("hard coded code should be correct"),
        );
        state.account(Address::zero()).balance(UINT96_MAX.into());
        let res = self
            .eth_client
            .debug_trace_call(
                tx,
                None,
                GethDebugTracingCallOptions {
                    tracing_options: GethDebugTracingOptions {
                        disable_storage: None,
                        disable_stack: None,
                        enable_memory: None,
                        enable_return_data: None,
                        tracer: Some(GethDebugTracerType::JsTracer(EXECUTOR_TRACER.into())),
                        tracer_config: None,
                        timeout: None,
                    },
                    state_overrides: Some(state),
                    block_overrides: None,
                },
            )
            .await
            .map_err(|e| {
                EntryPointError::from_middleware_error::<M>(e).expect_err("trace err is expected")
            })?;

        Ok(res)
    }

    pub async fn handle_ops<U: Into<PackedUserOperation>>(
        &self,
        uos: Vec<U>,
        beneficiary: Address,
    ) -> Result<(), EntryPointError> {
        self.entry_point_api
            .handle_ops(
                uos.into_iter().map(|u| u.into()).collect::<Vec<PackedUserOperation>>(),
                beneficiary,
            )
            .call()
            .await
            .or_else(|e| {
                Self::deserialize_error_msg(e).and_then(|op| match op {
                    EntryPointAPIErrors::FailedOp(err) => Err(EntryPointError::FailedOp(err)),
                    _ => Err(EntryPointError::Other { inner: format!("handle ops error: {op:?}") }),
                })
            })
    }

    pub async fn get_deposit_info(&self, addr: &Address) -> Result<DepositInfo, EntryPointError> {
        let res = self.entry_point_api.get_deposit_info(*addr).call().await;

        match res {
            Ok((deposit, staked, stake, unstake_delay_sec, withdraw_time)) => {
                Ok(DepositInfo { deposit, staked, stake, unstake_delay_sec, withdraw_time })
            }
            Err(err) => {
                Err(EntryPointError::Other { inner: format!("get deposit info error: {err:?}") })
            }
        }
    }

    pub async fn balance_of(&self, addr: &Address) -> Result<U256, EntryPointError> {
        let res = self.entry_point_api.balance_of(*addr).call().await;

        match res {
            Ok(balance) => Ok(balance),
            Err(err) => Err(EntryPointError::Other { inner: format!("balance of error: {err:?}") }),
        }
    }

    pub async fn get_nonce(&self, address: &Address, key: U256) -> Result<U256, EntryPointError> {
        let res = self.entry_point_api.get_nonce(*address, key).call().await;

        match res {
            Ok(nonce) => Ok(nonce),
            Err(err) => Err(EntryPointError::Other { inner: format!("get nonce error: {err:?}") }),
        }
    }

    pub async fn get_sender_address(
        &self,
        init_code: Bytes,
    ) -> Result<SenderAddressResult, EntryPointError> {
        let res = self.entry_point_api.get_sender_address(init_code).call().await;

        match res {
            Ok(_) => Err(EntryPointError::NoRevert { function: "get_sender_address".into() }),
            Err(e) => Self::deserialize_error_msg(e).and_then(|op| match op {
                EntryPointAPIErrors::SenderAddressResult(res) => Ok(res),
                EntryPointAPIErrors::FailedOp(err) => Err(EntryPointError::FailedOp(err)),
                _ => Err(EntryPointError::Other {
                    inner: format!("get sender address error: {op:?}"),
                }),
            }),
        }
    }

    pub async fn simulate_execution<U: Into<PackedUserOperation>>(
        &self,
        uo: U,
    ) -> Result<Bytes, EntryPointError> {
        let uo: PackedUserOperation = uo.into();

        self.eth_client
            .call(
                &TransactionRequest::new()
                    .from(self.address)
                    .to(uo.sender)
                    .data(uo.call_data.clone())
                    .into(),
                None,
            )
            .await
            .map_err(|err| EntryPointError::Provider { inner: err.to_string() })
    }

    pub async fn simulate_handle_op<U: Into<PackedUserOperation>>(
        &self,
        uo: U,
    ) -> Result<ExecutionResult, EntryPointError> {
        let call =
            self.entry_point_api.simulate_handle_op(uo.into(), Address::zero(), Bytes::default());
        let call_builder = CallBuilder::new(self.eth_client.provider(), &call.tx);
        let mut state = State::default();
        state.account(self.address).code(
            Bytes::from_str(ENTRYPOINT_SIMULATION_CODE_STR)
                .expect("hard coded code should be correct"),
        );
        let call_builder = call_builder.state(&state);
        let res = call_builder.await;
        Self::handle_contract_call_result::<ExecutionResult>(res)
    }

    pub async fn handle_aggregated_ops<U: Into<PackedUserOperation>>(
        &self,
        _uos_per_aggregator: Vec<U>,
        _beneficiary: Address,
    ) -> Result<(), EntryPointError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};

    #[tokio::test]
    #[ignore]
    async fn simulate_validation() {
        let eth_client = Arc::new(Provider::try_from("http://192.168.1.218:8545").unwrap());
        let ep = EntryPoint::<Provider<Http>>::new(
            eth_client.clone(),
            "0x0000000071727De22E5E9d8BAf0edAc6f37da032".parse().unwrap(),
        );

        let max_priority_fee_per_gas = 1500000000_u64.into();
        let max_fee_per_gas = max_priority_fee_per_gas + eth_client.get_gas_price().await.unwrap();

        let uo = UserOperationSigned {
            sender: "0xBBe6a3230Ef8abC44EF61B3fBf93Cd0394D1d21f".parse().unwrap(),
            nonce: U256::zero(),
            factory: "0xed886f2d1bbb38b4914e8c545471216a40cce938".parse().unwrap(),
            factory_data: "5fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a80000000000000000000000000000000000000000000000000000018661be6ed7".parse().unwrap(),
            call_data: "0xb61d27f6000000000000000000000000bbe6a3230ef8abc44ef61b3fbf93cd0394d1d21f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000".parse().unwrap(),
            call_gas_limit: 22016.into(),
            verification_gas_limit: 413910.into(),
            pre_verification_gas: 48480.into(),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            signature: "0xeb99f2f72c16b3eb5bdeadb243dd38a6e54771f1dd9b3d1d08e99e3e0840717331e6c8c83457c6c33daa3aa30a238197dbf7ea1f17d02aa57c3fa9e9ce3dc1731c".parse().unwrap(),
            ..Default::default()
        };

        let res = ep.simulate_validation(uo.clone()).await.unwrap();

        assert!(matches!(res, SimulateValidationResult::ValidationResult { .. },));

        let trace = ep.simulate_validation_trace(uo).await.unwrap();

        assert!(matches!(trace, GethTrace::Unknown { .. },));
    }
}
