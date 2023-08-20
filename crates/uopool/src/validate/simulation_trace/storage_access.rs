use crate::validate::{utils::extract_stake_info, SimulationTraceCheck, SimulationTraceHelper};
use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256},
    utils::keccak256,
};
use silius_primitives::{
    consts::entities::{FACTORY_INDEX, LEVEL_TO_ENTITY, NUMBER_LEVELS},
    reputation::StakeInfo,
    simulation::SimulationCheckError,
    UserOperation,
};
use std::collections::{HashMap, HashSet};

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
        info: &[StakeInfo; NUMBER_LEVELS],
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
                    slots
                        .entry(entity.address)
                        .or_insert(HashSet::new())
                        .insert(k.into());
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
    ) -> Result<bool, SimulationCheckError> {
        if *slot == addr.to_string() {
            return Ok(true);
        }

        if !slots.contains_key(addr) {
            return Ok(false);
        }

        let slot_num = U256::from_str_radix(slot, 16)
            .map_err(|_| SimulationCheckError::StorageAccess { slot: slot.clone() })?;

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
    /// The [check_user_operation] method implementation that checks if the user operation access storage other than the one associated with itself.
    ///
    /// # Arguments
    /// `uo` - The [UserOperation](UserOperation) to check
    /// `helper` - The [SimulationTraceHelper](crate::validate::SimulationTraceHelper)
    ///
    /// # Returns
    /// None if the check passes, otherwise a [SimulationCheckError] error.
    async fn check_user_operation(
        &self,
        uo: &UserOperation,
        helper: &mut SimulationTraceHelper<M>,
    ) -> Result<(), SimulationCheckError> {
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

        for (i, stake_info_i) in stake_info.iter().enumerate() {
            if let Some(l) = helper.js_trace.number_levels.get(i) {
                for (addr, acc) in &l.access {
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
                            if !(uo.init_code.is_empty()
                                || uo.sender == stake_info_i.address
                                    && stake_info[FACTORY_INDEX].is_staked())
                            {
                                slot_staked = slot.clone();
                            }
                        } else if *addr == stake_info_i.address
                            || self.associated_with_slot(&stake_info_i.address, &slot, &slots)?
                        {
                            slot_staked = slot.clone();
                        } else {
                            return Err(SimulationCheckError::StorageAccess { slot });
                        }
                    }

                    if !slot_staked.is_empty() && !stake_info_i.is_staked() {
                        return Err(SimulationCheckError::Unstaked {
                            entity: LEVEL_TO_ENTITY[i].to_string(),
                            message: format!("accessed slot {addr} slot {slot_staked}"),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
