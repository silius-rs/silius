use std::sync::Arc;

use aa_bundler_contracts::{EntryPoint, UserOperationEventFilter};
use aa_bundler_primitives::{CodeHash, ReputationEntry, UserOperation, UserOperationHash};
use ethers::{
    prelude::LogMeta,
    providers::Middleware,
    types::{Address, H256, U256},
};
use jsonrpsee::types::ErrorObject;
use tracing::warn;

use crate::{
    canonical::{sanity_check::SanityCheckResult, simulation::SimulationResult},
    mempool::MempoolBox,
    reputation::ReputationBox,
};

type VecUo = Vec<UserOperation>;
type VecCh = Vec<CodeHash>;

#[derive(Debug)]
pub struct VerificationResult {
    pub sanity_check_result: SanityCheckResult,
    pub simulation_result: SimulationResult,
}

pub struct UoPool<M: Middleware> {
    pub entry_point: EntryPoint<M>,
    pub mempool: MempoolBox<VecUo, VecCh>,
    pub reputation: ReputationBox<Vec<ReputationEntry>>,
    pub eth_provider: Arc<M>,
    pub max_verification_gas: U256,
    pub min_priority_fee_per_gas: U256,
    pub chain_id: U256,
}

impl<M: Middleware + 'static> UoPool<M> {
    pub fn new(
        entry_point: EntryPoint<M>,
        mempool: MempoolBox<VecUo, VecCh>,
        reputation: ReputationBox<Vec<ReputationEntry>>,
        eth_provider: Arc<M>,
        max_verification_gas: U256,
        min_priority_fee_per_gas: U256,
        chain_id: U256,
    ) -> Self {
        Self {
            entry_point,
            mempool,
            reputation,
            eth_provider,
            max_verification_gas,
            min_priority_fee_per_gas,
            chain_id,
        }
    }

    pub async fn verify_user_operation(
        &self,
        user_operation: &UserOperation,
    ) -> Result<VerificationResult, ErrorObject<'static>> {
        // sanity check
        let sanity_check_result = self.validate_user_operation(user_operation).await?;

        // simulation
        let simulation_result = self.simulate_user_operation(user_operation).await?;

        Ok(VerificationResult {
            sanity_check_result,
            simulation_result,
        })
    }

    pub async fn get_user_operation_event_meta(
        &self,
        user_operation_hash: H256,
    ) -> anyhow::Result<Option<(UserOperationEventFilter, LogMeta)>> {
        let mut event: Option<(UserOperationEventFilter, LogMeta)> = None;
        let filter = self
            .entry_point
            .entry_point_api()
            .event::<UserOperationEventFilter>()
            .topic1(user_operation_hash);
        let res: Vec<(UserOperationEventFilter, LogMeta)> = filter.query_with_meta().await?;
        if res.len() >= 2 {
            warn!(
                "There are duplicate user operations with the same hash: {user_operation_hash:x?}"
            );
        }
        // It is possible have two same user operatation in same bundle
        // see https://twitter.com/leekt216/status/1636414866662785024
        for log_meta in res.iter() {
            event = Some(log_meta.clone());
        }
        Ok(event)
    }

    pub fn include_address(&mut self, addr: Address) -> Option<()> {
        self.reputation.increment_included(&addr);
        Some(())
    }

    pub fn remove_user_operation(&mut self, user_operation_hash: &UserOperationHash) -> Option<()> {
        self.mempool.remove(user_operation_hash).ok();
        None
    }
}
