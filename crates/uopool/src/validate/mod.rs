//! User operation validator module provides all the necessary traits and types for validations.
use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, ReputationEntryOp},
    Reputation,
};
use alloy_chains::Chain;
use enumset::{EnumSet, EnumSetType};
use ethers::{providers::Middleware, types::U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame, EntryPoint};
use silius_primitives::{
    consts::entities::NUMBER_LEVELS,
    reputation::StakeInfo,
    sanity::SanityCheckError,
    simulation::{CodeHash, SimulationCheckError, StorageMap},
    uopool::ValidationError,
    UserOperation, UserOperationHash,
};

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
/// The [UserOperationValidator](UserOperationValidator) is a composable trait that allows bundler to choose validation rules(sanity, simultation, simulation trace) to apply.
#[async_trait::async_trait]
pub trait UserOperationValidator: Send + Sync {
    async fn validate_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
        reputation: &Reputation<H, R>,
        mode: EnumSet<UserOperationValidatorMode>,
    ) -> Result<UserOperationValidationOutcome, ValidationError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp;
}

/// The [UserOperation](UserOperation) sanity check helper trait.
pub struct SanityHelper<'a, M: Middleware + 'static> {
    entry_point: &'a EntryPoint<M>,
    chain: Chain,
}

#[async_trait::async_trait]
pub trait SanityCheck<M: Middleware>: Send + Sync {
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
        reputation: &Reputation<H, R>,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp;
}

macro_rules! sanity_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<M: Middleware,  $($name : SanityCheck<M>,)+ > SanityCheck<M> for ($($name,)+)
        {
            async fn check_user_operation<T, Y, X, Z, H, R>(
                &self,
                uo: &UserOperation,
                mempool: &Mempool<T, Y, X, Z>,
                reputation: &Reputation<H, R>,
                helper: &SanityHelper<M>,
            ) -> Result<(), SanityCheckError>
            where
                T: UserOperationAct,
                Y: UserOperationAddrAct,
                X: UserOperationAddrAct,
                Z: UserOperationCodeHashAct,
                H: HashSetOp,
                R: ReputationEntryOp,
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
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        _helper: &SanityHelper<M>,
    ) -> Result<(), SanityCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        Ok(())
    }
}

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

/// The [UserOperation](UserOperation) simulation check helper trait.
pub struct SimulationHelper<'a> {
    simulate_validation_result: &'a SimulateValidationResult,
    valid_after: Option<U256>,
}

pub trait SimulationCheck: Send + Sync {
    fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationHelper,
    ) -> Result<(), SimulationCheckError>;
}
macro_rules! simulation_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<$($name : SimulationCheck,)+ > SimulationCheck for ($($name,)+)
        {
            fn check_user_operation(
                &self,
                uo: &UserOperation,
                helper: &mut SimulationHelper,
            ) -> Result<(), SimulationCheckError>
                {
                    let ($($name,)+) = self;
                    ($($name.check_user_operation(uo, helper)?,)+);
                    Ok(())
                }
        }
    };
}
simulation_check_impls! {A}
simulation_check_impls! {A B}
simulation_check_impls! {A B C}
simulation_check_impls! {A B C D}
simulation_check_impls! {A B C D E}
simulation_check_impls! {A B C D E F}

/// The [UserOperation](UserOperation) simulation trace check helper trait.
pub struct SimulationTraceHelper<'a, M: Middleware + Send + Sync + 'static> {
    entry_point: &'a EntryPoint<M>,
    chain: Chain,
    simulate_validation_result: &'a SimulateValidationResult,
    js_trace: &'a JsTracerFrame,
    stake_info: Option<[StakeInfo; NUMBER_LEVELS]>,
    code_hashes: Option<Vec<CodeHash>>,
}

#[async_trait::async_trait]
pub trait SimulationTraceCheck<M: Middleware>: Send + Sync {
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
        reputation: &Reputation<H, R>,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp;
}
macro_rules! simulation_trace_check_impls {
    ( $( $name:ident )+ ) => {
        #[allow(non_snake_case)]
        #[async_trait::async_trait]
        impl<M: Middleware, $($name : SimulationTraceCheck<M>,)+> SimulationTraceCheck<M> for ($($name,)+)
        {
            async fn check_user_operation<T, Y, X, Z, H, R>(
                &self,
                uo: &UserOperation,
                mempool: &Mempool<T, Y, X, Z>,
                reputation: &Reputation<H, R>,
                helper: &mut SimulationTraceHelper<M>,
            ) -> Result<(), SimulationCheckError>
            where
                T: UserOperationAct,
                Y: UserOperationAddrAct,
                X: UserOperationAddrAct,
                Z: UserOperationCodeHashAct,
                H: HashSetOp,
                R: ReputationEntryOp,
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
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        _uo: &UserOperation,
        _mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
        _helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        Ok(())
    }
}
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
