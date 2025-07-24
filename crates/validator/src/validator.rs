use std::collections::HashMap;

use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use silius_chain::Chain;
use silius_primitives::user_operation::UserOperation;
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    error::ValidationError,
    sanity_checks::{SanityCheck, fee::FeeCheck},
    simulation_checks::SimulationCheck,
    tracing_check::{TracingCheck, TracingContext, opcode::OpcodeCheck, storage::StorageCheck},
};

pub struct Validator<P: Provider> {
    pub config: ValidatorConfig,
    pub sanity_checks: Vec<Box<dyn SanityCheck<P> + Send + Sync>>,
    pub simulation_checks: Vec<Box<dyn SimulationCheck<P> + Send + Sync>>,
    pub tracing_checks: Vec<Box<dyn TracingCheck<P> + Send + Sync>>,
}

impl<P: Provider> Validator<P> {
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn standard(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![Box::new(FeeCheck)],
            simulation_checks: vec![],
            tracing_checks: vec![Box::new(OpcodeCheck), Box::new(StorageCheck)],
        }
    }

    pub fn _unsafe(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            simulation_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub async fn validate_user_operation(
        &self,
        user_operation: &UserOperation,
        db: &SiliusDB,
        chain: &Chain<P>,
    ) -> Result<(), ValidationError> {
        // TODO: check if ok to use is_staked or multiply stake amount and price of asset (more than 1000 USD)
        let mut entity_staked = HashMap::new();
        entity_staked.insert(
            user_operation.sender,
            chain
                .get_deposit_info(user_operation.sender)
                .await?
                .is_staked(),
        );
        if let Some(paymaster) = user_operation.paymaster {
            entity_staked.insert(
                paymaster,
                chain.get_deposit_info(paymaster).await?.is_staked(),
            );
        }
        if let Some(factory) = user_operation.factory {
            entity_staked.insert(factory, chain.get_deposit_info(factory).await?.is_staked());
        }

        for sanity_check in &self.sanity_checks {
            sanity_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        for simulation_check in &self.simulation_checks {
            simulation_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        if !self.tracing_checks.is_empty() {
            let frame = chain.trace_handle_ops(user_operation).await?;

            let mut context = TracingContext::default();
            context.entity_staked = entity_staked;
            context.keccak = frame.keccak.clone();

            self._tracing_check_recursive(
                user_operation,
                &self.config,
                db,
                chain,
                &frame,
                &mut context,
            )
            .await?;
        }

        Ok(())
    }

    async fn _tracing_check_recursive(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
        frame: &Erc7562Frame,
        context: &mut TracingContext,
    ) -> Result<(), ValidationError> {
        context.update(user_operation, frame);

        for tracing_check in &self.tracing_checks {
            tracing_check
                .check_user_operation(user_operation, config, db, chain, frame, context)
                .await?;
        }

        for call in frame.calls.iter() {
            Box::pin(self._tracing_check_recursive(
                user_operation,
                config,
                db,
                chain,
                call,
                context,
            ))
            .await?;
        }

        Ok(())
    }
}
