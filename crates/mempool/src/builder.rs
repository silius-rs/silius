use crate::{
    validate::{
        validator::StandardUserOperationValidator, SanityCheck, SimulationCheck,
        SimulationTraceCheck,
    },
    Mempool, Reputation, UoPool,
};
use alloy_chains::Chain;
use ethers::{
    providers::Middleware,
    types::{Address, H256, U256},
};
use futures::channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
use silius_contracts::EntryPoint;
use silius_primitives::{provider::BlockStream, UoPoolMode, UserOperation, UserOperationSigned};
use std::{sync::Arc, time::Duration};
use tracing::warn;

type StandardUoPool<M, SanCk, SimCk, SimTrCk> =
    UoPool<M, StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>>;
pub struct UoPoolBuilder<M, SanCk, SimCk, SimTrCk>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M>,
    SimCk: SimulationCheck,
    SimTrCk: SimulationTraceCheck<M>,
{
    mode: UoPoolMode,
    eth_client: Arc<M>,
    entrypoint_addr: Address,
    chain: Chain,
    max_verification_gas: U256,
    mempool: Mempool,
    reputation: Reputation,
    validator: StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>,
    // It would be None if p2p is not enabled
    publish_sd: Option<UnboundedSender<(UserOperation, U256)>>,
}

impl<M, SanCk, SimCk, SimTrCk> UoPoolBuilder<M, SanCk, SimCk, SimTrCk>
where
    M: Middleware + Clone + 'static,
    SanCk: SanityCheck<M> + Clone + 'static,
    SimCk: SimulationCheck + Clone + 'static,
    SimTrCk: SimulationTraceCheck<M> + Clone + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: UoPoolMode,
        eth_client: Arc<M>,
        entrypoint_addr: Address,
        chain: Chain,
        max_verification_gas: U256,
        mempool: Mempool,
        reputation: Reputation,
        validator: StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>,
        publish_sd: Option<UnboundedSender<(UserOperation, U256)>>,
    ) -> Self {
        Self {
            mode,
            eth_client,
            entrypoint_addr,
            chain,
            max_verification_gas,
            mempool,
            reputation,
            validator,
            publish_sd,
        }
    }

    async fn handle_block_update(
        hash: H256,
        uopool: &mut StandardUoPool<M, SanCk, SimCk, SimTrCk>,
    ) -> eyre::Result<()> {
        let txs =
            uopool.entry_point.eth_client().get_block_with_txs(hash).await?.map(|b| b.transactions);

        if let Some(txs) = txs {
            for tx in txs {
                if tx.to == Some(uopool.entry_point.address()) {
                    let dec: Result<(Vec<UserOperationSigned>, Address), _> =
                        uopool.entry_point.entry_point_api().decode("handleOps", tx.input);

                    if let Ok((uos, _)) = dec {
                        uopool.remove_user_operations(
                            uos.iter()
                                .map(|uo| {
                                    UserOperation::from_user_operation_signed(
                                        uo.hash(&uopool.entry_point.address(), uopool.chain.id()),
                                        uo.clone(),
                                    )
                                })
                                .collect(),
                        );
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

    pub fn uopool(&self) -> StandardUoPool<M, SanCk, SimCk, SimTrCk> {
        let entry_point = EntryPoint::<M>::new(self.eth_client.clone(), self.entrypoint_addr);

        UoPool::<M, StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>>::new(
            self.mode,
            entry_point,
            self.validator.clone(),
            self.mempool.clone(),
            self.reputation.clone(),
            self.max_verification_gas,
            self.chain,
            self.publish_sd.as_ref().cloned(),
        )
    }
}
