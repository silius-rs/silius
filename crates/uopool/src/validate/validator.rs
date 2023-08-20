use super::{
    utils::{extract_pre_fund, extract_storage_map, extract_verification_gas_limit},
    SanityCheck, SanityHelper, SimulationCheck, SimulationHelper, SimulationTraceCheck,
    SimulationTraceHelper, UserOperationValidationOutcome, UserOperationValidator,
    UserOperationValidatorMode,
};
use crate::{
    mempool::MempoolBox,
    reputation::ReputationBox,
    uopool::{VecCh, VecUo},
};
use enumset::EnumSet;
use ethers::{providers::Middleware, types::GethTrace};
use silius_contracts::{
    entry_point::{EntryPointErr, SimulateValidationResult},
    tracer::JsTracerFrame,
    EntryPoint,
};
use silius_primitives::{
    reputation::ReputationEntry, simulation::SimulationCheckError, uopool::ValidationError, Chain,
    UserOperation,
};
use std::sync::Arc;

/// Standard implementation of [UserOperationValidator](UserOperationValidator).
pub struct StandardUserOperationValidator<M: Middleware + Clone + 'static> {
    /// Ethereum exeuctionclient.
    eth_client: Arc<M>,
    /// The [EntryPoint](EntryPoint) object.
    entry_point: EntryPoint<M>,
    /// A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    chain: Chain,
    /// An array of [SanityChecks](SanityCheck).
    sanity_checks: Vec<Box<dyn SanityCheck<M>>>,
    /// An array of [SimulationCheck](SimulationCheck).
    simulation_checks: Vec<Box<dyn SimulationCheck<M>>>,
    /// An array of [SimulationTraceChecks](SimulationTraceCheck).
    simulation_trace_checks: Vec<Box<dyn SimulationTraceCheck<M>>>,
}

impl<M: Middleware + Clone + 'static> StandardUserOperationValidator<M> {
    /// Creates a new [StandardUserOperationValidator](StandardUserOperationValidator).
    ///
    /// # Arguments
    /// `eth_client` - Ethereum exeuction client.
    /// `entry_point` - [EntryPoint](EntryPoint) object.
    /// `chain` - A [EIP-155](https://eips.ethereum.org/EIPS/eip-155) chain ID.
    ///
    /// # Returns
    /// A new [StandardUserOperationValidator](StandardUserOperationValidator).
    pub fn new(eth_client: Arc<M>, entry_point: EntryPoint<M>, chain: Chain) -> Self {
        Self {
            eth_client,
            entry_point,
            chain,
            sanity_checks: vec![],
            simulation_checks: vec![],
            simulation_trace_checks: vec![],
        }
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
    pub fn with_sanity_check(mut self, sanity_check: impl SanityCheck<M> + 'static) -> Self {
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
        simulation_check: impl SimulationCheck<M> + 'static,
    ) -> Self {
        self.simulation_checks.push(Box::new(simulation_check));
        self
    }

    pub fn with_simulation_trace_check(
        mut self,
        simulation_trace_check: impl SimulationTraceCheck<M> + 'static,
    ) -> Self {
        self.simulation_trace_checks
            .push(Box::new(simulation_trace_check));
        self
    }
}

#[async_trait::async_trait]
impl<M: Middleware + Clone + 'static> UserOperationValidator for StandardUserOperationValidator<M> {
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
        mempool: &MempoolBox<VecUo, VecCh>,
        reputation: &ReputationBox<Vec<ReputationEntry>>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, ValidationError> {
        let mut out: UserOperationValidationOutcome = Default::default();

        if !self.sanity_checks.is_empty() && mode.contains(UserOperationValidatorMode::Sanity) {
            let mut sanity_helper = SanityHelper {
                mempool,
                reputation,
                eth_client: self.eth_client.clone(),
                entry_point: self.entry_point.clone(),
                chain: self.chain,
            };

            for sanity_check in self.sanity_checks.iter() {
                sanity_check
                    .check_user_operation(uo, &mut sanity_helper)
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
                mempool,
                reputation,
                eth_client: self.eth_client.clone(),
                entry_point: self.entry_point.clone(),
                chain: self.chain,
                simulate_validation_result: &sim_res,
                valid_after: None,
            };

            for sim_check in self.simulation_checks.iter() {
                sim_check.check_user_operation(uo, &mut sim_helper).await?;
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
                eth_client: self.eth_client.clone(),
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
