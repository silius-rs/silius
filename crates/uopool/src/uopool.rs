use crate::{
    canonical::{sanity_check::SanityCheckResult, simulation::SimulationResult},
    mempool::MempoolBox,
    mempool_id,
    reputation::ReputationBox,
    utils::calculate_call_gas_limit,
    MempoolId, Overhead,
};
use aa_bundler_contracts::{
    entry_point::{EntryPointAPIEvents, EntryPointErr, UserOperationEventFilter},
    utils::parse_from_input_data,
    EntryPoint,
};
use aa_bundler_primitives::{
    get_address,
    reputation::{ReputationEntry, ReputationStatus, THROTTLED_MAX_INCLUDE},
    simulation::{CodeHash, SimulationError},
    uopool::{AddError, VerificationError},
    Chain, UoPoolMode, UserOperation, UserOperationByHash, UserOperationGasEstimation,
    UserOperationHash, UserOperationReceipt,
};
use anyhow::format_err;
use ethers::{
    prelude::LogMeta,
    providers::Middleware,
    types::{Address, BlockNumber, U256, U64},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::trace;

type VecUo = Vec<UserOperation>;
type VecCh = Vec<CodeHash>;

const LATEST_SCAN_DEPTH: u64 = 1000;

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    pub sanity_check_result: SanityCheckResult,
    pub simulation_result: SimulationResult,
}

pub struct UoPool<M: Middleware> {
    pub id: MempoolId,
    pub entry_point: EntryPoint<M>,
    pub mempool: MempoolBox<VecUo, VecCh>,
    pub reputation: ReputationBox<Vec<ReputationEntry>>,
    pub eth_provider: Arc<M>,
    pub max_verification_gas: U256,
    pub min_priority_fee_per_gas: U256,
    pub chain: Chain,
    pub mode: UoPoolMode,
}

impl<M: Middleware + 'static> UoPool<M> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entry_point: EntryPoint<M>,
        mempool: MempoolBox<VecUo, VecCh>,
        reputation: ReputationBox<Vec<ReputationEntry>>,
        eth_provider: Arc<M>,
        max_verification_gas: U256,
        min_priority_fee_per_gas: U256,
        chain: Chain,
        mode: UoPoolMode,
    ) -> Self {
        Self {
            id: mempool_id(&entry_point.address(), &chain.id().into()),
            entry_point,
            mempool,
            reputation,
            eth_provider,
            max_verification_gas,
            min_priority_fee_per_gas,
            chain,
            mode,
        }
    }

    pub fn entry_point_address(&self) -> Address {
        self.entry_point.address()
    }

    pub fn get_all(&self) -> Vec<UserOperation> {
        self.mempool.get_all()
    }

    pub fn get_reputation(&self) -> Vec<ReputationEntry> {
        self.reputation.get_all()
    }

    pub fn set_reputation(&mut self, reputation: Vec<ReputationEntry>) {
        self.reputation.set(reputation);
    }

    pub fn clear(&mut self) {
        self.mempool.clear();
        self.reputation.clear();
    }

    pub async fn add_user_operation(
        &mut self,
        uo: UserOperation,
        res: Option<Result<VerificationResult, VerificationError>>,
    ) -> Result<UserOperationHash, AddError> {
        let res = match res {
            Some(res) => res?,
            None => self.verify_user_operation(&uo).await?,
        };

        if let Some(uo_hash) = res.sanity_check_result.user_operation_hash {
            self.remove_user_operation(&uo_hash);
        }

        match self.mempool.add(
            uo.clone(),
            &self.entry_point.address(),
            &self.chain.id().into(),
        ) {
            Ok(uo_hash) => {
                // TODO: find better way to do it atomically
                let _ = self
                    .mempool
                    .set_code_hashes(&uo_hash, &res.simulation_result.code_hashes);

                trace!("User operation {uo:?} added to the mempool {}", self.id);

                // TODO: update reputation
                Ok(uo_hash)
            }
            Err(e) => Err(AddError::MempoolError {
                message: e.to_string(),
            }),
        }
    }

    pub async fn verify_user_operation(
        &self,
        uo: &UserOperation,
    ) -> Result<VerificationResult, VerificationError> {
        // sanity check
        let sanity_check_result = self.check_user_operation(uo).await?;

        // simulation
        let simulation_result = self.simulate_user_operation(uo, true).await?;

        Ok(VerificationResult {
            sanity_check_result,
            simulation_result,
        })
    }

    pub fn get_sorted_user_operations(&self) -> anyhow::Result<Vec<UserOperation>> {
        self.mempool.get_sorted()
    }

    pub async fn bundle_user_operations(
        &mut self,
        uos: Vec<UserOperation>,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let mut uos_valid = vec![];
        let mut senders = HashSet::new();
        let mut gas_total = U256::zero();
        let mut paymaster_dep = HashMap::new();
        let mut staked_entity_c = HashMap::new();

        for uo in uos {
            if senders.contains(&uo.sender) {
                continue;
            }

            let uo_hash = uo.hash(&self.entry_point.address(), &self.chain.id().into());

            let p_opt = get_address(&uo.paymaster_and_data.0);
            let f_opt = get_address(&uo.init_code.0);

            let p_st = self
                .reputation
                .get_status_from_bytes(&uo.paymaster_and_data);
            let f_st = self.reputation.get_status_from_bytes(&uo.init_code);

            let p_c = p_opt
                .map(|p| staked_entity_c.get(&p).cloned().unwrap_or(0))
                .unwrap_or(0);
            let f_c = f_opt
                .map(|f| staked_entity_c.get(&f).cloned().unwrap_or(0))
                .unwrap_or(0);

            match (p_st, f_st) {
                (ReputationStatus::BANNED, _) | (_, ReputationStatus::BANNED) => {
                    self.mempool.remove(&uo_hash).map_err(|err| {
                        format_err!(
                            "Removing a banned user operation {uo_hash:?} failed with error: {err:?}",
                        )
                    })?;
                    continue;
                }
                (ReputationStatus::THROTTLED, _) if p_c > THROTTLED_MAX_INCLUDE => {
                    continue;
                }
                (_, ReputationStatus::THROTTLED) if f_c > THROTTLED_MAX_INCLUDE => {
                    continue;
                }
                _ => (),
            };

            let sim_res = self.simulate_user_operation(&uo, true).await;

            match sim_res {
                Ok(sim_res) => {
                    if sim_res.valid_after.is_some() {
                        continue;
                    }

                    // TODO
                    // it would be better to use estimate_gas instead of call_gas_limit
                    // The result of call_gas_limit is usesally higher and less user op would be included
                    let gas_cost = sim_res
                        .verification_gas_limit
                        .saturating_add(uo.call_gas_limit);
                    let gas_total_new = gas_total.saturating_add(gas_cost);
                    if gas_total_new.gt(&self.max_verification_gas) {
                        break;
                    }

                    if let Some(p) = p_opt {
                        let balance = match paymaster_dep.get(&p) {
                            Some(n) => *n,
                            None => {
                                self.eth_provider.get_balance(p, None)
                                .await.map_err(|err| {
                                    format_err!(
                                        "Getting balance of paymaster {p:?} failed with error: {err:?}",
                                    )
                                })?
                            }
                        };

                        if balance.lt(&sim_res.pre_fund) {
                            continue;
                        }

                        staked_entity_c
                            .entry(p)
                            .and_modify(|c| *c += 1)
                            .or_insert(1);
                        paymaster_dep.insert(p, balance.saturating_sub(sim_res.pre_fund));
                    }

                    if let Some(f) = f_opt {
                        staked_entity_c
                            .entry(f)
                            .and_modify(|c| *c += 1)
                            .or_insert(1);
                    }

                    gas_total = gas_total_new;
                }
                Err(_) => {
                    self.mempool.remove(&uo_hash).map_err(|err| {
                        format_err!(
                            "Removing a user operation {uo_hash:?} with 2nd failed simulation failed with error: {err:?}",
                        )
                    })?;
                    continue;
                }
            }

            uos_valid.push(uo.clone());
            senders.insert(uo.sender);
        }

        Ok(uos_valid)
    }

    pub async fn base_fee_per_gas(&self) -> anyhow::Result<U256> {
        let block = self
            .eth_provider
            .get_block(BlockNumber::Latest)
            .await?
            .ok_or(format_err!("No block found"))?;
        block
            .base_fee_per_gas
            .ok_or(format_err!("No base fee found"))
    }

    pub async fn estimate_user_operation_gas(
        &self,
        uo: &UserOperation,
    ) -> Result<UserOperationGasEstimation, SimulationError> {
        let sim_res = self.simulate_user_operation(uo, false).await?;

        match self.entry_point.simulate_execution(uo.clone()).await {
            Ok(_) => {}
            Err(err) => {
                return Err(match err {
                    EntryPointErr::JsonRpcError(err) => SimulationError::Execution {
                        message: err.message,
                    },
                    _ => SimulationError::UnknownError {
                        message: format!("{err:?}"),
                    },
                })
            }
        }

        let exec_res = match self.entry_point.simulate_handle_op(uo.clone()).await {
            Ok(res) => res,
            Err(err) => {
                return Err(match err {
                    EntryPointErr::JsonRpcError(err) => SimulationError::Execution {
                        message: err.message,
                    },
                    _ => SimulationError::UnknownError {
                        message: format!("{err:?}"),
                    },
                })
            }
        };

        let base_fee_per_gas =
            self.base_fee_per_gas()
                .await
                .map_err(|err| SimulationError::UnknownError {
                    message: err.to_string(),
                })?;
        let call_gas_limit = calculate_call_gas_limit(
            exec_res.paid,
            exec_res.pre_op_gas,
            uo.max_fee_per_gas
                .min(uo.max_priority_fee_per_gas + base_fee_per_gas),
        );

        Ok(UserOperationGasEstimation {
            pre_verification_gas: Overhead::default().calculate_pre_verification_gas(uo),
            verification_gas_limit: sim_res.verification_gas_limit,
            call_gas_limit,
        })
    }

    pub async fn get_user_operation_event_meta(
        &self,
        uo_hash: &UserOperationHash,
    ) -> anyhow::Result<Option<(UserOperationEventFilter, LogMeta)>> {
        let mut event: Option<(UserOperationEventFilter, LogMeta)> = None;
        let filter = self
            .entry_point
            .entry_point_api()
            .event::<UserOperationEventFilter>()
            .topic1(uo_hash.0);
        let res: Vec<(UserOperationEventFilter, LogMeta)> = filter.query_with_meta().await?;
        // It is possible have two same user operatation in same bundle
        // see https://twitter.com/leekt216/status/1636414866662785024
        for log_meta in res.iter() {
            event = Some(log_meta.clone());
        }
        Ok(event)
    }

    pub async fn get_user_operation_by_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> anyhow::Result<UserOperationByHash> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some((uo, ep)) = self
                .eth_provider
                .get_transaction(log_meta.transaction_hash)
                .await?
                .and_then(|tx| {
                    let uos = parse_from_input_data(tx.input)?;
                    let ep = tx.to?;
                    uos.iter()
                        .find(|uo| uo.sender == event.sender && uo.nonce == event.nonce)
                        .map(|uo| (uo.clone(), ep))
                })
            {
                return Ok(UserOperationByHash {
                    user_operation: uo,
                    entry_point: ep,
                    transaction_hash: log_meta.transaction_hash,
                    block_hash: log_meta.block_hash,
                    block_number: log_meta.block_number,
                });
            }
        }

        Err(format_err!("No user operation found"))
    }

    pub async fn get_user_operation_receipt(
        &self,
        uo_hash: &UserOperationHash,
    ) -> anyhow::Result<UserOperationReceipt> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some(tx_receipt) = self
                .eth_provider
                .get_transaction_receipt(log_meta.transaction_hash)
                .await?
            {
                let uo = self.get_user_operation_by_hash(uo_hash).await?;
                return Ok(UserOperationReceipt {
                    user_operation_hash: *uo_hash,
                    sender: event.sender,
                    nonce: event.nonce,
                    actual_gas_cost: event.actual_gas_cost,
                    actual_gas_used: event.actual_gas_used,
                    success: event.success,
                    tx_receipt: tx_receipt.clone(),
                    logs: tx_receipt.logs.into_iter().collect(),
                    paymaster: get_address(&uo.user_operation.paymaster_and_data),
                    reason: String::new(), // TODO: this must be set to revert reason
                });
            }
        }

        Err(format_err!("No user operation found"))
    }

    pub async fn handle_past_events(&mut self) -> anyhow::Result<()> {
        let block_num = self.eth_provider.get_block_number().await?;
        let block_st = std::cmp::max(
            1u64,
            block_num
                .checked_sub(U64::from(LATEST_SCAN_DEPTH))
                .unwrap_or(U64::from(0))
                .as_u64(),
        );

        let filter = self.entry_point.events().from_block(block_st);
        let events = filter.query().await?;

        for event in events {
            match event {
                EntryPointAPIEvents::UserOperationEventFilter(uo_event) => {
                    self.remove_user_operation(&uo_event.user_op_hash.into());
                    self.include_address(uo_event.sender);
                    self.include_address(uo_event.paymaster);
                    // TODO: include event aggregator
                }
                EntryPointAPIEvents::AccountDeployedFilter(event) => {
                    self.include_address(event.factory);
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn include_address(&mut self, addr: Address) -> Option<()> {
        self.reputation.increment_included(&addr);
        Some(())
    }

    pub fn remove_user_operation(&mut self, uo_hash: &UserOperationHash) -> Option<()> {
        self.mempool.remove(uo_hash).ok();
        None
    }

    pub fn remove_user_operations(&mut self, uo_hashes: Vec<UserOperationHash>) {
        for uo_hash in uo_hashes {
            self.remove_user_operation(&uo_hash);
        }
    }
}
