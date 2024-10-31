use crate::{
    estimate::estimate_user_op_gas,
    mempool::Mempool,
    mempool_id,
    utils::div_ceil,
    validate::{
        utils::merge_storage_maps, UserOperationValidationOutcome, UserOperationValidator,
        UserOperationValidatorMode,
    },
    InvalidMempoolUserOperationError, MempoolError, MempoolErrorKind, MempoolId, Overhead,
    Reputation, ReputationError, SanityError, SimulationError,
};
use alloy_chains::Chain;
use ethers::{
    prelude::LogMeta,
    providers::Middleware,
    types::{Address, BlockNumber, U256},
};
use eyre::format_err;
use futures::channel::mpsc::UnboundedSender;
use silius_contracts::{
    entry_point::UserOperationEventFilter, utils::parse_from_input_data, EntryPoint,
    EntryPointError,
};
use silius_primitives::{
    constants::validation::reputation::THROTTLED_ENTITY_BUNDLE_COUNT,
    get_address,
    p2p::NetworkMessage,
    reputation::{ReputationEntry, StakeInfo, StakeInfoResponse, Status},
    simulation::{StorageMap, ValidationConfig},
    UoPoolMode, UserOperation, UserOperationByHash, UserOperationGasEstimation, UserOperationHash,
    UserOperationReceipt,
};
use std::collections::{HashMap, HashSet};
use tracing::{debug, error, info, trace};

const FILTER_MAX_DEPTH: u64 = 10;
const PRE_VERIFICATION_SAFE_RESERVE_PERC: u64 = 10; // percentage how higher pre verification gas we return

/// The alternative mempool pool implementation that provides functionalities to add, remove,
/// validate, and serves data requests from the RPC API. Architecturally, the
/// [UoPool](UoPool) is the backend service managed by the user operation service and serves
/// requests from the RPC API.
pub struct UoPool<M: Middleware + 'static, V: UserOperationValidator> {
    /// The unique ID of the mempool
    pub id: MempoolId,
    /// User operation pool mode
    pub mode: UoPoolMode,
    /// The [EntryPoint](EntryPoint) contract object
    pub entry_point: EntryPoint<M>,
    /// The [UserOperationValidator](UserOperationValidator) object
    pub validator: V,
    /// The [Mempool](Mempool) object
    pub mempool: Mempool,
    /// The [Reputation](Reputation) object
    pub reputation: Reputation,
    // The maximum gas limit for [UserOperation](UserOperation) gas verification.
    pub max_verification_gas: U256,
    // The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID
    pub chain: Chain,
    // Connection to the p2p network (None if not enabled)
    network: Option<UnboundedSender<NetworkMessage>>,
}

impl<M: Middleware + 'static, V: UserOperationValidator> UoPool<M, V> {
    /// Creates a new [UoPool](UoPool) object
    ///
    /// # Arguments
    /// `mode` - The [UoPoolMode](UoPoolMode) object
    /// `entry_point` - The [EntryPoint](EntryPoint) contract object
    /// `validator` - The [UserOperationValidator](UserOperationValidator) object
    /// `mempool` - The [Mempool](Mempool) object
    /// `reputation` - The [Reputation](Reputation) object
    /// `max_verification_gas` - The maximum gas limit for [UserOperation](UserOperation) gas
    /// verification.
    /// `chain` - The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID
    /// `network` - Connection to the p2p network (None if not enabled)
    ///
    /// # Returns
    /// `Self` - The [UoPool](UoPool) object
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: UoPoolMode,
        entry_point: EntryPoint<M>,
        validator: V,
        mempool: Mempool,
        reputation: Reputation,
        max_verification_gas: U256,
        chain: Chain,
        network: Option<UnboundedSender<NetworkMessage>>,
    ) -> Self {
        Self {
            id: mempool_id(&entry_point.address(), chain.id()),
            mode,
            entry_point,
            validator,
            mempool,
            reputation,
            max_verification_gas,
            chain,
            network,
        }
    }

    /// Returns all of the [UserOperations](UserOperation) in the mempool
    ///
    /// # Returns
    /// `Result<Vec<UserOperation>, eyre::Error>` - An array of [UserOperations](UserOperation)
    pub fn get_all(&self) -> eyre::Result<Vec<UserOperation>> {
        self.mempool.get_all().map_err(|err| {
            format_err!("Getting all user operations from mempool failed with error: {err:?}",)
        })
    }

    /// Returns an array of [ReputationEntry](ReputationEntry) for entities.
    ///
    /// # Returns
    /// `Vec<ReputationEntry>` - An array of [ReputationEntry](ReputationEntry)
    pub fn get_reputation(&self) -> Vec<ReputationEntry> {
        self.reputation.get_all().unwrap_or_default()
    }

    /// Sets the [ReputationEntry](ReputationEntry) for entities
    ///
    /// # Arguments
    /// `reputation` - An array of [ReputationEntry](ReputationEntry)
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn set_reputation(
        &mut self,
        reputation: Vec<ReputationEntry>,
    ) -> Result<(), ReputationError> {
        self.reputation.set_entities(reputation)
    }

    /// Batch clears the [Mempool](Mempool).
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }

    /// Batch clears the [Reputation](Reputation).
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn clear_reputation(&mut self) {
        self.reputation.clear();
    }

    /// Batch clears the [Mempool](Mempool) and [Reputation](Reputation).
    ///
    /// # Returns
    /// `()` - Returns nothing
    pub fn clear(&mut self) {
        self.mempool.clear();
        self.reputation.clear();
    }

    /// Adds bulk of [UserOperations](UserOperation) into the mempool.
    /// The function first validates the [UserOperations](UserOperation).
    ///
    /// # Arguments
    /// `user_operations` - The array of [UserOperations](UserOperation) to add
    /// `val_config` - The optional [ValidationConfig](ValidationConfig) object
    ///
    /// # Returns
    /// `Result<(), MempoolError>` - Ok if the [UserOperations](UserOperation) are added
    /// successfully into the mempool
    pub async fn add_user_operations(
        &mut self,
        user_operations: Vec<UserOperation>,
        val_config: Option<ValidationConfig>,
    ) -> Result<(), MempoolError> {
        for uo in user_operations {
            let res = self.validate_user_operation(&uo, val_config.clone()).await;
            self.add_user_operation(uo, res).await?;
        }

        Ok(())
    }

    /// Validates a single [UserOperation](UserOperation) and returns the validation outcome by
    /// calling [UserOperationValidator::validate_user_operation](UserOperationValidator::validate_user_operation)
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to validate
    /// `val_config` - The optional [ValidationConfig](ValidationConfig) object
    ///
    /// # Returns
    /// `Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError>` - The validation
    /// outcome
    pub async fn validate_user_operation(
        &self,
        uo: &UserOperation,
        val_config: Option<ValidationConfig>,
    ) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError> {
        self.validator
            .validate_user_operation(
                uo,
                &self.mempool,
                &self.reputation,
                val_config,
                UserOperationValidatorMode::Sanity |
                    UserOperationValidatorMode::Simulation |
                    UserOperationValidatorMode::SimulationTrace,
            )
            .await
    }

    /// Adds a single validated user operation into the pool
    /// Indirectly invoked by RPC API via gRPC sevice to add a [UserOperation](UserOperation) into
    /// the mempool The function first validates the [UserOperation](UserOperation) by calling
    /// [UoPool::validate_user_operation](UoPool::validate_user_operation). If
    /// [UserOperation](UserOperation) passes the validation, then adds it into the mempool by
    /// calling [Mempool::add](Mempool::add).
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to add
    /// `res` - The [UserOperationValidationOutcome](UserOperationValidationOutcome) of the
    /// validation
    ///
    /// # Returns
    /// `Result<UserOperationHash, MempoolError>` - The hash of the added
    /// [UserOperation](UserOperation)
    pub async fn add_user_operation(
        &mut self,
        uo: UserOperation,
        res: Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError>,
    ) -> Result<UserOperationHash, MempoolError> {
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                if let InvalidMempoolUserOperationError::Sanity(SanityError::Reputation(
                    ReputationError::BannedEntity { address, entity: _ },
                )) = err
                {
                    self.remove_user_operation_by_entity(&address);
                }
                return Err(MempoolError { hash: uo.hash, kind: err.into() });
            }
        };

        if let Some(uo_hash) = res.prev_hash {
            self.remove_user_operation(&uo_hash);
        }

        if let Some(ref sender) = self.network {
            sender
                .unbounded_send(NetworkMessage::Publish {
                    user_operation: uo.clone(),
                    verified_at_block_hash: res.verified_block,
                    validation_config: res.val_config,
                })
                .expect("Failed to send user operation to publish channel")
        };

        match self.mempool.add(uo.clone()) {
            Ok(uo_hash) => {
                // TODO: find better way to do it atomically
                if let Some(code_hashes) = res.code_hashes {
                    match self.mempool.set_code_hashes(&uo_hash, code_hashes){
                        Ok(_) => (),
                        Err(e) => error!("Failed to set code hashes for user operation {uo_hash:?} with error: {e:?}"),
                    }
                }
                info!("{uo_hash:?} added to the mempool {:?}", self.id);
                trace!("{uo:?} added to the mempool {:?}", self.id);

                // update reputation
                self.reputation
                    .increment_seen(&uo.sender)
                    .map_err(|e| MempoolError { hash: uo_hash, kind: e.into() })?;
                if let Some(f_addr) = get_address(&uo.init_code) {
                    self.reputation
                        .increment_seen(&f_addr)
                        .map_err(|e| MempoolError { hash: uo_hash, kind: e.into() })?;
                }
                if let Some(p_addr) = get_address(&uo.paymaster_and_data) {
                    self.reputation
                        .increment_seen(&p_addr)
                        .map_err(|e| MempoolError { hash: uo_hash, kind: e.into() })?;
                }

                Ok(uo_hash)
            }
            Err(e) => Err(MempoolError { hash: uo.hash, kind: e }),
        }
    }

    /// Sorts the [UserOperations](UserOperation) in the mempool by calling the
    /// [Mempool::get_sorted](Mempool::get_sorted) function
    ///
    /// # Returns
    /// `Result<Vec<UserOperation>, eyre::Error>` - The sorted [UserOperations](UserOperation)
    pub fn get_sorted_user_operations(&self) -> eyre::Result<Vec<UserOperation>> {
        self.mempool.get_sorted().map_err(|err| {
            format_err!("Getting sorted user operations from mempool failed with error: {err:?}",)
        })
    }

    /// Bundles an array of [UserOperations](UserOperation)
    /// The function first checks the reputations of the entities, then validate each
    /// [UserOperation](UserOperation) by calling
    /// [UoPool::validate_user_operation](UoPool::validate_user_operation).
    /// If the [UserOperations](UserOperation) passes the validation, push it into the `uos_valid`
    /// array.
    ///
    /// # Arguments
    /// `uos` - An array of [UserOperations](UserOperation) to bundle
    ///
    /// # Returns
    /// `Result<(Vec<UserOperation>, StorageMap), eyre::Error>` - The bundled
    /// [UserOperations](UserOperation).
    pub async fn bundle_user_operations(
        &mut self,
        uos: Vec<UserOperation>,
    ) -> eyre::Result<(Vec<UserOperation>, StorageMap)> {
        let mut uos_valid = vec![];
        let mut senders = HashSet::new();
        let mut gas_total = U256::zero();
        let mut paymaster_dep = HashMap::new();
        let mut staked_entity_c = HashMap::new();
        let mut storage_maps: Vec<StorageMap> = Vec::new();

        let senders_all = uos.iter().map(|uo| uo.sender).collect::<HashSet<_>>();

        'uos: for uo in uos {
            if senders.contains(&uo.sender) {
                continue;
            }

            let p_opt = get_address(&uo.paymaster_and_data.0);
            let f_opt = get_address(&uo.init_code.0);

            let p_st = Status::from(
                self.reputation.get_status_from_bytes(&uo.paymaster_and_data).map_err(|err| {
                    format_err!("Error getting reputation status with error: {err:?}")
                })?,
            );
            let f_st = Status::from(self.reputation.get_status_from_bytes(&uo.init_code).map_err(
                |err| format_err!("Error getting reputation status with error: {err:?}"),
            )?);

            let p_c = p_opt.map(|p| staked_entity_c.get(&p).cloned().unwrap_or(0)).unwrap_or(0);
            let f_c = f_opt.map(|f| staked_entity_c.get(&f).cloned().unwrap_or(0)).unwrap_or(0);

            match (p_st, f_st) {
                (Status::BANNED, _) | (_, Status::BANNED) => {
                    self.mempool.remove(&uo.hash).map_err(|err| {
                        format_err!(
                            "Removing a banned user operation {:?} failed with error: {err:?}",
                            uo.hash,
                        )
                    })?;
                    continue;
                }
                (Status::THROTTLED, _) if p_c > THROTTLED_ENTITY_BUNDLE_COUNT => {
                    continue;
                }
                (_, Status::THROTTLED) if f_c > THROTTLED_ENTITY_BUNDLE_COUNT => {
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
                    None,
                    UserOperationValidatorMode::Simulation |
                        UserOperationValidatorMode::SimulationTrace,
                )
                .await;

            debug!("Second validation for userop {:?} result: {:?}", uo.hash, val_out);

            match val_out {
                Ok(val_out) => {
                    if val_out.valid_after.is_some() {
                        continue;
                    }

                    for addr in val_out.storage_map.root_hashes.keys() {
                        if *addr != uo.sender && senders_all.contains(addr) {
                            continue 'uos;
                        }
                    }

                    for addr in val_out.storage_map.slots.keys() {
                        if *addr != uo.sender && senders_all.contains(addr) {
                            continue 'uos;
                        }
                    }

                    storage_maps.push(val_out.storage_map);

                    // TODO
                    // it would be better to use estimate_gas instead of call_gas_limit
                    // The result of call_gas_limit is usesally higher and less user op would be
                    // included
                    let gas_cost = val_out.verification_gas_limit.saturating_add(uo.call_gas_limit);
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

                        staked_entity_c.entry(p).and_modify(|c| *c += 1).or_insert(1);
                        paymaster_dep.insert(p, balance.saturating_sub(val_out.pre_fund));
                    }

                    if let Some(f) = f_opt {
                        staked_entity_c.entry(f).and_modify(|c| *c += 1).or_insert(1);
                    }

                    gas_total = gas_total_new;
                }
                Err(_) => {
                    self.mempool.remove(&uo.hash).map_err(|err| {
                        format_err!(
                            "Removing a user operation {:?} with 2nd failed simulation failed with error: {err:?}", uo.hash,
                        )
                    })?;
                    continue;
                }
            }

            uos_valid.push(uo.clone());
            senders.insert(uo.sender);
        }

        Ok((uos_valid, merge_storage_maps(storage_maps)))
    }

    /// Gets the block base fee per gas
    ///
    /// # Returns
    /// `Result<U256, eyre::Error>` - The block base fee per gas.
    pub async fn base_fee_per_gas(&self) -> eyre::Result<U256> {
        let block = self
            .entry_point
            .eth_client()
            .get_block(BlockNumber::Latest)
            .await?
            .ok_or(format_err!("No block found"))?;
        block.base_fee_per_gas.ok_or(format_err!("No base fee found"))
    }

    /// Estimates the `verification_gas_limit`, `call_gas_limit` and `pre_verification_gas` for a
    /// user operation. The function is indirectly invoked by the `estimate_user_operation_gas`
    /// JSON RPC method.
    ///
    /// # Arguments
    /// * `uo` - The [UserOperation](UserOperation) to estimate the gas for.
    ///
    /// # Returns
    /// `Result<UserOperationGasEstimation, MempoolError>` - The gas estimation result,
    /// which includes the `verification_gas_limit`, `call_gas_limit` and `pre_verification_gas`.
    pub async fn estimate_user_operation_gas(
        &self,
        uo: &UserOperation,
    ) -> Result<UserOperationGasEstimation, MempoolError> {
        let pre_verification_gas = div_ceil(
            Overhead::default().calculate_pre_verification_gas(uo).saturating_mul(
                U256::from(100).saturating_add(PRE_VERIFICATION_SAFE_RESERVE_PERC.into()),
            ),
            U256::from(100),
        );

        let (verification_gas_limit, call_gas_limit) = match self.mode {
            UoPoolMode::Standard => estimate_user_op_gas(&uo.user_operation, &self.entry_point)
                .await
                .map_err(|e| match e {
                    EntryPointError::FailedOp(op) => MempoolError {
                        hash: uo.hash,
                        kind: MempoolErrorKind::InvalidUserOperation(
                            InvalidMempoolUserOperationError::Simulation(
                                SimulationError::Validation { inner: op.reason },
                            ),
                        ),
                    },
                    EntryPointError::ExecutionReverted(e) => MempoolError {
                        hash: uo.hash,
                        kind: MempoolErrorKind::InvalidUserOperation(
                            InvalidMempoolUserOperationError::Simulation(
                                SimulationError::Execution { inner: e },
                            ),
                        ),
                    },
                    EntryPointError::Provider { inner } => {
                        MempoolError { hash: uo.hash, kind: MempoolErrorKind::Provider { inner } }
                    }
                    _ => MempoolError {
                        hash: uo.hash,
                        kind: MempoolErrorKind::Other { inner: e.to_string() },
                    },
                })?,
            UoPoolMode::Unsafe => {
                let ret =
                    self.entry_point.simulate_handle_op(uo.clone().user_operation).await.map_err(
                        |e| match e {
                            EntryPointError::FailedOp(op) => MempoolError {
                                hash: uo.hash,
                                kind: MempoolErrorKind::InvalidUserOperation(
                                    InvalidMempoolUserOperationError::Simulation(
                                        SimulationError::Validation { inner: op.reason },
                                    ),
                                ),
                            },
                            EntryPointError::ExecutionReverted(e) => MempoolError {
                                hash: uo.hash,
                                kind: MempoolErrorKind::InvalidUserOperation(
                                    InvalidMempoolUserOperationError::Simulation(
                                        SimulationError::Execution { inner: e },
                                    ),
                                ),
                            },
                            EntryPointError::Provider { inner } => MempoolError {
                                hash: uo.hash,
                                kind: MempoolErrorKind::Provider { inner },
                            },
                            _ => MempoolError {
                                hash: uo.hash,
                                kind: MempoolErrorKind::Other { inner: e.to_string() },
                            },
                        },
                    )?;

                let verification_gas_limit = div_ceil(
                    ret.pre_op_gas.saturating_sub(pre_verification_gas).saturating_mul(3.into()),
                    2.into(),
                );
                let call_gas_limit = div_ceil(ret.paid, uo.user_operation.max_fee_per_gas)
                    .saturating_sub(ret.pre_op_gas)
                    .saturating_add(35000.into());

                (verification_gas_limit, call_gas_limit)
            }
        };

        Ok(UserOperationGasEstimation {
            pre_verification_gas,
            verification_gas_limit,
            call_gas_limit,
        })
    }

    /// Filters the events logged from the [EntryPoint](EntryPoint) contract for a given user
    /// operation hash.
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to filter the events for.
    ///
    /// # Returns
    /// `Result<Option<(UserOperationEventFilter, LogMeta)>, eyre::Error>` - The filtered event, if
    /// any.
    pub async fn get_user_operation_event_meta(
        &self,
        uo_hash: &UserOperationHash,
    ) -> eyre::Result<Option<(UserOperationEventFilter, LogMeta)>> {
        let mut event: Option<(UserOperationEventFilter, LogMeta)> = None;
        let latest_block = self.entry_point.eth_client().get_block_number().await?;
        let filter = self
            .entry_point
            .entry_point_api()
            .event::<UserOperationEventFilter>()
            .from_block(latest_block - FILTER_MAX_DEPTH)
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
    /// `Result<UserOperationByHash, eyre::Error>` - The user operation, if any.
    pub async fn get_user_operation_by_hash(
        &self,
        uo_hash: &UserOperationHash,
    ) -> eyre::Result<UserOperationByHash> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some((uo, ep)) = self
                .entry_point
                .eth_client()
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
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) to get the user operation receipt
    ///   for.
    ///
    /// # Returns
    /// `Result<UserOperationReceipt, eyre::Error>` - The user operation receipt, if any.
    pub async fn get_user_operation_receipt(
        &self,
        uo_hash: &UserOperationHash,
    ) -> eyre::Result<UserOperationReceipt> {
        let event = self.get_user_operation_event_meta(uo_hash).await?;

        if let Some((event, log_meta)) = event {
            if let Some(tx_receipt) = self
                .entry_point
                .eth_client()
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

    /// Removes the [UserOperation](UserOperation) from the user operation mempool
    /// given the [UserOperationHash](UserOperationHash).
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

    pub fn remove_user_operation_by_entity(&mut self, entity: &Address) -> Option<()> {
        self.mempool.remove_by_entity(entity).ok();
        None
    }

    /// Removes multiple [UserOperations](UserOperation) from the
    /// user operation mempool given an array of
    /// [UserOperation](UserOperation).
    ///
    /// # Arguments
    /// * `uos` - The array of [UserOperation](UserOperation).
    ///
    /// # Returns
    /// `Option<()>` - None
    pub fn remove_user_operations(&mut self, uos: Vec<UserOperation>) -> Option<()> {
        for uo in uos {
            self.remove_user_operation(&uo.hash);

            // update reputations
            self.reputation.increment_included(&uo.sender).ok();

            if let Some(addr) = get_address(&uo.paymaster_and_data) {
                self.reputation.increment_included(&addr).ok();
            }

            if let Some(addr) = get_address(&uo.init_code) {
                self.reputation.increment_included(&addr).ok();
            }
        }

        None
    }

    /// Gets the [StakeInfoResponse](StakeInfoResponse) for entity
    ///
    /// # Arguments
    /// * `addr` - The address of the entity.
    ///
    /// # Returns
    /// `Result<StakeInfoResponse, eyre::Error>` - Stake info of the entity.
    pub async fn get_stake_info(&self, addr: &Address) -> eyre::Result<StakeInfoResponse> {
        let info = self.entry_point.get_deposit_info(addr).await?;
        let stake_info = StakeInfo {
            address: *addr,
            stake: U256::from(info.stake),
            unstake_delay: U256::from(info.unstake_delay_sec),
        };
        Ok(StakeInfoResponse {
            stake_info,
            is_staked: self.reputation.verify_stake("", Some(stake_info), None, None).is_ok(),
        })
    }
}
