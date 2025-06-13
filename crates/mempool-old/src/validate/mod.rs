//! User operation validator module provides all the necessary traits and types for validations.
use crate::{
    mempool::Mempool, InvalidMempoolUserOperationError, Reputation, SanityError, SimulationError,
};
use alloy_chains::Chain;
use enumset::{EnumSet, EnumSetType};
use ethers::{providers::Middleware, types::U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame, EntryPoint};
use silius_primitives::{
    constants::validation::entities::NUMBER_OF_LEVELS,
    reputation::StakeInfo,
    simulation::{CodeHash, StorageMap, ValidationConfig},
    UserOperation, UserOperationHash,
};

pub mod sanity;
pub mod simulation;
pub mod simulation_trace;
pub mod utils;
pub mod validator;

/// The outcome of a user operation validation.
#[derive(Debug, Clone, Default)]
pub struct UserOperationValidationOutcome {
    // which validation config was used
    pub val_config: ValidationConfig,
    pub prev_hash: Option<UserOperationHash>,
    pub pre_fund: U256,
    pub verification_gas_limit: U256,
    // Simulation
    pub valid_after: Option<U256>,
    // Simulation trace
    pub code_hashes: Option<Vec<CodeHash>>,
    pub storage_map: StorageMap,
    // the block which the user operation is verified on
    pub verified_block: U256,
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
/// The [UserOperationValidator](UserOperationValidator) is a composable trait that allows bundler
/// to choose validation rules(sanity, simultation, simulation trace) to apply.
#[async_trait::async_trait]
pub trait UserOperationValidator: Send + Sync {
    async fn validate_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        reputation: &Reputation,
        val_config: Option<ValidationConfig>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, InvalidMempoolUserOperationError>;
}

/// The [UserOperation] sanity check helper trait.
pub struct SanityHelper<'a, M: Middleware + 'static> {
    entry_point: &'a EntryPoint<M>,
    chain: Chain,
    val_config: ValidationConfig,
}

#[async_trait::async_trait]
pub trait SanityCheck<M: Middleware>: Send + Sync {
    /// Performs a sanity check on a user operation.
    ///
    /// This method checks the validity of a user operation by verifying it against the mempool,
    /// reputation system, and other sanity checks provided by the `SanityHelper`.
    ///
    /// # Arguments
    ///
    /// * `uo` - The user operation to be checked.
    /// * `mempool` - The mempool to verify the user operation against.
    /// * `reputation` - The reputation system to consider during the sanity check.
    /// * `helper` - The `SanityHelper` instance that provides additional sanity checks.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the user operation passes all sanity checks, otherwise returns a
    /// `SanityError` indicating the reason for failure.
    ///
    /// # Generic Parameters
    ///
    /// * `T` - The type implementing the `UserOperationAct` trait.
    /// * `Y` - The type implementing the `UserOperationAddrAct` trait.
    /// * `X` - The type implementing the `UserOperationAddrAct` trait.
    /// * `Z` - The type implementing the `UserOperationCodeHashAct` trait.
    /// * `H` - The type implementing the `HashSetOp` trait.
    /// * `R` - The type implementing the `ReputationEntryOp` trait.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        reputation: &Reputation,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityError>;
}

macro_rules! sanity_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<M: Middleware, $($name : SanityCheck<M>,)+> SanityCheck<M> for ($($name,)+)
        {
            async fn check_user_operation(
                &self,
                uo: &UserOperation,
                mempool: &Mempool,
                reputation: &Reputation,
                helper: &SanityHelper<M>,
            ) -> Result<(), SanityError>
                {
                    let ($($name,)+) = self;
                    ($($name.check_user_operation(uo, mempool, reputation, helper).await?,)+);
                    Ok(())
                }
        }
    };
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for () {
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        _helper: &SanityHelper<M>,
    ) -> Result<(), SanityError> {
        Ok(())
    }
}

// These macro enable people to chain sanity check implementations:
// `(SanityCheck1, SanityCheck2, SanityCheck3, ...).check_user_operation(uo, mempool, reputation,
// helper)`` SanityCheck1,2,3 could be any data type which implement SanityCheck trait.
sanity_check_impls! { A }
sanity_check_impls! { A B }
sanity_check_impls! { A B C }
sanity_check_impls! { A B C D }
sanity_check_impls! { A B C D F }
sanity_check_impls! { A B C D F G }
sanity_check_impls! { A B C D F G I }
sanity_check_impls! { A B C D F G I J }
sanity_check_impls! { A B C D F G I J K }
sanity_check_impls! { A B C D F G I J K L }

/// The [UserOperation] simulation check helper trait.
pub struct SimulationHelper<'a> {
    simulate_validation_result: &'a SimulateValidationResult,
    val_config: ValidationConfig,
    valid_after: Option<U256>,
}

/// Trait for performing simulation checks on user operations.
pub trait SimulationCheck: Send + Sync {
    /// Checks a user operation against a simulation helper.
    ///
    /// # Arguments
    ///
    /// * `uo` - The user operation to be checked.
    /// * `helper` - The simulation helper to assist in the check.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the user operation passes the simulation check,
    /// otherwise returns a `SimulationError`.
    fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationHelper,
    ) -> Result<(), SimulationError>;
}

macro_rules! simulation_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<$($name : SimulationCheck,)+> SimulationCheck for ($($name,)+)
        {
            fn check_user_operation(
                &self,
                uo: &UserOperation,
                helper: &mut SimulationHelper,
            ) -> Result<(), SimulationError>
                {
                    let ($($name,)+) = self;
                    ($($name.check_user_operation(uo, helper)?,)+);
                    Ok(())
                }
        }
    };
}

// These macro enable people to chain simulation check implementations:
// `(SimulationCheck1, SimulationCheck2, SimulationCheck3, ...).check_user_operation(uo, helper)``
// SimulationCheck1,2,3 could be any data type which implement SimulationCheck trait.
simulation_check_impls! { A }
simulation_check_impls! { A B }
simulation_check_impls! { A B C }
simulation_check_impls! { A B C D }
simulation_check_impls! { A B C D F }
simulation_check_impls! { A B C D F G }
simulation_check_impls! { A B C D F G I }
simulation_check_impls! { A B C D F G I J }
simulation_check_impls! { A B C D F G I J K }
simulation_check_impls! { A B C D F G I J K L }

/// The [UserOperation] simulation trace check helper trait.
pub struct SimulationTraceHelper<'a, M: Middleware + Send + Sync + 'static> {
    entry_point: &'a EntryPoint<M>,
    chain: Chain,
    simulate_validation_result: &'a SimulateValidationResult,
    js_trace: &'a JsTracerFrame,
    val_config: ValidationConfig,
    stake_info: Option<[StakeInfo; NUMBER_OF_LEVELS]>,
    code_hashes: Option<Vec<CodeHash>>,
}

#[async_trait::async_trait]
pub trait SimulationTraceCheck<M: Middleware>: Send + Sync {
    /// Asynchronously checks a user operation against the mempool, reputation, and simulation
    /// trace.
    ///
    /// # Arguments
    ///
    /// * `uo` - The user operation to be checked.
    /// * `mempool` - The mempool containing user operations.
    /// * `reputation` - The reputation data structure.
    /// * `helper` - The simulation trace helper.
    ///
    /// # Generic Parameters
    ///
    /// * `T` - Type implementing the `UserOperationAct` trait.
    /// * `Y` - Type implementing the `UserOperationAddrAct` trait.
    /// * `X` - Type implementing the `UserOperationAddrAct` trait.
    /// * `Z` - Type implementing the `UserOperationCodeHashAct` trait.
    /// * `H` - Type implementing the `HashSetOp` trait.
    /// * `R` - Type implementing the `ReputationEntryOp` trait.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the user operation passes the simulation check, or an error of type
    /// `SimulationError` otherwise.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        mempool: &Mempool,
        reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError>;
}

macro_rules! simulation_trace_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<M: Middleware, $($name : SimulationTraceCheck<M>,)+> SimulationTraceCheck<M> for ($($name,)+)
        {
            async fn check_user_operation(
                &self,
                uo: &UserOperation,
                mempool: &Mempool,
                reputation: &Reputation,
                helper: &mut SimulationTraceHelper<M>,
            ) -> Result<(), SimulationError>
                {
                    let ($($name,)+) = self;
                    ($($name.check_user_operation(uo, mempool, reputation, helper).await?,)+);
                    Ok(())
                }
        }
    };
}

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for () {
    async fn check_user_operation(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        _helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        Ok(())
    }
}

// These macro enable people to chain simulation check implementations:
// `(SimulationTraceCheck1, SimulationTraceCheck2, SimulationTraceCheck3,
// ...).check_user_operation(uo, mempool, reputeation helper)`` SimulationTraceCheck1,2,3 could be
// any data type which implement SimulationTraceCheck trait.
simulation_trace_check_impls! { A }
simulation_trace_check_impls! { A B }
simulation_trace_check_impls! { A B C }
simulation_trace_check_impls! { A B C D }
simulation_trace_check_impls! { A B C D F }
simulation_trace_check_impls! { A B C D F G }
simulation_trace_check_impls! { A B C D F G I }
simulation_trace_check_impls! { A B C D F G I J }
simulation_trace_check_impls! { A B C D F G I J K }
simulation_trace_check_impls! { A B C D F G I J K L }
