use std::collections::{HashMap, HashSet};

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_provider::Provider;
use alloy_rpc_types_trace::geth::erc7562::Erc7562Frame;
use silius_chain::Chain;
use silius_primitives::{network_spec::network_spec, user_operation::UserOperation};
use silius_storage::db::SiliusDB;

use crate::{
    config::ValidatorConfig,
    tracing_check::{TracingCheck, TracingContext, error::TracingCheckError},
};

pub struct StorageCheck;

#[async_trait::async_trait]
impl<P: Provider> TracingCheck<P> for StorageCheck {
    async fn check_user_operation(
        &self,
        user_operation: &UserOperation,
        _config: &ValidatorConfig,
        _db: &SiliusDB,
        _chain: &Chain<P>,
        frame: &Erc7562Frame,
        context: &TracingContext,
    ) -> Result<(), TracingCheckError> {
        let to = if let Some(to) = frame.to
            && to != network_spec().entry_point_address
        {
            to
        } else {
            return Ok(());
        };

        let slots = [
            frame.accessed_slots.writes.keys().collect::<Vec<_>>(),
            frame.accessed_slots.reads.keys().collect::<Vec<_>>(),
            frame
                .accessed_slots
                .transient_writes
                .keys()
                .collect::<Vec<_>>(),
            frame
                .accessed_slots
                .transient_reads
                .keys()
                .collect::<Vec<_>>(),
        ]
        .concat();

        let address_slots = _parse_slots(
            &[
                user_operation.sender,
                user_operation.factory.unwrap_or_default(),
                user_operation.paymaster.unwrap_or_default(),
            ],
            &context.keccak,
        );

        let is_entity_staked = context.is_current_entity_staked();
        let is_factory_staked =
            context.is_entity_staked(user_operation.factory.unwrap_or_default());
        let is_factory = user_operation.factory.is_some();

        for slot in slots {
            let is_sender = to == user_operation.sender;
            let is_sender_associated =
                _associated_with(slot, &user_operation.sender, &address_slots);
            let is_entity = to == context.current_entity_address;
            let is_entity_associated =
                _associated_with(slot, &context.current_entity_address, &address_slots);
            let is_read_only_access = frame.accessed_slots.writes.is_empty()
                && frame.accessed_slots.transient_writes.is_empty();

            let is_allowed_entity_staked = is_entity || is_entity_associated || is_read_only_access;
            let is_allowed_factory_staked = is_sender_associated && is_factory;

            let is_allowed = (is_allowed_entity_staked && is_entity_staked)
                || (is_sender_associated && !is_factory)
                || (is_allowed_factory_staked && is_factory_staked);

            if is_sender || is_allowed {
                continue;
            }

            return Err(
                if (is_allowed_entity_staked && !is_entity_staked)
                    || (is_allowed_factory_staked && !is_factory_staked)
                {
                    TracingCheckError::UnstakedEntitySlotAccess(
                        context.current_entity,
                        to,
                        slot.clone(),
                    )
                } else {
                    TracingCheckError::ForbiddenSlotAccess(
                        context.current_entity,
                        String::from(
                            if frame.accessed_slots.writes.contains_key(slot)
                                || frame.accessed_slots.transient_writes.contains_key(slot)
                            {
                                "write to"
                            } else {
                                "read from"
                            },
                        ),
                        String::from(
                            if frame.accessed_slots.transient_reads.contains_key(slot)
                                || frame.accessed_slots.transient_writes.contains_key(slot)
                            {
                                "transient"
                            } else {
                                ""
                            },
                        ),
                        to,
                        slot.clone(),
                    )
                },
            );
        }

        Ok(())
    }
}

fn _parse_slots(addresses: &[Address], keccak: &Vec<Bytes>) -> HashMap<Address, HashSet<B256>> {
    let mut slots: HashMap<Address, HashSet<B256>> = HashMap::new();

    for k in keccak {
        for address in addresses {
            if address.is_zero() {
                continue;
            }

            let address_padded = Bytes::from([vec![0; 12], address.to_vec().into()].concat());

            if k.starts_with(&address_padded) {
                let k = keccak256(k);
                slots.entry(*address).or_default().insert(k.into());
            }
        }
    }

    slots
}

fn _associated_with(
    slot: &B256,
    address: &Address,
    address_slots: &HashMap<Address, HashSet<B256>>,
) -> bool {
    if slot.ends_with(address.to_vec().as_slice()) {
        return true;
    }

    let slots = if let Some(slots) = address_slots.get(address) {
        slots
    } else {
        return false;
    };

    let slot_number = U256::from_be_bytes(slot.0);

    for slot in slots {
        let slot_num = U256::from_be_bytes(slot.0);

        if slot_number >= slot_num && slot_number < (slot_num + U256::from(128)) {
            return true;
        }
    }

    false
}
