use super::{
    sanity::{
        call_gas::CallGas, entities::Entities, max_fee::MaxFee, paymaster::Paymaster,
        sender::Sender, unstaked_entities::UnstakedEntities, verification_gas::VerificationGas,
    },
    simulation::{signature::Signature, timestamp::Timestamp},
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
    mempool::{Mempool, MempoolBox},
    reputation::ReputationBox,
    uopool::{VecCh, VecUo},
    Reputation as Rep,
};
use enumset::EnumSet;
use ethers::{
    providers::Middleware,
    types::{GethTrace, U256},
};
use silius_contracts::{
    entry_point::{EntryPointErr, SimulateValidationResult},
    tracer::JsTracerFrame,
    EntryPoint,
};
use silius_primitives::{
    reputation::ReputationEntry, simulation::SimulationCheckError, uopool::ValidationError, Chain,
    UserOperation,
};
use std::fmt::{Debug, Display};

/// Standard implementation of [UserOperationValidator](UserOperationValidator).
pub struct StandardUserOperationValidator<M: Middleware + Clone + 'static, P, R, E>
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
{
    /// The [EntryPoint](EntryPoint) object.
    entry_point: EntryPoint<M>,
    /// A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    chain: Chain,
    /// An array of [SanityChecks](SanityCheck).
    sanity_checks: Vec<Box<dyn SanityCheck<M, P, R, E>>>,
    /// An array of [SimulationCheck](SimulationCheck).
    simulation_checks: Vec<Box<dyn SimulationCheck>>,
    /// An array of [SimulationTraceChecks](SimulationTraceCheck).
    simulation_trace_checks: Vec<Box<dyn SimulationTraceCheck<M, P, R, E>>>,
}

impl<M: Middleware + Clone + 'static, P, R, E> StandardUserOperationValidator<M, P, R, E>
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
    E: Debug + Display,
{
    /// Creates a new [StandardUserOperationValidator](StandardUserOperationValidator).
    ///
    /// # Arguments
    /// `entry_point` - [EntryPoint](EntryPoint) object.
    /// `chain` - A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    ///
    /// # Returns
    /// A new [StandardUserOperationValidator](StandardUserOperationValidator).
    pub fn new(entry_point: EntryPoint<M>, chain: Chain) -> Self {
        Self {
            entry_point,
            chain,
            sanity_checks: vec![],
            simulation_checks: vec![],
            simulation_trace_checks: vec![],
        }
    }

    /// Creates a new [StandardUserOperationValidator](StandardUserOperationValidator)
    /// with the default sanity checks and simulation checks for canonical mempool.
    ///
    /// # Arguments
    /// `entry_point` - [EntryPoint](EntryPoint) object.
    /// `chain` - A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    /// `max_verification_gas` - max verification gas that bundler would accept for one user operation
    /// `min_priority_fee_per_gas` - min priority fee per gas that bundler would accept for one user operation
    /// `max_uos_per_sender` - max user operations that bundler would accept from one sender
    /// `gas_increase_perc` - gas increase percentage that bundler would accept for overwriting one user operation
    ///
    /// # Returns
    /// A new [StandardUserOperationValidator](StandardUserOperationValidator).
    pub fn new_canonical(
        entry_point: EntryPoint<M>,
        chain: Chain,
        max_verification_gas: U256,
        min_priority_fee_per_gas: U256,
    ) -> Self {
        Self::new(entry_point.clone(), chain)
            .with_sanity_check(Sender)
            .with_sanity_check(VerificationGas {
                max_verification_gas,
            })
            .with_sanity_check(CallGas)
            .with_sanity_check(MaxFee {
                min_priority_fee_per_gas,
            })
            .with_sanity_check(Paymaster)
            .with_sanity_check(Entities)
            .with_sanity_check(UnstakedEntities)
            .with_simulation_check(Signature)
            .with_simulation_check(Timestamp)
            .with_simulation_trace_check(Gas)
            .with_simulation_trace_check(Opcodes)
            .with_simulation_trace_check(ExternalContracts)
            .with_simulation_trace_check(StorageAccess)
            .with_simulation_trace_check(CallStack)
            .with_simulation_trace_check(CodeHashes)
            .with_simulation_trace_check(ExternalContracts)
    }

    /// Creates a new [StandardUserOperationValidator](StandardUserOperationValidator)
    /// with the default sanity checks
    /// Simulation checks are not included in this method.
    /// The unsafe model could be useful for L2 bundler.
    ///
    /// # Arguments
    /// `entry_point` - [EntryPoint](EntryPoint) object.
    /// `chain` - A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    /// `max_verification_gas` - max verification gas that bundler would accept for one user operation
    /// `min_priority_fee_per_gas` - min priority fee per gas that bundler would accept for one user operation
    /// `max_uos_per_sender` - max user operations that bundler would accept from one sender
    /// `gas_increase_perc` - gas increase percentage that bundler would accept for overwriting one user operation
    ///
    /// # Returns
    /// A new [StandardUserOperationValidator](StandardUserOperationValidator).
    pub fn new_canonical_unsafe(
        entry_point: EntryPoint<M>,
        chain: Chain,
        max_verification_gas: U256,
        min_priority_fee_per_gas: U256,
    ) -> Self {
        Self::new(entry_point.clone(), chain)
            .with_sanity_check(Sender)
            .with_sanity_check(VerificationGas {
                max_verification_gas,
            })
            .with_sanity_check(CallGas)
            .with_sanity_check(MaxFee {
                min_priority_fee_per_gas,
            })
            .with_sanity_check(Paymaster)
            .with_sanity_check(Entities)
            .with_sanity_check(UnstakedEntities)
            .with_simulation_check(Signature)
            .with_simulation_check(Timestamp)
    }

    /// Simulates validation of a [UserOperation](UserOperation) via the [simulate_validation](crate::entry_point::EntryPoint::simulate_validation) method of the [entry_point](crate::entry_point::EntryPoint).
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to simulate validation on.
    ///
    /// # Returns
    /// A [SimulateValidationResult](crate::entry_point::SimulateValidationResult) if the simulation was successful, otherwise a [SimulationCheckError](crate::simulation::SimulationCheckError).
    async fn simulate_validation(
        &self,
        uo: &UserOperation,
    ) -> Result<SimulateValidationResult, SimulationCheckError> {
        match self.entry_point.simulate_validation(uo.clone()).await {
            Ok(res) => Ok(res),
            Err(err) => match err {
                EntryPointErr::FailedOp(f) => {
                    Err(SimulationCheckError::Validation { message: f.reason })
                }
                _ => Err(SimulationCheckError::UnknownError {
                    message: format!(
                        "Unknown error when simulating validation on entry point. Error message: {err:?}"
                    ),
                }),
            },
        }
    }

    /// Simulates validation of a [UserOperation](UserOperation) via the [simulate_validation_trace](crate::entry_point::EntryPoint::simulate_validation_trace) method of the [entry_point](crate::entry_point::EntryPoint)
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to simulate validation on.
    ///
    /// # Returns
    /// A [GethTrace](ethers::types::GethTrace) if the simulation was successful, otherwise a [SimulationCheckError](crate::simulation::SimulationCheckError).
    async fn simulate_validation_trace(
        &self,
        uo: &UserOperation,
    ) -> Result<GethTrace, SimulationCheckError> {
        match self.entry_point.simulate_validation_trace(uo.clone()).await {
            Ok(trace) => Ok(trace),
            Err(err) => match err {
                EntryPointErr::FailedOp(f) => {
                    Err(SimulationCheckError::Validation { message: f.reason })
                }
                _ => Err(SimulationCheckError::UnknownError {
                    message: format!(
                        "Unknown error when simulating validation on entry point. Error message: {err:?}"
                    ),
                }),
            },
        }
    }

    /// Pushes a [SanityCheck](SanityCheck) to the sanity_checks array.
    ///
    /// # Arguments
    /// `sanity_check` - [SanityCheck](SanityCheck) to push.
    ///
    /// # Returns
    /// A reference to [self](StandardUserOperationValidator).
    pub fn with_sanity_check(
        mut self,
        sanity_check: impl SanityCheck<M, P, R, E> + 'static,
    ) -> Self {
        self.sanity_checks.push(Box::new(sanity_check));
        self
    }

    /// Pushes a [SimulationCheck](SimulationCheck) to the simulation_checks array.
    ///
    /// # Arguments
    /// `simulation_check` - [SimulationCheck](SimulationCheck) to push.
    ///
    /// # Returns
    /// A reference to [self](StandardUserOperationValidator).
    pub fn with_simulation_check(
        mut self,
        simulation_check: impl SimulationCheck + 'static,
    ) -> Self {
        self.simulation_checks.push(Box::new(simulation_check));
        self
    }

    pub fn with_simulation_trace_check(
        mut self,
        simulation_trace_check: impl SimulationTraceCheck<M, P, R, E> + 'static,
    ) -> Self {
        self.simulation_trace_checks
            .push(Box::new(simulation_trace_check));
        self
    }
}

#[async_trait::async_trait]
impl<M: Middleware + Clone + 'static, P, R, E> UserOperationValidator<P, R, E>
    for StandardUserOperationValidator<M, P, R, E>
where
    P: Mempool<UserOperations = VecUo, CodeHashes = VecCh, Error = E> + Send + Sync,
    R: Rep<ReputationEntries = Vec<ReputationEntry>, Error = E> + Send + Sync,
    E: Debug + Display,
{
    /// Validates a [UserOperation](UserOperation) via the [simulate_validation](crate::entry_point::EntryPoint::simulate_validation) method of the [entry_point](crate::entry_point::EntryPoint).
    /// The function also optionally performs sanity checks and simulation checks if the [UserOperationValidatorMode](UserOperationValidatorMode) contains the respective flags.
    ///
    /// # Arguments
    /// `uo` - [UserOperation](UserOperation) to validate.
    /// `mempool` - [MempoolBox](crate::mempool::MempoolBox) to check for duplicate [UserOperation](UserOperation)s.
    /// `reputation` - [ReputationBox](crate::reputation::ReputationBox).
    /// `mode` - [UserOperationValidatorMode](UserOperationValidatorMode) flag.
    ///
    /// # Returns
    /// A [UserOperationValidationOutcome](UserOperationValidationOutcome) if the validation was successful, otherwise a [ValidationError](ValidationError).
    async fn validate_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &MempoolBox<VecUo, VecCh, P, E>,
        reputation: &ReputationBox<Vec<ReputationEntry>, R, E>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, ValidationError> {
        let mut out: UserOperationValidationOutcome = Default::default();

        if !self.sanity_checks.is_empty() && mode.contains(UserOperationValidatorMode::Sanity) {
            let sanity_helper = SanityHelper {
                mempool,
                reputation,
                entry_point: self.entry_point.clone(),
                chain: self.chain,
            };

            for sanity_check in self.sanity_checks.iter() {
                sanity_check
                    .check_user_operation(uo, &sanity_helper)
                    .await?;
            }
        }

        if let Some(uo) = mempool.get_prev_by_sender(uo) {
            out.prev_hash = Some(uo.hash(&self.entry_point.address(), &self.chain.id().into()));
        }

        let sim_res = self.simulate_validation(uo).await?;

        if !self.simulation_checks.is_empty()
            && mode.contains(UserOperationValidatorMode::Simulation)
        {
            let mut sim_helper = SimulationHelper {
                simulate_validation_result: &sim_res,
                valid_after: None,
            };

            for sim_check in self.simulation_checks.iter() {
                sim_check.check_user_operation(uo, &mut sim_helper)?;
            }

            out.valid_after = sim_helper.valid_after;
        }

        out.pre_fund = extract_pre_fund(&sim_res);
        out.verification_gas_limit = extract_verification_gas_limit(&sim_res);

        if !self.simulation_trace_checks.is_empty()
            && mode.contains(UserOperationValidatorMode::SimulationTrace)
        {
            let geth_trace = self.simulate_validation_trace(uo).await?;
            let js_trace: JsTracerFrame = JsTracerFrame::try_from(geth_trace).map_err(|error| {
                SimulationCheckError::Validation {
                    message: error.to_string(),
                }
            })?;

            let mut sim_helper = SimulationTraceHelper {
                mempool,
                reputation,
                entry_point: self.entry_point.clone(),
                chain: self.chain,
                simulate_validation_result: &sim_res,
                js_trace: &js_trace,
                stake_info: None,
                code_hashes: None,
            };

            for sim_check in self.simulation_trace_checks.iter() {
                sim_check.check_user_operation(uo, &mut sim_helper).await?;
            }

            out.code_hashes = sim_helper.code_hashes;
            out.storage_map = Some(extract_storage_map(&js_trace));
        }

        Ok(out)
    }
}
