use ethers::{
    abi::AbiDecode,
    contract::ContractError,
    prelude::HttpClientError,
    providers::{Http, Provider, ProviderError},
};

use crate::{
    contracts::gen::entry_point_api::{self, SimulationResult},
    types::user_operation::UserOperation,
    uopool::services::uopool::UoPoolService,
};

#[derive(Debug)]
pub enum BadUserOperationError {
    EntryPointSimulateValidation { result: String },
}

#[derive(Debug)]
pub enum UserOperationSimulationError {
    Simulation(BadUserOperationError),
    Internal(anyhow::Error),
    Provider(ProviderError),
}

impl From<anyhow::Error> for UserOperationSimulationError {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSimulationError::Internal(e)
    }
}

impl From<ProviderError> for UserOperationSimulationError {
    fn from(e: ProviderError) -> Self {
        UserOperationSimulationError::Provider(e)
    }
}

impl UoPoolService {
    async fn entry_point_simulate_validation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulationResult, UserOperationSimulationError> {
        // View call to simulateValidation(userop)
        let simulation_response = self
            .entry_point
            .simulate_validation(entry_point_api::UserOperation::from(user_operation.clone()))
            .await;

        let result = format!("{:?}", simulation_response);

        if let Err(ContractError::MiddlewareError(ProviderError::JsonRpcClientError(
            json_rpc_client_error,
        ))) = simulation_response
        {
            let http_client_error = json_rpc_client_error
                .downcast_ref::<HttpClientError>()
                .unwrap();

            if let HttpClientError::JsonRpcError(json_rpc_error) = http_client_error {
                if let Some(value) = json_rpc_error.data.clone() {
                    if let Some(hex_string) = value.as_str() {
                        if let Ok(simulation_result) = SimulationResult::decode_hex(hex_string) {
                            return Ok(simulation_result);
                        }
                    }
                }
            }
        }

        return Err(UserOperationSimulationError::Simulation(
            BadUserOperationError::EntryPointSimulateValidation { result },
        ));
    }

    async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulationResult, UserOperationSimulationError> {
        let simulation_result = self.entry_point_simulate_validation(user_operation).await?;

        Ok(simulation_result)
    }
}

#[cfg(test)]
mod tests {
    use crate::uopool::UserOperationPool;
    use ethers::types::{Bytes, U256};
    use std::{str::FromStr, sync::Arc};

    use super::*;

    #[tokio::test]
    async fn user_operation_simulation() {
        let uo_pool_service = UoPoolService::new(
            Arc::new(UserOperationPool::new()),
            Arc::new(Provider::try_from("https://rpc-mumbai.maticvigil.com/").unwrap()),
            "0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17"
                .parse()
                .unwrap(),
            U256::from(1500000),
        );

        let user_operation_valid = UserOperation {
            sender: "0xAB7e2cbFcFb6A5F33A75aD745C3E5fB48d689B54".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xe19e9755942bb0bd0cccce25b1742596b8a8250b3bf2c3e70000000000000000000000001d9a2cb3638c2fc8bf9c01d088b79e75cd188b17000000000000000000000000789d9058feecf1948af429793e7f1eb4a75db2220000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000ab7e2cbfcfb6a5f33a75ad745c3e5fb48d689b5400000000000000000000000000000000000000000000000002c68af0bb14000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(21900),
            verification_gas_limit: U256::from(1218343),
            pre_verification_gas: U256::from(50768),
            max_fee_per_gas: U256::from(3501638950 as u64),
            max_priority_fee_per_gas: U256::from(2551157264 as u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xb5a4efa90d560f95b508e6b0e7c2dc17a7e86928af551175fe2d9f6a1bd79a604e8a83a391d25c4b3dce56a0a1549c5f40d1a08c3f4b80982556efa768eca7f81c").unwrap(),
        };

        assert_eq!(
            uo_pool_service
                .simulate_user_operation(&user_operation_valid)
                .await
                .unwrap(),
            SimulationResult {
                pre_op_gas: U256::from(1180466),
                prefund: U256::from(3293572109919069 as u64),
                deadline: U256::from(0),
                paymaster_info: (U256::from(0), U256::from(0)),
                sender_info: (U256::from(0), U256::from(0)),
                factory_info: (U256::from(0), U256::from(0)),
            }
        );
    }
}
