use crate::{
    contracts::{gen::entry_point_api, EntryPoint, EntryPointErr, SimulateValidationResult},
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
    Simulation(BadUserOperationError),
    EntryPoint(EntryPointErr),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for UserOperationSimulationError {
    fn from(e: anyhow::Error) -> Self {
        UserOperationSimulationError::Internal(e)
    }
}

impl From<EntryPointErr> for UserOperationSimulationError {
    fn from(e: EntryPointErr) -> Self {
        UserOperationSimulationError::EntryPoint(e)
    }
}

impl<M: Middleware + 'static> UoPoolService<M> {
    async fn entry_point_simulate_validation(
        &self,
        user_operation: &UserOperation,
        entry_point: &EntryPoint<M>,
    ) -> Result<SimulateValidationResult, UserOperationSimulationError> {
        // View call to simulateValidation(userop)
        let simulation_response = entry_point
            .simulate_validation(entry_point_api::UserOperation::from(user_operation.clone()))
            .await?;
        Ok(simulation_response)
    }

    async fn entry_point_simulate_validation_trace(
        &self,
        user_operation: &UserOperation,
        entry_point: &EntryPoint<M>,
    ) -> Result<(), UserOperationSimulationError> {
        let simulation_trace = entry_point
            .simulate_validation_trace(entry_point_api::UserOperation::from(user_operation.clone()))
            .await?;
        println!("{simulation_trace:?}");
        Ok(())
    }

    async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
        entry_point: &EntryPoint<M>,
    ) -> Result<SimulateValidationResult, UserOperationSimulationError> {
        let simulation_result = self
            .entry_point_simulate_validation(user_operation, entry_point)
            .await?;
        self.entry_point_simulate_validation_trace(user_operation, entry_point)
            .await?;
        Ok(simulation_result)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::uopool::{mempool_id, MempoolBox, MempoolId};
//     use ethers::{
//         providers::{Http, Provider},
//         types::{Address, Bytes, U256},
//     };
//     use parking_lot::RwLock;
//     use std::{collections::HashMap, str::FromStr, sync::Arc};

//     use super::*;

//     #[tokio::test]
//     async fn user_operation_simulation() {
//         let chain_id = U256::from(5);
//         let entry_point = "0x1D9a2CB3638C2FC8bF9C01D088B79E75CD188b17"
//             .parse::<Address>()
//             .unwrap();
//         let eth_provider =
//             Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
//         let mut entry_points = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
//         entry_points.insert(
//             mempool_id(entry_point, chain_id),
//             EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
//         );

//         let uo_pool_service = UoPoolService::new(
//             Arc::new(RwLock::new(HashMap::<
//                 MempoolId,
//                 MempoolBox<Vec<UserOperation>>,
//             >::new())),
//             Arc::new(entry_points),
//             eth_provider,
//             U256::from(1500000),
//             chain_id,
//         );

//         let user_operation_valid = UserOperation {
//             sender: "0x751ba0ccc3ad1392e325f8c3b9b197b7bdb61402".parse().unwrap(),
//             nonce: U256::zero(),
//             init_code: Bytes::from_str("0x").unwrap(),
//             call_data: Bytes::from_str("0x80c5c7d0000000000000000000000000e6ac5629b9ade2132f42887fbbc3a3860afbd07b00000000000000000000000000000000000000000000000003782dace9d9000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000").unwrap(),
//             call_gas_limit: U256::from(1000000),
//             verification_gas_limit: U256::from(1100000),
//             pre_verification_gas: U256::from(48432),
//             max_fee_per_gas: U256::from(3714080682 as u64),
//             max_priority_fee_per_gas: U256::from(3390000000 as u64),
//             paymaster_and_data: Bytes::from_str("0x").unwrap(),
//             signature: Bytes::from_str("0x8689a9b5900a859eeddc7183b5898ecb8bba09a2381f6678f322729f8a4f94237c2ecf3f0849a3edd2fee465f55fb02f3882633cf50162019db18ace342801461c").unwrap(),
//         };

//         assert_eq!(
//             uo_pool_service
//                 .simulate_user_operation(
//                     &user_operation_valid,
//                     uo_pool_service
//                         .entry_points
//                         .get(&mempool_id(entry_point, chain_id))
//                         .unwrap()
//                 )
//                 .await
//                 .unwrap(),
//             SimulateValidationResult::SimulationResult(entry_point_api::SimulationResult {
//                 pre_op_gas: U256::from(118468),
//                 prefund: U256::from(7417466185066080 as u64),
//                 deadline: U256::from(0),
//                 sender_info: (U256::from(0), U256::from(0)),
//                 factory_info: (U256::from(0), U256::from(0)),
//                 paymaster_info: (U256::from(0), U256::from(0)),
//             })
//         );
//     }
// }
