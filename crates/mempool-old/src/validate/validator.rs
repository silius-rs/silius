use super::{
    sanity::{
        call_gas::CallGas, entities::Entities, max_fee::MaxFee, paymaster::Paymaster,
        sender::Sender, unstaked_entities::UnstakedEntities, verification_gas::VerificationGas,
    },
    simulation::{
        signature::Signature, timestamp::Timestamp, verification_extra_gas::VerificationExtraGas,
    },
    simulation_trace::{
        call_stack::CallStack, code_hashes::CodeHashes, external_contracts::ExternalContracts,
        gas::Gas, opcodes::Opcodes, storage_access::StorageAccess,
    },
    utils::{extract_pre_fund, extract_storage_map, extract_verification_gas_limit},
    SanityCheck, SanityHelper, SimulationCheck, SimulationHelper, SimulationTraceCheck,
    SimulationTraceHelper, UserOperationValidationOutcome, UserOperationValidator,
    UserOperationValidatorMode,
};
use crate::{
    mempool::Mempool, InvalidMempoolUserOperationError, Reputation, SanityError, SimulationError,
};
use alloy_chains::Chain;
use enumset::EnumSet;
use ethers::{
    providers::Middleware,
    types::{BlockNumber, GethTrace, U256},
};
use silius_contracts::{
    entry_point::{EntryPointError, SimulateValidationResult},
    tracer::JsTracerFrame,
    EntryPoint,
};
use silius_primitives::{simulation::ValidationConfig, UserOperation};
use tracing::debug;

pub type StandardValidator<M> = StandardUserOperationValidator<
    M,
    (Sender, VerificationGas, CallGas, MaxFee, Paymaster, Entities, UnstakedEntities),
    (Signature, Timestamp, VerificationExtraGas),
    (Gas, Opcodes, ExternalContracts, StorageAccess, CallStack, CodeHashes),
>;

type UnsafeValidator<M> = StandardUserOperationValidator<
    M,
    (Sender, VerificationGas, CallGas, MaxFee, Paymaster, Entities, UnstakedEntities),
    (Signature, Timestamp, VerificationExtraGas),
    (),
>;

/// Standard implementation of [UserOperationValidator].
pub struct StandardUserOperationValidator<M: Middleware + 'static, SanCk, SimCk, SimTrCk>
where
    SanCk: SanityCheck<M>,
    SimCk: SimulationCheck,
    SimTrCk: SimulationTraceCheck<M>,
{
    /// The [EntryPoint](EntryPoint) object.
    entry_point: EntryPoint<M>,
    /// A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    chain: Chain,
    /// An array of [SanityChecks](SanityCheck).
    sanity_checks: SanCk,
    /// An array of [SimulationCheck](SimulationCheck).
    simulation_checks: SimCk,
    /// An array of [SimulationTraceChecks](SimulationTraceCheck).
    simulation_trace_checks: SimTrCk,
}

impl<M: Middleware + Clone + 'static, SanCk, SimCk, SimTrCk> Clone
    for StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>
where
    SanCk: SanityCheck<M> + Clone,
    SimCk: SimulationCheck + Clone,
    SimTrCk: SimulationTraceCheck<M> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            entry_point: self.entry_point.clone(),
            chain: self.chain,
            sanity_checks: self.sanity_checks.clone(),
            simulation_checks: self.simulation_checks.clone(),
            simulation_trace_checks: self.simulation_trace_checks.clone(),
        }
    }
}

/// Creates a new [StandardUserOperationValidator]
/// with the default sanity checks and simulation checks for canonical mempool.
///
/// # Arguments
/// `entry_point` - [EntryPoint] object.
/// `chain` - A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
/// `max_verification_gas` - max verification gas that bundler would accept for one user operation
/// `min_priority_fee_per_gas` - min priority fee per gas that bundler would accept for one user
/// operation `max_uos_per_sender` - max user operations that bundler would accept from one sender
/// `gas_increase_perc` - gas increase percentage that bundler would accept for overwriting one user
/// operation
///
/// # Returns
/// A new [StandardUserOperationValidator].
pub fn new_canonical<M: Middleware + 'static>(
    entry_point: EntryPoint<M>,
    chain: Chain,
    max_verification_gas: U256,
    min_priority_fee_per_gas: U256,
) -> StandardValidator<M> {
    StandardUserOperationValidator::new(
        entry_point,
        chain,
        (
            Sender,
            VerificationGas { max_verification_gas },
            CallGas,
            MaxFee { min_priority_fee_per_gas },
            Paymaster,
            Entities,
            UnstakedEntities,
        ),
        (Signature, Timestamp, VerificationExtraGas),
        (Gas, Opcodes, ExternalContracts, StorageAccess, CallStack, CodeHashes),
    )
}

pub fn new_canonical_unsafe<M: Middleware + Clone + 'static>(
    entry_point: EntryPoint<M>,
    chain: Chain,
    max_verification_gas: U256,
    min_priority_fee_per_gas: U256,
) -> UnsafeValidator<M> {
    StandardUserOperationValidator::new(
        entry_point.clone(),
        chain,
        (
            Sender,
            VerificationGas { max_verification_gas },
            CallGas,
            MaxFee { min_priority_fee_per_gas },
            Paymaster,
            Entities,
            UnstakedEntities,
        ),
        (Signature, Timestamp, VerificationExtraGas),
        (),
    )
}

impl<M: Middleware + 'static, SanCk, SimCk, SimTrCk>
    StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>
where
    SanCk: SanityCheck<M>,
    SimCk: SimulationCheck,
    SimTrCk: SimulationTraceCheck<M>,
{
    pub fn new(
        entry_point: EntryPoint<M>,
        chain: Chain,
        sanity_checks: SanCk,
        simulation_checks: SimCk,
        simulation_trace_checks: SimTrCk,
    ) -> Self {
        Self { entry_point, chain, sanity_checks, simulation_checks, simulation_trace_checks }
    }

    /// Simulates validation of a [UserOperation](UserOperation) via the
    /// [simulate_validation](crate::entry_point::EntryPoint::simulate_validation) method of the
    /// [entry_point](crate::entry_point::EntryPoint).
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to simulate validation on.
    ///
    /// # Returns
    /// A [SimulateValidationResult](crate::entry_point::SimulateValidationResult) if the simulation
    /// was successful, otherwise a [SimulationError](crate::error::SimulationError).
    async fn simulate_validation(
        &self,
        uo: &UserOperation,
    ) -> Result<SimulateValidationResult, SimulationError> {
        match self.entry_point.simulate_validation(uo.user_operation.clone()).await {
            Ok(res) => Ok(res),
            Err(err) => Err(match err {
                EntryPointError::FailedOp(op) => SimulationError::Validation { inner: op.reason },
                EntryPointError::Provider { inner } => SimulationError::Provider { inner },
                _ => SimulationError::Other { inner: err.to_string() },
            }),
        }
    }

    /// Simulates validation of a [UserOperation](UserOperation) via the
    /// [simulate_validation_trace](crate::entry_point::EntryPoint::simulate_validation_trace)
    /// method of the [entry_point](crate::entry_point::EntryPoint)
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to simulate validation on.
    ///
    /// # Returns
    /// A [GethTrace](ethers::types::GethTrace) if the simulation was successful, otherwise a
    /// [SimulationError](crate::error::SimulationError).
    async fn simulate_validation_trace(
        &self,
        uo: &UserOperation,
    ) -> Result<GethTrace, SimulationError> {
        match self.entry_point.simulate_validation_trace(uo.user_operation.clone()).await {
            Ok(trace) => Ok(trace),
            Err(err) => Err(match err {
                EntryPointError::FailedOp(op) => SimulationError::Validation { inner: op.reason },
                EntryPointError::Provider { inner } => SimulationError::Provider { inner },
                _ => SimulationError::Other { inner: err.to_string() },
            }),
        }
    }
}

#[async_trait::async_trait]
impl<M: Middleware + 'static, SanCk, SimCk, SimTrCk> UserOperationValidator
    for StandardUserOperationValidator<M, SanCk, SimCk, SimTrCk>
where
    SanCk: SanityCheck<M>,
    SimCk: SimulationCheck,
    SimTrCk: SimulationTraceCheck<M>,
{
    /// Validates a [UserOperation](UserOperation) via the
    /// [simulate_validation](silius_contracts::entry_point::EntryPoint::simulate_validation) method
    /// of the entry point. The function also optionally performs
    /// sanity checks and simulation checks if the
    /// [UserOperationValidatorMode](UserOperationValidatorMode) contains the respective flags.
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to validate.
    /// `mempool` - [Mempool](Mempool) object.
    /// `reputation` - [Reputation](Reputation) object.
    /// `val_config` - Optional [ValidationConfig](ValidationConfig) object.
    /// `mode` - [UserOperationValidatorMode](UserOperationValidatorMode) flag.
    ///
    /// # Returns
    /// A [UserOperationValidationOutcome](UserOperationValidationOutcome) if the validation was
    /// successful, otherwise a
    /// [InvalidMempoolUserOperationError](InvalidMempoolUserOperationError).
    async fn validate_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        reputation: &Reputation,
        val_config: Option<ValidationConfig>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError> {
        let mut out: UserOperationValidationOutcome = Default::default();

        if let Some(val_config) = val_config.clone() {
            out.val_config = val_config;
        } else {
            out.val_config = ValidationConfig {
                min_stake: Some(reputation.min_stake()),
                min_unstake_delay: Some(reputation.min_unstake_delay()),
                topic: None,
                ignore_prev: false,
            };
        }

        if mode.contains(UserOperationValidatorMode::Sanity) {
            let sanity_helper = SanityHelper {
                entry_point: &self.entry_point,
                chain: self.chain,
                val_config: val_config.clone().unwrap_or_default(),
            };

            self.sanity_checks
                .check_user_operation(uo, mempool, reputation, &sanity_helper)
                .await?;
        }

        if let Some(uo) = mempool.get_prev_by_sender(uo) {
            out.prev_hash = Some(uo.hash);
        }

        debug!("Simulate user operation from {:?}", uo.sender);
        let sim_res = self.simulate_validation(uo).await?;

        if mode.contains(UserOperationValidatorMode::Simulation) {
            let mut sim_helper = SimulationHelper {
                simulate_validation_result: &sim_res,
                val_config: val_config.clone().unwrap_or_default(),
                valid_after: None,
            };

            self.simulation_checks.check_user_operation(uo, &mut sim_helper)?;

            out.valid_after = sim_helper.valid_after;
        }

        out.pre_fund = extract_pre_fund(&sim_res);
        out.verification_gas_limit = extract_verification_gas_limit(&sim_res);

        let block_number = self
            .entry_point
            .eth_client()
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|e| SanityError::Provider { inner: e.to_string() })?
            .expect("block should exist");
        out.verified_block = U256::from(block_number.hash.expect("block hash should exist").0);

        if mode.contains(UserOperationValidatorMode::SimulationTrace) {
            debug!("Simulate user operation with trace from {:?}", uo.sender);
            let geth_trace = self.simulate_validation_trace(uo).await?;
            let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace)
                .map_err(|error| SimulationError::Validation { inner: error.to_string() })?;

            let mut sim_helper = SimulationTraceHelper {
                entry_point: &self.entry_point,
                chain: self.chain,
                simulate_validation_result: &sim_res,
                js_trace: &js_trace,
                val_config: val_config.unwrap_or_default(),
                stake_info: None,
                code_hashes: None,
            };

            self.simulation_trace_checks
                .check_user_operation(uo, mempool, reputation, &mut sim_helper)
                .await?;

            out.code_hashes = sim_helper.code_hashes;
            out.storage_map = extract_storage_map(&js_trace);
        }

        Ok(out)
    }
}
