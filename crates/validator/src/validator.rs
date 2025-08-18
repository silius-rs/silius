use std::collections::HashMap;

use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use silius_chain::{
    Chain,
    error::{ChainError, RevertReason, decode_revert_reason},
};
use silius_primitives::{network_spec::network_spec, user_operation::UserOperation};
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    error::ValidationError,
    sanity_checks::{SanityCheck, fee::FeeCheck},
    tracing_checks::{
        TracingCheck, TracingContext, opcode::OpcodeCheck, storage::StorageCheck,
        utils::extract_validation_result,
    },
    types::ValidationResult,
};

pub const VALID_UNTIL_SECONDS_IN_FUTURE: u64 = 30; // 30 seconds

pub struct Validator<P: Provider> {
    pub config: ValidatorConfig,
    pub sanity_checks: Vec<Box<dyn SanityCheck<P> + Send + Sync>>,
    pub tracing_checks: Vec<Box<dyn TracingCheck<P> + Send + Sync>>,
}

impl<P: Provider> Validator<P> {
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![],
            tracing_checks: vec![],
        }
    }

    pub fn standard(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![Box::new(FeeCheck)],
            tracing_checks: vec![Box::new(OpcodeCheck), Box::new(StorageCheck)],
        }
    }

    pub fn _unsafe(config: ValidatorConfig) -> Self {
        Self {
            config,
            sanity_checks: vec![Box::new(FeeCheck)],
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

        let (frame, validation_result) = if !self.tracing_checks.is_empty() {
            let frame = chain.trace_handle_ops(user_operation).await?;
            let validation_result = extract_validation_result(&frame);
            (frame, validation_result)
        } else {
            match chain.simulate_handle_ops(user_operation).await {
                Ok(_) => (
                    Erc7562Frame::default(),
                    ValidationResult {
                        pre_op_gas: user_operation.pre_verification_gas
                            + user_operation.verification_gas_limit
                            + user_operation
                                .paymaster_verification_gas_limit
                                .unwrap_or_default(),
                        ..Default::default()
                    },
                ),
                Err(error) => match error {
                    ChainError::Revert(data) => {
                        let (frame, mut validation_result) =
                            (Erc7562Frame::default(), ValidationResult::default());
                        let revert_reason = decode_revert_reason(data);
                        match revert_reason {
                            RevertReason::InvalidSignature => {
                                validation_result.sig_failed = true;
                            }
                            RevertReason::InvalidPaymasterSignature => {
                                validation_result.paymaster_sig_failed = true;
                            }
                            RevertReason::False => {}
                            _ => {
                                return Err(ValidationError::Other(
                                    "Unknown revert reason".to_string(),
                                ));
                            }
                        }
                        (frame, validation_result)
                    }
                    _ => return Err(ValidationError::ChainError(error)),
                },
            }
        };

        self._validate_validation_result(&validation_result)?;

        // TODO: add preverification gas check

        for sanity_check in &self.sanity_checks {
            sanity_check
                .check_user_operation(user_operation, &self.config, db, chain)
                .await?;
        }

        if !self.tracing_checks.is_empty() {
            let mut context = TracingContext::default();
            context.validation_result = validation_result;
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

    fn _validate_validation_result(
        &self,
        result: &ValidationResult,
    ) -> Result<(), ValidationError> {
        // TODO: check preverfication gas (this is second time, add first time as well)

        if result.sig_failed {
            return Err(ValidationError::Signature(
                "AA24: Invalid user operation signature".to_string(),
            ));
        }
        if result.paymaster_sig_failed {
            return Err(ValidationError::Signature(
                "AA34: Invalid paymaster signature".to_string(),
            ));
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| ValidationError::Other(e.to_string()))?
            .as_secs();

        if result.valid_after > current_time {
            return Err(ValidationError::NotInTimeRange(format!(
                "time range in the future: current time {} is before valid after {}",
                current_time, result.valid_after
            )));
        }

        if result.valid_until != 0 && result.valid_until < current_time {
            return Err(ValidationError::NotInTimeRange(format!(
                "time range in the past: current time {} is after valid until {}",
                current_time, result.valid_until
            )));
        }

        if result.valid_until != 0
            && result.valid_until < current_time + VALID_UNTIL_SECONDS_IN_FUTURE
        {
            return Err(ValidationError::NotInTimeRange(format!(
                "time range too short: current time {} is after valid until {}, allowed {} seconds in the future",
                current_time, result.valid_until, VALID_UNTIL_SECONDS_IN_FUTURE
            )));
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
        if frame.from == network_spec().entry_point_address
            && frame.to == Some(network_spec().entry_point_address)
        {
            return Ok(());
        }

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
