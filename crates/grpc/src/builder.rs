use std::sync::Arc;

use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use silius_contracts::EntryPoint;
use silius_primitives::{
    reputation::{ReputationEntry, BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK},
    Chain,
};
use silius_uopool::{
    validate::validator::StandardUserOperationValidator, Mempool, MempoolBox, Reputation,
    ReputationBox, UoPool, VecCh, VecUo,
};

use crate::uopool::{GAS_INCREASE_PERC, MAX_UOS_PER_UNSTAKED_SENDER};

pub struct UoPoolBuilder<M, P, R>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    is_unsafe: bool,
    eth_client: Arc<M>,
    entrypoint_addr: Address,
    chain: Chain,
    max_verification_gas: U256,
    min_stake: U256,
    min_unstake_delay: U256,
    min_priority_fee_per_gas: U256,
    whitelist: Vec<Address>,
    mempool: MempoolBox<VecUo, VecCh, P>,
    reputation: ReputationBox<Vec<ReputationEntry>, R>,
}

impl<M, P, R> UoPoolBuilder<M, P, R>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = anyhow::Error> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = anyhow::Error> + Send + Sync,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        is_unsafe: bool,
        eth_client: Arc<M>,
        entrypoint_addr: Address,
        chain: Chain,
        max_verification_gas: U256,
        min_stake: U256,
        min_unstake_delay: U256,
        min_priority_fee_per_gas: U256,
        whitelist: Vec<Address>,
        mempool: P,
        reputation: R,
    ) -> Self {
        let mempool = MempoolBox::<VecUo, VecCh, P>::new(mempool);
        let mut reputation = ReputationBox::<Vec<ReputationEntry>, R>::new(reputation);
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            min_stake,
            min_unstake_delay,
        );
        for addr in whitelist.iter() {
            reputation.add_whitelist(addr);
        }
        Self {
            is_unsafe,
            eth_client,
            entrypoint_addr,
            chain,
            max_verification_gas,
            min_stake,
            min_unstake_delay,
            min_priority_fee_per_gas,
            whitelist,
            mempool,
            reputation,
        }
    }

    pub fn uo_pool(&self) -> UoPool<M, StandardUserOperationValidator<M, P, R>, P, R> {
        let entry_point = EntryPoint::<M>::new(self.eth_client.clone(), self.entrypoint_addr);

        let validator = if self.is_unsafe {
            StandardUserOperationValidator::new_canonical_unsafe(
                entry_point.clone(),
                self.chain,
                self.max_verification_gas,
                self.min_priority_fee_per_gas,
                MAX_UOS_PER_UNSTAKED_SENDER,
                GAS_INCREASE_PERC.into(),
            )
        } else {
            StandardUserOperationValidator::new_canonical(
                entry_point.clone(),
                self.chain,
                self.max_verification_gas,
                self.min_priority_fee_per_gas,
                MAX_UOS_PER_UNSTAKED_SENDER,
                GAS_INCREASE_PERC.into(),
            )
        };

        UoPool::<M, StandardUserOperationValidator<M, P, R>, P, R>::new(
            entry_point,
            validator,
            self.mempool.clone(),
            self.reputation.clone(),
            self.max_verification_gas,
            self.chain,
        )
    }
}
