use alloy_primitives::{Address, Bytes};
use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use silius_chain::Chain;
use silius_primitives::{
    entity::Entity, network_spec::network_spec, user_operation::UserOperation,
};
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig, tracing_checks::error::TracingCheckError, types::ValidationResult,
};

pub mod code;
pub mod error;
pub mod opcode;
pub mod storage;
pub mod utils;

#[derive(Default)]
pub struct TracingContext {
    pub current_entity: Entity,
    pub current_entity_address: Address,
    pub validation_result: ValidationResult,
    pub keccak: Vec<Bytes>,
}

impl TracingContext {
    pub fn update(&mut self, user_operation: &UserOperation, frame: &Erc7562Frame) {
        if frame.from != Address::ZERO && frame.from != network_spec().entry_point_address {
            return;
        }

        if let Some(to) = frame.to {
            if user_operation.sender == to {
                self.current_entity = Entity::Account;
                self.current_entity_address = user_operation.sender;
            } else if let Some(factory) = user_operation.factory
                && factory == to
            {
                self.current_entity = Entity::Factory;
                self.current_entity_address = factory;
            } else if let Some(paymaster) = user_operation.paymaster
                && paymaster == to
            {
                self.current_entity = Entity::Paymaster;
                self.current_entity_address = paymaster;
            } else if to == network_spec().entry_point_address {
                self.current_entity = Entity::EntryPoint;
                self.current_entity_address = to;
            } else {
                self.current_entity = Entity::Unknown;
            }
        }
    }

    pub fn is_current_entity_staked(&self) -> bool {
        self.is_entity_staked(self.current_entity_address)
    }

    pub fn is_entity_staked(&self, address: Address) -> bool {
        self.validation_result
            .entity_staked
            .get(&address)
            .copied()
            .unwrap_or(false)
    }
}

#[async_trait::async_trait]
pub trait TracingCheck<P: Provider> {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        config: &ValidatorConfig,
        db: &SiliusDB,
        chain: &Chain<P>,
        frame: &Erc7562Frame,
        helper: &TracingContext,
    ) -> Result<(), TracingCheckError>;
}
