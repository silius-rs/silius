use crate::{
    mempool::Mempool,
    validate::{utils::extract_stake_info, SimulationTraceCheck, SimulationTraceHelper},
    Reputation, SimulationError,
};
use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
    utils::keccak256,
};
use silius_contracts::entry_point::SELECTORS_INDICES;
use silius_primitives::{
    constants::validation::entities::{FACTORY_LEVEL, LEVEL_TO_ENTITY, NUMBER_OF_LEVELS},
    reputation::StakeInfo,
    UserOperation,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct StorageAccess;

impl StorageAccess {
    /// The helper method that parses the slots from the JS trace.
    ///
    /// # Arguments
    /// `keccak` - The keccak of the JS trace
    /// `info` - The stake info
    /// `slots` - The slots to parse
    ///
    /// # Returns
    /// None
    fn parse_slots(
        &self,
        keccak: Vec<Bytes>,
        info: &[StakeInfo; NUMBER_OF_LEVELS],
        slots: &mut HashMap<Address, HashSet<Bytes>>,
    ) {
        for kecc in keccak {
            for entity in info {
                if entity.address.is_zero() {
                    continue;
                }

                let addr_b =
                    Bytes::from([vec![0; 12], entity.address.to_fixed_bytes().to_vec()].concat());

                if kecc.starts_with(&addr_b) {
                    let k = keccak256(kecc.clone());
                    slots.entry(entity.address).or_default().insert(k.into());
                }
            }
        }
    }

    /// The helper method that checks if the slot is associated with the address.
    ///
    /// # Arguments
    /// `addr` - The address to check
    /// `slot` - The slot to check
    /// `slots` - The slots to check
    ///
    /// # Returns
    /// true if the slot is associated with the address, otherwise false.
    fn associated_with_slot(
        &self,
        addr: &Address,
        slot: &String,
        slots: &HashMap<Address, HashSet<Bytes>>,
    ) -> Result<bool, SimulationError> {
        if *slot == addr.to_string() {
            return Ok(true);
        }

        if !slots.contains_key(addr) {
            return Ok(false);
        }

        let slot_num = U256::from_str_radix(slot, 16)
            .map_err(|_| SimulationError::StorageAccess { slot: slot.clone() })?;

        if let Some(slots) = slots.get(addr) {
            for slot in slots {
                let slot_ent_num = U256::from(slot.as_ref());

                if slot_num >= slot_ent_num && slot_num < (slot_ent_num + 128) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SimulationTraceCheck<M> for StorageAccess {
    /// The method implementation that checks if the user operation access
    /// storage other than the one associated with itself.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationError] error.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        _mempool: &Mempool,
        _reputation: &Reputation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationError> {
        if helper.stake_info.is_none() {
            helper.stake_info = Some(extract_stake_info(uo, helper.simulate_validation_result));
        }

        let mut slots = HashMap::new();
        self.parse_slots(
            helper.js_trace.keccak.clone(),
            &helper.stake_info.unwrap_or_default(),
            &mut slots,
        );

        let mut slot_staked = String::new();
        let stake_info = helper.stake_info.unwrap_or_default();

        for call_info in helper.js_trace.calls_from_entry_point.iter() {
            let level = SELECTORS_INDICES.get(call_info.top_level_method_sig.as_ref()).cloned();

            if let Some(l) = level {
                let stake_info_l = stake_info[l];

                for (addr, acc) in &call_info.access {
                    // [STO-010] - Access to the "account" storage is always allowed
                    if *addr == uo.sender || *addr == helper.entry_point.address() {
                        continue;
                    }

                    slot_staked.clear();

                    for slot in [
                        acc.reads.keys().cloned().collect::<Vec<String>>(),
                        acc.writes.keys().cloned().collect(),
                    ]
                    .concat()
                    {
                        if self.associated_with_slot(&uo.sender, &slot, &slots)? {
                            // [STO-021], [STO-022] - Access to associated storage of the account in
                            // an external (non-entity contract) is allowed if either The account
                            // already exists or There is an initCode and the factory contract is
                            // staked
                            if !(uo.init_code.is_empty() ||
                                uo.sender == stake_info_l.address &&
                                    stake_info[FACTORY_LEVEL].is_staked())
                            {
                                slot_staked.clone_from(&slot);
                            }
                        } else if *addr == stake_info_l.address // [STO-031] - access the entity's own storage (if entity staked)
                            || self.associated_with_slot(&stake_info_l.address, &slot, &slots)? // [STO-032] - read/write Access to storage slots that is associated with the entity, in any non-entity contract (if entity staked)
                            || !acc.writes.contains_key(&slot)
                        // [STO-033] - read-only access to any storage in non-entity contract (if
                        // entity staked)
                        {
                            slot_staked.clone_from(&slot);
                        } else {
                            return Err(SimulationError::StorageAccess { slot });
                        }
                    }

                    if !slot_staked.is_empty() && !stake_info_l.is_staked() {
                        return Err(SimulationError::Unstaked {
                            entity: LEVEL_TO_ENTITY[l].into(),
                            address: stake_info_l.address,
                            inner: format!("accessed slot {slot_staked}"),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
