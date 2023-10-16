use ethers::{
    providers::Middleware,
    types::{Address, H256, U256},
};
use eyre::format_err;
use futures_util::StreamExt;
use silius_contracts::EntryPoint;
use silius_primitives::{
    consts::reputation::{
        BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, MIN_UNSTAKE_DELAY, THROTTLING_SLACK,
    },
    get_address,
    provider::BlockStream,
    reputation::ReputationEntry,
    Chain, UserOperation,
};
use silius_uopool::{
    validate::validator::StandardUserOperationValidator, Mempool, MempoolBox, Reputation,
    ReputationBox, UoPool, VecCh, VecUo,
};
use std::sync::Arc;
use std::{
    fmt::{Debug, Display},
    time::Duration,
};
use tracing::warn;

pub struct UoPoolBuilder<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    is_unsafe: bool,
    eth_client: Arc<M>,
    entrypoint_addr: Address,
    chain: Chain,
    max_verification_gas: U256,
    min_stake: U256,
    min_priority_fee_per_gas: U256,
    whitelist: Vec<Address>,
    mempool: MempoolBox<VecUo, VecCh, P, E>,
    reputation: ReputationBox<Vec<ReputationEntry>, R, E>,
}

impl<M, P, R, E> UoPoolBuilder<M, P, R, E>
where
    M: Middleware + Clone + 'static,
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync + 'static,
    R: Reputation<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync + 'static,
    E: Debug + Display + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        is_unsafe: bool,
        eth_client: Arc<M>,
        entrypoint_addr: Address,
        chain: Chain,
        max_verification_gas: U256,
        min_stake: U256,
        min_priority_fee_per_gas: U256,
        whitelist: Vec<Address>,
        mempool: P,
        reputation: R,
    ) -> Self {
        // sets mempool
        let mempool = MempoolBox::<VecUo, VecCh, P, E>::new(mempool);

        // sets reputation
        let mut reputation = ReputationBox::<Vec<ReputationEntry>, R, E>::new(reputation);
        reputation.init(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            min_stake,
            MIN_UNSTAKE_DELAY.into(),
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
            min_priority_fee_per_gas,
            whitelist,
            mempool,
            reputation,
        }
    }

    async fn handle_block_update(
        hash: H256,
        uopool: &mut UoPool<M, StandardUserOperationValidator<M, P, R, E>, P, R, E>,
    ) -> eyre::Result<()> {
        let txs = uopool
            .entry_point
            .eth_client()
            .get_block_with_txs(hash)
            .await?
            .map(|b| b.transactions);

        if let Some(txs) = txs {
            for tx in txs {
                if tx.to == Some(uopool.entry_point_address()) {
                    let dec: Result<(Vec<UserOperation>, Address), _> = uopool
                        .entry_point
                        .entry_point_api()
                        .decode("handleOps", tx.input);

                    if let Ok((uos, _)) = dec {
                        uopool.remove_user_operations(
                            uos.iter()
                                .map(|uo| {
                                    uo.hash(
                                        &uopool.entry_point_address(),
                                        &uopool.chain.id().into(),
                                    )
                                })
                                .collect(),
                        );

                        for uo in uos {
                            // update reputations
                            uopool
                                .reputation
                                .increment_included(&uo.sender)
                                .map_err(|e| {
                                    format_err!(
                                        "Failed to increment sender reputation: {:?}",
                                        e.to_string()
                                    )
                                })?;

                            if let Some(addr) = get_address(&uo.paymaster_and_data) {
                                uopool.reputation.increment_included(&addr).map_err(|e| {
                                    format_err!(
                                        "Failed to increment paymaster reputation: {:?}",
                                        e.to_string()
                                    )
                                })?;
                            }

                            if let Some(addr) = get_address(&uo.init_code) {
                                uopool.reputation.increment_included(&addr).map_err(|e| {
                                    format_err!(
                                        "Failed to increment factory reputation: {:?}",
                                        e.to_string()
                                    )
                                })?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn register_block_updates(&self, mut block_stream: BlockStream) {
        let mut uopool = self.uopool();
        tokio::spawn(async move {
            while let Some(hash) = block_stream.next().await {
                if let Ok(hash) = hash {
                    let h: H256 = hash;
                    let _ = Self::handle_block_update(h, &mut uopool)
                        .await
                        .map_err(|e| warn!("Failed to handle block update: {:?}", e));
                }
            }
        });
    }

    pub fn register_reputation_updates(&self) {
        let mut uopool = self.uopool();
        tokio::spawn(async move {
            loop {
                let _ = uopool
                    .reputation
                    .update_hourly()
                    .map_err(|e| warn!("Failed to update hourly reputation: {:?}", e));
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });
    }

    pub fn uopool(&self) -> UoPool<M, StandardUserOperationValidator<M, P, R, E>, P, R, E> {
        let entry_point = EntryPoint::<M>::new(self.eth_client.clone(), self.entrypoint_addr);

        let validator = if self.is_unsafe {
            StandardUserOperationValidator::new_canonical_unsafe(
                entry_point.clone(),
                self.chain,
                self.max_verification_gas,
                self.min_priority_fee_per_gas,
            )
        } else {
            StandardUserOperationValidator::new_canonical(
                entry_point.clone(),
                self.chain,
                self.max_verification_gas,
                self.min_priority_fee_per_gas,
            )
        };

        UoPool::<M, StandardUserOperationValidator<M, P, R, E>, P, R, E>::new(
            entry_point,
            validator,
            self.mempool.clone(),
            self.reputation.clone(),
            self.max_verification_gas,
            self.chain,
        )
    }
}
