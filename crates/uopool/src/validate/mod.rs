//! User operation validator module provides all the necessary traits and types for validations.
use crate::{
    mempool::MempoolBox,
    reputation::ReputationBox,
    uopool::{VecCh, VecUo},
};
use enumset::{EnumSet, EnumSetType};
use ethers::{providers::Middleware, types::U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame, EntryPoint};
use silius_primitives::{
    consts::entities::NUMBER_LEVELS,
    reputation::{ReputationEntry, StakeInfo},
    sanity::SanityCheckError,
    simulation::{CodeHash, SimulationCheckError, StorageMap},
    uopool::ValidationError,
    Chain, UserOperation, UserOperationHash,
};
use std::sync::Arc;

pub mod sanity;
pub mod simulation;
pub mod simulation_trace;
mod utils;
pub mod validator;

/// The outcome of a user operation validation.
#[derive(Debug, Clone, Default)]
pub struct UserOperationValidationOutcome {
    pub prev_hash: Option<UserOperationHash>,
    pub pre_fund: U256,
    pub verification_gas_limit: U256,
    // Simulation
    pub valid_after: Option<U256>,
    // Simulation trace
    pub code_hashes: Option<Vec<CodeHash>>,
    pub storage_map: Option<StorageMap>,
}

/// The mode in which the user operation validator is running.
/// The validator has three modes: sanity, simulation, and simulation trace.
#[derive(EnumSetType, Debug)]
pub enum UserOperationValidatorMode {
    Sanity,
    Simulation,
    SimulationTrace,
}

/// The [UserOperation](UserOperation) validator trait.
/// The [UserOperationValidator](UserOperationValidator) is a composable trait that allows bundler to choose validation rules(sanity, simultation, simulation trace) to apply.
#[async_trait::async_trait]
pub trait UserOperationValidator: Send + Sync {
    async fn validate_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &MempoolBox<VecUo, VecCh>,
        reputation: &ReputationBox<Vec<ReputationEntry>>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, ValidationError>;
}

/// The [UserOperation](UserOperation) sanity check helper trait.
pub struct SanityHelper<'a, M: Middleware + 'static> {
    mempool: &'a MempoolBox<VecUo, VecCh>,
    reputation: &'a ReputationBox<Vec<ReputationEntry>>,
    eth_client: Arc<M>,
    entry_point: EntryPoint<M>,
    chain: Chain,
}

#[async_trait::async_trait]
pub trait SanityCheck<M: Middleware>: Send + Sync {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SanityHelper<M>,
    ) -> Result<(), SanityCheckError>;
}

/// The [UserOperation](UserOperation) simulation check helper trait.
pub struct SimulationHelper<'a, M: Middleware + 'static> {
    mempool: &'a MempoolBox<VecUo, VecCh>,
    reputation: &'a ReputationBox<Vec<ReputationEntry>>,
    eth_client: Arc<M>,
    entry_point: EntryPoint<M>,
    chain: Chain,
    simulate_validation_result: &'a SimulateValidationResult,
    valid_after: Option<U256>,
}

#[async_trait::async_trait]
pub trait SimulationCheck<M: Middleware>: Send + Sync {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationHelper<M>,
    ) -> Result<(), SimulationCheckError>;
}

/// The [UserOperation](UserOperation) simulation trace check helper trait.
pub struct SimulationTraceHelper<'a, M: Middleware + 'static> {
    mempool: &'a MempoolBox<VecUo, VecCh>,
    reputation: &'a ReputationBox<Vec<ReputationEntry>>,
    eth_client: Arc<M>,
    entry_point: EntryPoint<M>,
    chain: Chain,
    simulate_validation_result: &'a SimulateValidationResult,
    js_trace: &'a JsTracerFrame,
    stake_info: Option<[StakeInfo; NUMBER_LEVELS]>,
    code_hashes: Option<Vec<CodeHash>>,
}

#[async_trait::async_trait]
pub trait SimulationTraceCheck<M: Middleware>: Send + Sync {
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError>;
}
