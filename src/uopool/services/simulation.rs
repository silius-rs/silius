use crate::{
    contracts::{gen::entry_point_api, EntryPointErr, SimulateValidationResult},
    types::user_operation::UserOperation,
    uopool::services::uopool::UoPoolService,
};
use ethers::providers::Middleware;

#[derive(Debug)]
pub enum BadUserOperationError {
    // EntryPointSimulateValidation { result: String },
}

#[derive(Debug)]
pub enum UserOperationSimulationError {
    SimulationError(BadUserOperationError),
    EntryPointError(EntryPointErr),
    InternalError(anyhow::Error),
}

impl From<anyhow::Error> for UserOperationSimulationError {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSimulationError::InternalError(e)
    }
}

impl From<EntryPointErr> for UserOperationSimulationError {
    fn from(e: EntryPointErr) -> Self {
        UserOperationSimulationError::EntryPointError(e)
    }
}

impl<M: Middleware + 'static> UoPoolService<M> {
    async fn entry_point_simulate_validation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulateValidationResult, UserOperationSimulationError> {
        // View call to simulateValidation(userop)
        let simulation_response = self
            .entry_point
            .simulate_validation(entry_point_api::UserOperation::from(user_operation.clone()))
            .await?;
        Ok(simulation_response)
    }

    async fn entry_point_simulate_validation_trace(
        &self,
        user_operation: &UserOperation,
    ) -> Result<(), UserOperationSimulationError> {
        let simulation_trace = self
            .entry_point
            .simulate_validation_trace(entry_point_api::UserOperation::from(user_operation.clone()))
            .await?;
        println!("{simulation_trace:?}");
        Ok(())
    }

    async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<SimulateValidationResult, UserOperationSimulationError> {
        let simulation_result = self.entry_point_simulate_validation(user_operation).await?;
        self.entry_point_simulate_validation_trace(user_operation)
            .await?;
        Ok(simulation_result)
    }
}

#[cfg(test)]
mod tests {
    use crate::uopool::UserOperationPool;
    use ethers::{
        prelude::Abigen,
        providers::Provider,
        types::{Bytes, U256},
    };
    use std::{str::FromStr, sync::Arc};

    use super::*;

    #[tokio::test]
    async fn user_operation_simulation() {
        let uo_pool_service = UoPoolService::new(
            Arc::new(UserOperationPool::new()),
            Arc::new(Provider::try_from("http://164.8.250.25:58545").unwrap()),
            "0x7d695d8c5dd5c71fb4a4d3a81503d6e71a0b3dff"
                .parse()
                .unwrap(),
            U256::from(1500000),
        );

        // Abigen::new("EntryPoint", "$OUT_DIR/IEntryPoint.sol/IEntryPoint.json").unwrap().generate().unwrap().write_to_file("entry_point.rs").unwrap();

        let user_operation_valid = UserOperation {
            sender: "0x751ba0ccc3ad1392e325f8c3b9b197b7bdb61402".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0x").unwrap(),
            call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000e6ac5629b9ade2132f42887fbbc3a3860afbd07b00000000000000000000000000000000000000000000000003782dace9d9000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(1000000),
            verification_gas_limit: U256::from(1100000),
            pre_verification_gas: U256::from(48432),
            max_fee_per_gas: U256::from(3714080682 as u64),
            max_priority_fee_per_gas: U256::from(3390000000 as u64),
            paymaster_and_data: Bytes::from_str("0x").unwrap(),
            signature: Bytes::from_str("0x8689a9b5900a859eeddc7183b5898ecb8bba09a2381f6678f322729f8a4f94237c2ecf3f0849a3edd2fee465f55fb02f3882633cf50162019db18ace342801461c").unwrap(),
        };

        assert_eq!(
            uo_pool_service
                .simulate_user_operation(&user_operation_valid)
                .await
                .unwrap(),
            SimulateValidationResult::SimulationResult(entry_point_api::SimulationResult {
                pre_op_gas: U256::from(118468),
                prefund: U256::from(7417466185066080 as u64),
                deadline: U256::from(0),
                sender_info: (U256::from(0), U256::from(0)),
                factory_info: (U256::from(0), U256::from(0)),
                paymaster_info: (U256::from(0), U256::from(0)),
            })
        );
    }
}
