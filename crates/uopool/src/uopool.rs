use crate::{
    mempool::MempoolBox,
    mempool_id,
    reputation::ReputationBox,
    utils::calculate_call_gas_limit,
    validate::{
        UserOperationValidationOutcome, UserOperationValidator, UserOperationValidatorMode,
    },
    MempoolId, Overhead,
};
use anyhow::format_err;
use ethers::{
    prelude::LogMeta,
    providers::Middleware,
    types::{Address, BlockNumber, U256, U64},
};
use silius_contracts::{
    entry_point::{EntryPointAPIEvents, EntryPointErr, UserOperationEventFilter},
    utils::parse_from_input_data,
    EntryPoint,
};
use silius_primitives::{
    get_address,
    reputation::{ReputationEntry, Status, THROTTLED_MAX_INCLUDE},
    simulation::{CodeHash, SimulationCheckError},
    uopool::{AddError, ValidationError},
    Chain, UserOperation, UserOperationByHash, UserOperationGasEstimation, UserOperationHash,
    UserOperationReceipt,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::trace;

pub type VecUo = Vec<UserOperation>;
pub type VecCh = Vec<CodeHash>;

const LATEST_SCAN_DEPTH: u64 = 1000;

/// The alternative mempool pool implementation that provides functionalities to add, remove, validate, and serves data requests from the [RPC API](EthApiServer).
/// Architecturally, the [UoPool](UoPool) is the backend service managed by the [UoPoolService](UoPoolService) and serves requests from the [RPC API](EthApiServer).
pub struct UoPool<M: Middleware + 'static, V: UserOperationValidator> {
    /// The unique ID of the mempool
    pub id: MempoolId,
    /// The [EntryPoint](EntryPoint) contract object
    pub entry_point: EntryPoint<M>,
    /// The [UserOperationValidator](UserOperationValidator) object
    pub validator: V,
    /// The [MempoolBox](MempoolBox) is a [Boxed pointer](https://doc.rust-lang.org/std/boxed/struct.Box.html) to a [Mempool](Mempool) object
    pub mempool: MempoolBox<VecUo, VecCh>,
    /// The [ReputationBox](ReputationBox) is a [Boxed pointer](https://doc.rust-lang.org/std/boxed/struct.Box.html) to a [ReputationEntry](ReputationEntry) object
    pub reputation: ReputationBox<Vec<ReputationEntry>>,
    /// The Ethereum client [Middleware](ethers::providers::Middleware)
    pub eth_client: Arc<M>,
    // The maximum gas limit for [UserOperation](UserOperation) gas verification.
    pub max_verification_gas: U256,
    // The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID
    pub chain: Chain,
}

impl<M: Middleware + 'static, V: UserOperationValidator> UoPool<M, V> {
    /// Creates a new [UoPool](UoPool) object
    ///
    /// # Arguments
    /// `entry_point` - The [EntryPoint](EntryPoint) contract object
    /// `validator` - The [UserOperationValidator](UserOperationValidator) object
    /// `mempool` - The [MempoolBox](MempoolBox) is a [Boxed pointer](https://doc.rust-lang.org/std/boxed/struct.Box.html) to a [Mempool](Mempool) object
    /// `reputation` - The [ReputationBox](ReputationBox) is a [Boxed pointer](https://doc.rust-lang.org/std/boxed/struct.Box.html) to a [ReputationEntry](ReputationEntry) object
    /// `eth_client` - The Ethereum client [Middleware](ethers::providers::Middleware)
    /// `max_verification_gas` - The maximum gas limit for [UserOperation](UserOperation) gas verification.
    /// `chain` - The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID
    ///
    /// # Returns
    /// `Self` - The [UoPool](UoPool) object
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entry_point: EntryPoint<M>,
        validator: V,
        mempool: MempoolBox<VecUo, VecCh>,
        reputation: ReputationBox<Vec<ReputationEntry>>,
        eth_client: Arc<M>,
        max_verification_gas: U256,
        chain: Chain,
    ) -> Self {
        Self {
            id: mempool_id(&entry_point.address(), &chain.id().into()),
            entry_point,
            validator,
            mempool,
            reputation,
            eth_client,
            max_verification_gas,
            chain,
        }
    }

    /// Returns the [EntryPoint](EntryPoint) contract address
    ///
    /// # Returns
    /// `Address` - The [EntryPoint](EntryPoint) contract address
    pub fn entry_point_address(&self) -> Address {
        self.entry_point.address()
    }

    /// Returns all of the [UserOperations](UserOperation) in the mempool
    ///
    /// # Returns
    /// `Vec<UserOperation>` - An array of [UserOperations](UserOperation)
    pub fn get_all(&self) -> Vec<UserOperation> {
        self.mempool.get_all()
    }

    /// Returns an array of [ReputationEntry](ReputationEntry) for entities.
    ///
    /// # Returns
    /// `Vec<ReputationEntry>` - An array of [ReputationEntry](ReputationEntry)
    pub fn get_reputation(&self) -> Vec<ReputationEntry> {
        self.reputation.get_all()
    }

    /// Sets the [ReputationEntry](ReputationEntry) for entities
    ///
    /// # Arguments
    /// `reputation` - An array of [ReputationEntry](ReputationEntry)
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn set_reputation(&mut self, reputation: Vec<ReputationEntry>) -> anyhow::Result<()> {
        self.reputation.set_entities(reputation)
    }

    /// Batch clears the [Mempool](Mempool) and [Reputation](Reputation).
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn clear(&mut self) {
        self.mempool.clear();
        self.reputation.clear();
    }

    /// Validates a single [UserOperation](UserOperation) and returns the validation outcome by calling [UserOperationValidator::validate_user_operation](UserOperationValidator::validate_user_operation)
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to validate
    ///
    /// # Returns
    /// `Result<UserOperationValidationOutcome, ValidationError>` - The validation outcome
    pub async fn validate_user_operation(
        &self,
        uo: &UserOperation,
    ) -> Result<UserOperationValidationOutcome, ValidationError> {
        self.validator
            .validate_user_operation(
                uo,
                &self.mempool,
                &self.reputation,
                UserOperationValidatorMode::Sanity
                    | UserOperationValidatorMode::Simulation
                    | UserOperationValidatorMode::SimulationTrace,
            )
            .await
    }

    /// Adds a single validated user operation into the pool
    /// Indirectly invoked by [EthApiServer::send_user_operation](EthApiServer::send_user_operation) via [UoPoolService::add](UoPoolService::add) to add a [UserOperation](UserOperation) into the mempool
    /// The function first validates the [UserOperation](UserOperation) by calling [UoPool::validate_user_operation](UoPool::validate_user_operation). If [UserOperation](UserOperation) passes the validation, then adds it into the mempool by calling [Mempool::add](Mempool::add).
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to add
    /// `res` - The [UserOperationValidationOutcome](UserOperationValidationOutcome) of the validation
    ///
    /// # Returns
    /// `Result<UserOperationHash, AddError>` - The hash of the added [UserOperation](UserOperation)
    pub async fn add_user_operation(
        &mut self,
        uo: UserOperation,
        res: Option<UserOperationValidationOutcome>,
    ) -> Result<UserOperationHash, AddError> {
        let res = res.unwrap_or(self.validate_user_operation(&uo).await?);

        if let Some(uo_hash) = res.prev_hash {
            self.remove_user_operation(&uo_hash);
        }

        match self.mempool.add(
            uo.clone(),
            &self.entry_point.address(),
            &self.chain.id().into(),
        ) {
            Ok(uo_hash) => {
                // TODO: find better way to do it atomically
                if let Some(code_hashes) = res.code_hashes {
                    let _ = self.mempool.set_code_hashes(&uo_hash, &code_hashes);
                }

                trace!("User operation {uo:?} added to the mempool {}", self.id);

                // update reputation
                self.reputation
                    .increment_seen(&uo.sender)
                    .map_err(|e| AddError::MempoolError {
                        message: e.to_string(),
                    })?;
                if let Some(f_addr) = get_address(&uo.init_code) {
                    self.reputation.increment_seen(&f_addr).map_err(|e| {
                        AddError::MempoolError {
                            message: e.to_string(),
                        }
                    })?;
                }
                if let Some(p_addr) = get_address(&uo.paymaster_and_data) {
                    self.reputation.increment_seen(&p_addr).map_err(|e| {
                        AddError::MempoolError {
                            message: e.to_string(),
                        }
                    })?;
                }

                Ok(uo_hash)
            }
            Err(e) => Err(AddError::MempoolError {
                message: e.to_string(),
            }),
        }
    }

    /// Sorts the [UserOperations](UserOperation) in the mempool by calling the [Mempool::get_sorted](Mempool::get_sorted) function
    ///
    /// # Returns
    /// `Result<Vec<UserOperation>, anyhow::Error>` - The sorted [UserOperations](UserOperation)
    pub fn get_sorted_user_operations(&self) -> anyhow::Result<Vec<UserOperation>> {
        self.mempool.get_sorted()
    }

    /// Bundles an array of [UserOperations](UserOperation)
    /// The function first checks the reputations of the entiries, then validate each [UserOperation](UserOperation) by calling [UoPool::validate_user_operation](UoPool::validate_user_operation).
    /// If the [UserOperations](UserOperation) passes the validation, push it into the `uos_valid` array.
    ///
    /// # Arguments
    /// `uos` - An array of [UserOperations](UserOperation) to bundle
    ///
    /// # Returns
    /// `Result<Vec<UserOperation>, anyhow::Error>` - The bundled [UserOperations](UserOperation).
    pub async fn bundle_user_operations(
        &mut self,
        uos: Vec<UserOperation>,
    ) -> anyhow::Result<Vec<UserOperation>> {
        let mut uos_valid = vec![];
        let mut senders = HashSet::new();
        let mut gas_total = U256::zero();
        let mut paymaster_dep = HashMap::new();
        let mut staked_entity_c = HashMap::new();

        let senders_all = uos.iter().map(|uo| uo.sender).collect::<HashSet<_>>();

        'uos: for uo in uos {
            if senders.contains(&uo.sender) {
                continue;
            }

            let uo_hash = uo.hash(&self.entry_point.address(), &self.chain.id().into());

            let p_opt = get_address(&uo.paymaster_and_data.0);
            let f_opt = get_address(&uo.init_code.0);

            let p_st = Status::from(
                self.reputation
                    .get_status_from_bytes(&uo.paymaster_and_data)
                    .map_err(|err| {
                        format_err!("Error getting reputation status with error: {err:?}")
                    })?,
            );
            let f_st = Status::from(
                self.reputation
                    .get_status_from_bytes(&uo.init_code)
                    .map_err(|err| {
                        format_err!("Error getting reputation status with error: {err:?}")
                    })?,
            );

            let p_c = p_opt
                .map(|p| staked_entity_c.get(&p).cloned().unwrap_or(0))
                .unwrap_or(0);
            let f_c = f_opt
                .map(|f| staked_entity_c.get(&f).cloned().unwrap_or(0))
                .unwrap_or(0);

            match (p_st, f_st) {
                (Status::BANNED, _) | (_, Status::BANNED) => {
                    self.mempool.remove(&uo_hash).map_err(|err| {
                        format_err!(
                            "Removing a banned user operation {uo_hash:?} failed with error: {err:?}",
                        )
                    })?;
                    continue;
                }
                (Status::THROTTLED, _) if p_c > THROTTLED_MAX_INCLUDE => {
                    continue;
                }
                (_, Status::THROTTLED) if f_c > THROTTLED_MAX_INCLUDE => {
                    continue;
                }
                _ => (),
            };

            let val_out = self
                .validator
                .validate_user_operation(
                    &uo,
                    &self.mempool,
                    &self.reputation,
                    UserOperationValidatorMode::Simulation
                        | UserOperationValidatorMode::SimulationTrace,
                )
                .await;

            match val_out {
                Ok(val_out) => {
                    if val_out.valid_after.is_some() {
                        continue;
                    }

                    if let Some(storage_map) = val_out.storage_map {
                        for addr in storage_map.keys() {
                            if *addr != uo.sender && senders_all.contains(addr) {
                                continue 'uos;
                            }
                        }
                    }

                    // TODO
                    // it would be better to use estimate_gas instead of call_gas_limit
                    // The result of call_gas_limit is usesally higher and less user op would be included
                    let gas_cost = val_out
                        .verification_gas_limit
                        .saturating_add(uo.call_gas_limit);
                    let gas_total_new = gas_total.saturating_add(gas_cost);
                    if gas_total_new.gt(&self.max_verification_gas) {
                        break;
                    }

                    if let Some(p) = p_opt {
                        let balance = match paymaster_dep.get(&p) {
                            Some(n) => *n,
                            None => self.entry_point.balance_of(&p).await.map_err(|err| {
                                format_err!(
                                    "Getting balance of paymaster {p:?} failed with error: {err:?}",
                                )
                            })?,
                        };

                        if balance.lt(&val_out.pre_fund) {
                            continue;
                        }

                        staked_entity_c
                            .entry(p)
                            .and_modify(|c| *c += 1)
                            .or_insert(1);
                        paymaster_dep.insert(p, balance.saturating_sub(val_out.pre_fund));
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

    /// Gets the block base fee per gas
    ///
    /// # Returns
    /// `Result<U256, anyhow::Error>` - The block base fee per gas.
    pub async fn base_fee_per_gas(&self) -> anyhow::Result<U256> {
        let block = self
            .eth_client
            .get_block(BlockNumber::Latest)
            .await?
            .ok_or(format_err!("No block found"))?;
        block
            .base_fee_per_gas
            .ok_or(format_err!("No base fee found"))
    }

    /// Estimates the `verification_gas_limit`, `call_gas_limit` and `pre_verification_gas` for a user operation.
    /// The function is indirectly invoked by the `estimate_user_operation_gas` JSON RPC method.
    ///
    /// # Arguments
    /// * `uo` - The [UserOperation](UserOperation) to estimate the gas for.
    ///
    /// # Returns
    /// `Result<UserOperationGasEstimation, SimulationCheckError>` - The gas estimation result, which includes the `verification_gas_limit`, `call_gas_limit` and `pre_verification_gas`.
    pub async fn estimate_user_operation_gas(
        &self,
        uo: &UserOperation,
    ) -> Result<UserOperationGasEstimation, SimulationCheckError> {
        let val_out = self
            .validator
            .validate_user_operation(
                uo,
                &self.mempool,
                &self.reputation,
                UserOperationValidatorMode::SimulationTrace.into(),
            )
            .await
            .map_err(|err| match err {
                ValidationError::Sanity(_) => SimulationCheckError::UnknownError {
                    message: "Unknown error".to_string(),
                },
                ValidationError::Simulation(err) => err,
            })?;

        match self.entry_point.simulate_execution(uo.clone()).await {
            Ok(_) => {}
            Err(err) => {
                return Err(match err {
                    EntryPointErr::JsonRpcError(err) => SimulationCheckError::Execution {
                        message: err.message,
                    },
                    _ => SimulationCheckError::UnknownError {
                        message: format!("{err:?}"),
                    },
                })
            }
        }

        let exec_res = match self.entry_point.simulate_handle_op(uo.clone()).await {
            Ok(res) => res,
            Err(err) => {
                return Err(match err {
                    EntryPointErr::JsonRpcError(err) => SimulationCheckError::Execution {
                        message: err.message,
                    },
                    _ => SimulationCheckError::UnknownError {
                        message: format!("{err:?}"),
                    },
                })
            }
        };

        let base_fee_per_gas =
            self.base_fee_per_gas()
                .await
                .map_err(|err| SimulationCheckError::UnknownError {
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
            verification_gas_limit: val_out.verification_gas_limit,
            call_gas_limit,
        })
    }

    /// Filters the events logged from the [EntryPoint](EntryPoint) contract for a given user operation hash.
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to filter the events for.
    ///
    /// # Returns
    /// `Result<Option<(UserOperationEventFilter, LogMeta)>, anyhow::Error>` - The filtered event, if any.
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

    /// Gets the user operation by hash.
    /// The function is indirectly invoked by the `get_user_operation_by_hash` JSON RPC method.
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to get the user operation for.
    ///
    /// # Returns
    /// `Result<UserOperationByHash, anyhow::Error>` - The user operation, if any.
    pub async fn get_user_operation_by_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> anyhow::Result<UserOperationByHash> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some((uo, ep)) = self
                .eth_client
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

    /// Gets the [UserOperationReceipt](UserOperationReceipt) by hash.
    /// The function is indirectly invoked by the `get_user_operation_receipt` JSON RPC method.
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to get the user operation receipt for.
    ///
    /// # Returns
    /// `Result<UserOperationReceipt, anyhow::Error>` - The user operation receipt, if any.
    pub async fn get_user_operation_receipt(
        &self,
        uo_hash: &UserOperationHash,
    ) -> anyhow::Result<UserOperationReceipt> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some(tx_receipt) = self
                .eth_client
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

    /// Queries the [EntryPoint](EntryPoint) contract for the past events logged that are included in the current block.
    /// If [UserOperation](UserOperation) is found, it is removed from the [UserOperationQueue](UserOperationQueue), while simultaneously incrementing the reputation of the sender and paymaster.
    ///
    /// # Returns
    /// `Result<(), anyhow::Error>` - None if the query was successful.
    pub async fn handle_past_events(&mut self) -> anyhow::Result<()> {
        let block_num = self.eth_client.get_block_number().await?;
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
                    self.reputation.increment_included(&uo_event.sender)?;
                    self.reputation.increment_included(&uo_event.paymaster)?;
                    // TODO: include event aggregator
                }
                EntryPointAPIEvents::AccountDeployedFilter(event) => {
                    self.reputation.increment_included(&event.factory)?;
                }
                _ => (),
            }
        }

        Ok(())
    }

    /// Removes the [UserOperation](UserOperation) from the [UserOperationQueue](UserOperationQueue) given the [UserOperationHash](UserOperationHash).
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to remove the user operation for.
    ///
    /// # Returns
    /// `Option<()>` - None if the user operation was successfully removed.
    pub fn remove_user_operation(&mut self, uo_hash: &UserOperationHash) -> Option<()> {
        self.mempool.remove(uo_hash).ok();
        None
    }

    /// Removes multiple [UserOperations](UserOperation) from the [UserOperationQueue](UserOperationQueue) given an array of [UserOperationHash](UserOperationHash).
    ///
    /// # Arguments
    /// * `uo_hashes` - The array of [UserOperationHash](UserOperationHash) to remove the user operations for.
    ///
    /// # Returns
    /// `Option<()>` - None
    pub fn remove_user_operations(&mut self, uo_hashes: Vec<UserOperationHash>) {
        for uo_hash in uo_hashes {
            self.remove_user_operation(&uo_hash);
        }
    }
}
