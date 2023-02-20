use ethers::{
    providers::Middleware,
    types::{Address, GethTrace},
};

use crate::{
    contracts::{tracer::JsTracerFrame, EntryPointErr, SimulateValidationResult},
    types::{simulation::SimulateValidationError, user_operation::UserOperation},
    uopool::mempool_id,
};

use super::UoPoolService;

impl<M: Middleware + 'static> UoPoolService<M>
where
    EntryPointErr<M>: From<<M as Middleware>::Error>,
{
    async fn simulate_validation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<SimulateValidationResult, SimulateValidationError<M>> {
        let mempool_id = mempool_id(entry_point, &self.chain_id);

        if let Some(entry_point) = self.entry_points.get(&mempool_id) {
            return match entry_point
                .simulate_validation(user_operation.clone())
                .await
            {
                Ok(simulate_validation_result) => Ok(simulate_validation_result),
                Err(entry_point_error) => match entry_point_error {
                    EntryPointErr::MiddlewareErr(middleware_error) => {
                        Err(SimulateValidationError::Middleware(middleware_error))
                    }
                    EntryPointErr::FailedOp(failed_op) => {
                        Err(SimulateValidationError::UserOperationRejected {
                            message: format!("{failed_op}"),
                        })
                    }
                    _ => Err(SimulateValidationError::UserOperationRejected {
                        message: "unknown error".to_string(),
                    }),
                },
            };
        }

        Err(SimulateValidationError::UserOperationRejected {
            message: "invalid entry point".to_string(),
        })
    }

    async fn simulate_validation_trace(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<GethTrace, SimulateValidationError<M>> {
        let mempool_id = mempool_id(entry_point, &self.chain_id);

        if let Some(entry_point) = self.entry_points.get(&mempool_id) {
            return match entry_point
                .simulate_validation_trace(user_operation.clone())
                .await
            {
                Ok(geth_trace) => Ok(geth_trace),
                Err(entry_point_error) => match entry_point_error {
                    EntryPointErr::MiddlewareErr(middleware_error) => {
                        Err(SimulateValidationError::Middleware(middleware_error))
                    }
                    EntryPointErr::FailedOp(failed_op) => {
                        Err(SimulateValidationError::UserOperationRejected {
                            message: format!("{failed_op}"),
                        })
                    }
                    _ => Err(SimulateValidationError::UserOperationRejected {
                        message: "unknown error".to_string(),
                    }),
                },
            };
        }

        Err(SimulateValidationError::UserOperationRejected {
            message: "invalid entry point".to_string(),
        })
    }

    async fn forbidden_opcodes(
        &self,
        simulate_validation_result: &SimulateValidationResult,
        trace: &JsTracerFrame,
    ) -> Result<(), SimulateValidationError<M>> {
        println!("simulate_validation_result: {simulate_validation_result:?}");
        println!("trace: {trace:?}");
        Ok(())
    }

    pub async fn simulate_user_operation(
        &self,
        user_operation: &UserOperation,
        entry_point: &Address,
    ) -> Result<(), SimulateValidationError<M>> {
        let simulate_validation_result = self
            .simulate_validation(user_operation, entry_point)
            .await?;

        let geth_trace = self
            .simulate_validation_trace(user_operation, entry_point)
            .await?;

        let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace).map_err(|error| {
            SimulateValidationError::UserOperationRejected {
                message: error.to_string(),
            }
        })?;

        // may not invokes any forbidden opcodes
        self.forbidden_opcodes(&simulate_validation_result, &js_trace)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use ethers::{
        providers::{Http, Provider},
        types::{Bytes, U256},
    };
    use parking_lot::RwLock;

    use crate::{
        contracts::EntryPoint,
        types::reputation::{
            ReputationEntry, BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
        },
        uopool::{
            memory_mempool::MemoryMempool, memory_reputation::MemoryReputation, MempoolBox,
            MempoolId, ReputationBox,
        },
    };

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn simulate_validation_trace() {
        let chain_id = U256::from(1337);
        let entry_point = "0x1306b01bC3e4AD202612D3843387e94737673F53"
            .parse::<Address>()
            .unwrap();
        let eth_provider = Arc::new(Provider::try_from("http://127.0.0.1:8545").unwrap());

        let mut entry_points_map = HashMap::<MempoolId, EntryPoint<Provider<Http>>>::new();
        let mut mempools = HashMap::<MempoolId, MempoolBox<Vec<UserOperation>>>::new();
        let mut reputations = HashMap::<MempoolId, ReputationBox<Vec<ReputationEntry>>>::new();

        let m_id = mempool_id(&entry_point, &chain_id);
        mempools.insert(m_id, Box::<MemoryMempool>::default());

        reputations.insert(m_id, Box::<MemoryReputation>::default());
        if let Some(reputation) = reputations.get_mut(&m_id) {
            reputation.init(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                U256::from(0),
                U256::from(0),
            );
        }
        entry_points_map.insert(
            m_id,
            EntryPoint::<Provider<Http>>::new(eth_provider.clone(), entry_point),
        );

        let uo_pool_service = UoPoolService::new(
            Arc::new(entry_points_map),
            Arc::new(RwLock::new(mempools)),
            Arc::new(RwLock::new(reputations)),
            eth_provider,
            U256::from(1500000),
            U256::from(2),
            chain_id,
        );

        let user_operation = UserOperation {
            sender: "0xBBe6a3230Ef8abC44EF61B3fBf93Cd0394D1d21f".parse().unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::from_str("0xed886f2d1bbb38b4914e8c545471216a40cce9385fbfb9cf000000000000000000000000ae72a48c1a36bd18af168541c53037965d26e4a80000000000000000000000000000000000000000000000000000018661be6ed7").unwrap(),
            call_data: Bytes::from_str("0xb61d27f6000000000000000000000000bbe6a3230ef8abc44ef61b3fbf93cd0394d1d21f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000004affed0e000000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: U256::from(22016),
            verification_gas_limit: U256::from(413910),
            pre_verification_gas: U256::from(48480),
            max_fee_per_gas: U256::from(2000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::from_str("0xeb99f2f72c16b3eb5bdeadb243dd38a6e54771f1dd9b3d1d08e99e3e0840717331e6c8c83457c6c33daa3aa30a238197dbf7ea1f17d02aa57c3fa9e9ce3dc1731c").unwrap(),
        };

        assert!(uo_pool_service
            .simulate_user_operation(&user_operation, &entry_point)
            .await
            .is_ok());
    }
}
