use crate::{
    contracts::{gen::entry_point_api, EntryPoint, EntryPointErr, SimulateValidationResult, tracer::JsTracerFrame},
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
    ) -> Result</*SimulateValidationResult*/ (), UserOperationSimulationError> {
        // call to simulate validation, save result, needed to return data (deadline ...)
        // let simulation_result = self
        //     .entry_point_simulate_validation(user_operation, entry_point)
        //     .await?;
        // this returns simulate validation trace that needs to be analyzed
        // let simulate_validation_trace = self.entry_point_simulate_validation_trace(user_operation, entry_point)
        //     .await?;

        let simulate_validation_trace = serde_json::from_str::<JsTracerFrame>(r#"{"numberLevels":[{"access":{},"opcodes":{},"contractSize":{}},{"access":{"0xaa329079de62271962a1d79a0f373b7c5c1bf578":{"reads":{"0":2,"1":1,"360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc":2},"writes":{"0":1}},"0x1306b01bc3e4ad202612d3843387e94737673f53":{"reads":{"dc0c2bb49df36cd45b0446a71b1ae7c0d8ae97eb7ac09776c92e34c70b7cba02":3},"writes":{"dc0c2bb49df36cd45b0446a71b1ae7c0d8ae97eb7ac09776c92e34c70b7cba02":1}}},"opcodes":{"MSTORE":24,"CALLDATASIZE":15,"JUMPI":42,"JUMPDEST":81,"JUMP":45,"SLOAD":8,"CALLDATACOPY":3,"DELEGATECALL":2,"CALLDATALOAD":11,"REVERT":2,"RETURNDATASIZE":5,"RETURNDATACOPY":2,"CALLVALUE":2,"CALLER":3,"MLOAD":16,"KECCAK256":3,"BYTE":1,"STATICCALL":1,"EXP":1,"SSTORE":2,"CALL":1,"LOG2":1,"STOP":1,"RETURN":2},"contractSize":{"0xa366f3a6fc3a6e11b6cc0aef809f7b626eeff8bc":8082,"0x0000000000000000000000000000000000000001":0,"0x1306b01bc3e4ad202612d3843387e94737673f53":23424}},{"access":{},"opcodes":{},"contractSize":{}}],"keccak":["0x19457468657265756d205369676e6564204d6573736167653a0a33324d0d581beff61963810b1eb66f12c07896c41583eac3171960156fdb38582f50","0x000000000000000000000000aa329079de62271962a1d79a0f373b7c5c1bf5780000000000000000000000000000000000000000000000000000000000000000","0x000000000000000000000000aa329079de62271962a1d79a0f373b7c5c1bf5780000000000000000000000000000000000000000000000000000000000000000"],"logs":[{"topics":["0x2da466a7b24304f47e87fa2e1e5a81b9831ce54fec19055ce277ca2f39ba42c4","0xaa329079de62271962a1d79a0f373b7c5c1bf578"],"data":"0x0000000000000000000000000000000000000000000000000001164db93c605c"}],"calls":[{"type":"STATICCALL","from":"0x1306b01bc3e4ad202612d3843387e94737673f53","to":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","method":"0x3ad59dbc","gas":49187594},{"type":"DELEGATECALL","from":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","to":"0xa366f3a6fc3a6e11b6cc0aef809f7b626eeff8bc","method":"0x3ad59dbc","gas":48414272},{"type":"REVERT","gasUsed":169,"data":"0x"},{"type":"REVERT","gasUsed":5050,"data":"0x"},{"type":"CALL","from":"0x1306b01bc3e4ad202612d3843387e94737673f53","to":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","method":"0x0825d1fc","gas":100000,"value":"0"},{"type":"DELEGATECALL","from":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","to":"0xa366f3a6fc3a6e11b6cc0aef809f7b626eeff8bc","method":"0x0825d1fc","gas":93511},{"type":"STATICCALL","from":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","to":"0x0000000000000000000000000000000000000001","method":"0xf2fe301c","gas":90563},{"type":"RETURN","gasUsed":3000,"data":"0x000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a8"},{"type":"CALL","from":"0xaa329079de62271962a1d79a0f373b7c5c1bf578","to":"0x1306b01bc3e4ad202612d3843387e94737673f53","method":"0x","gas":72942,"value":"145988743014268"},{"type":"RETURN","gasUsed":5004,"data":"0x"},{"type":"RETURN","gasUsed":24573,"data":"0x0000000000000000000000000000000000000000000000000000000000000000"},{"type":"RETURN","gasUsed":29619,"data":"0x0000000000000000000000000000000000000000000000000000000000000000"},{"type":"REVERT","gasUsed":0,"data":"0x3dd956e900000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000166780000000000000000000000000000000000000000000000000001164db93c605c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000000"}],"debug":["enter gas=49187594 type=STATICCALL to=0xaa329079de62271962a1d79a0f373b7c5c1bf578 in=0x3ad59dbc","enter gas=48414272 type=DELEGATECALL to=0xa366f3a6fc3a6e11b6cc0aef809f7b626eeff8bc in=0x3ad59dbc","fault depth=3 gas=48414103 cost=0 err=execution reverted","fault depth=2 gas=49182544 cost=0 err=execution reverted","enter gas=100000 type=CALL to=0xaa329079de62271962a1d79a0f373b7c5c1bf578 in=0x0825d1fc00000000000000000000000000000000000000000000000000000000000000804d0d581beff61963810b1eb66f12c07896c41583eac3171960156fdb38582f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000084c6a72b277c000000000000000000000000aa329079de62271962a1d79a0f373b7c5c1bf57800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000","enter gas=93511 type=DELEGATECALL to=0xa366f3a6fc3a6e11b6cc0aef809f7b626eeff8bc in=0x0825d1fc00000000000000000000000000000000000000000000000000000000000000804d0d581beff61963810b1eb66f12c07896c41583eac3171960156fdb38582f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000084c6a72b277c000000000000000000000000aa329079de62271962a1d79a0f373b7c5c1bf57800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000","enter gas=90563 type=STATICCALL to=0x0000000000000000000000000000000000000001 in=0xf2fe301c496ca137c07c906470e8df54803b51d46e9ec273073fabd1f336897d000000000000000000000000000000000000000000000000000000000000001c466c761a2e6474ea5f2e30f1b9d5adafcdabb6fa84d14bb66c89e01582775b4001df839ddb8f40f0bc41094f1ed0a252fd168c30083995da50f5471b6cf7bada","enter gas=72942 type=CALL to=0x1306b01bc3e4ad202612d3843387e94737673f53 in=0x","REVERT 0x3dd956e900000000000000000000000000000000000000000000000000000000000000e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000166780000000000000000000000000000000000000000000000000001164db93c605c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000000","fault depth=1 gas=49920001 cost=0 err=execution reverted"]}"#).unwrap();
        println!("{:?}", simulate_validation_trace);

        // check if ended with revert

        // check not use forbidden opcodes

        // GAS opcode condition

        // check storage access

        // limitation on call opcodes

        // extcodehash does not change (this simulation trace will probably have to be saved somewhere)

        // three things may not access address with no code

        // allow only one create2

        // if everything good, handle first simulation result
        // Ok(simulation_result)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::uopool::{mempool_id, MempoolBox, MempoolId};
    use ethers::{
        providers::{Http, Provider},
        types::{Address, Bytes, U256},
    };
    use parking_lot::RwLock;
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use super::*;

    #[tokio::test]
    async fn user_operation_simulation() {
        let chain_id = U256::from(5);
        let entry_point = "0x1306b01bC3e4AD202612D3843387e94737673F53"
            .parse::<Address>()
            .unwrap();
        let eth_provider = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());
        let mut entry_points = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
        entry_points.insert(
            mempool_id(entry_point, chain_id),
            EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
        );

        let uo_pool_service = UoPoolService::new(
            Arc::new(RwLock::new(HashMap::<
                MempoolId,
                MempoolBox<Vec<UserOperation>>,
            >::new())),
            Arc::new(entry_points),
            eth_provider,
            U256::from(1500000),
            chain_id,
        );

        let user_operation_valid = UserOperation {
            sender: "0x1Dab64b6033009880BDbfA8BCda9c6eb740CeF63".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a800000000000000000000000000000000000000000000000000000185d9d4e2eb").unwrap(),
            call_data: Bytes::from_str("0xb61d27f60000000000000000000000001dab64b6033009880bdbfa8bcda9c6eb740cef63000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(22016),
            verification_gas_limit: U256::from(413910),
            pre_verification_gas: U256::from(48480),
            max_fee_per_gas: U256::from(2068315760 as u64),
            max_priority_fee_per_gas: U256::from(1500000000 as u64),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xd4e74029e82a148b7cdee8783b6886e44c552d4921aa2561ed57c77ab6e5578605bc9ed6c7a0f508bdba258726cc90861ae723443cb97f2ad48e8709373dbda31b").unwrap(),
        };

        assert_eq!(
            uo_pool_service
                .simulate_user_operation(
                    &user_operation_valid,
                    uo_pool_service
                        .entry_points
                        .get(&mempool_id(entry_point, chain_id))
                        .unwrap()
                )
                .await
                .unwrap(),
            ()
            // SimulateValidationResult::ValidationResult(entry_point_api::ValidationResult {
            //     return_info: (U256::from(0), U256::from(0), false, 0, 0, Bytes::default()),
            //     sender_info: (U256::from(0), U256::from(0)),
            //     factory_info: (U256::from(0), U256::from(0)),
            //     paymaster_info: (U256::from(0), U256::from(0)),
            // })
        );
    }
}
