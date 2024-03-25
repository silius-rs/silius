use ethers::types::{Address, U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame};
use silius_primitives::{
    constants::validation::entities::NUMBER_OF_LEVELS,
    get_address,
    reputation::StakeInfo,
    simulation::{StorageMap, StorageMapEntry},
    UserOperation,
};
use std::collections::hash_map::Entry;

/// Helper function to extract the gas limit for verification from the simulation result
///
/// # Arguments
/// `sim_res` - The [simulation result](SimulateValidationResult) from the simulation
///
/// # Returns
/// The gas limit for verification
pub fn extract_verification_gas_limit(sim_res: &SimulateValidationResult) -> U256 {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => res.return_info.0,
        SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.0,
    }
}

/// Helper function to extract the pre-fund for verification from the simulation result
///
/// # Arguments
/// `sim_res` - The [simulation result](SimulateValidationResult) from the simulation
///
/// # Returns
/// The pre-fund for verification
pub fn extract_pre_fund(sim_res: &SimulateValidationResult) -> U256 {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => res.return_info.1,
        SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.1,
    }
}

/// Helper function to extract the post-fund for verification from the simulation result
///
/// # Arguments
/// `sim_res` - The [simulation result](SimulateValidationResult) from the simulation
///
/// # Returns
/// The post-fund for verification
pub fn extract_timestamps(sim_res: &SimulateValidationResult) -> (U256, U256) {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => {
            (res.return_info.3.into(), res.return_info.4.into())
        }
        SimulateValidationResult::ValidationResultWithAggregation(res) => {
            (res.return_info.3.into(), res.return_info.4.into())
        }
    }
}

/// Helper function to extract the stake info from the simulation result
///
/// # Arguments
/// `uo` - The [user operation](UserOperation) to extract the stake info from
/// `sim_res` - The [simulation result](SimulateValidationResult) from the simulation
///
/// # Returns
/// The stake info for the factory, account and paymaster
pub fn extract_stake_info(
    uo: &UserOperation,
    sim_res: &SimulateValidationResult,
) -> [StakeInfo; NUMBER_OF_LEVELS] {
    let (f_info, s_info, p_info) = match sim_res {
        SimulateValidationResult::ValidationResult(res) => {
            (res.factory_info, res.sender_info, res.paymaster_info)
        }
        SimulateValidationResult::ValidationResultWithAggregation(res) => {
            (res.factory_info, res.sender_info, res.paymaster_info)
        }
    };

    [
        // factory
        StakeInfo {
            address: get_address(&uo.init_code).unwrap_or(Address::zero()),
            stake: f_info.0,
            unstake_delay: f_info.1,
        },
        // account
        StakeInfo { address: uo.sender, stake: s_info.0, unstake_delay: s_info.1 },
        // paymaster
        StakeInfo {
            address: get_address(&uo.paymaster_and_data).unwrap_or(Address::zero()),
            stake: p_info.0,
            unstake_delay: p_info.1,
        },
    ]
}

/// Helper function to extract the storage map from the simulation result
///
/// # Arguments
/// `js_trace` - The [js tracer frame](JsTracerFrame) to extract the storage map from
///
/// # Returns
/// The [storage map](StorageMap)
pub fn extract_storage_map(js_trace: &JsTracerFrame) -> StorageMap {
    let mut storage_map = StorageMap::default();

    for l in js_trace.calls_from_entry_point.iter() {
        for (addr, acc) in l.access.iter() {
            storage_map.slots.insert(*addr, StorageMapEntry::Slots(acc.reads.clone()));
        }
    }

    storage_map
}

/// Helper function to merge multiple storage maps into one.
///
/// # Arguments
/// `storage_maps` - The vector of storage maps to merge
///
/// # Returns
/// The [storage map](StorageMap)
pub fn merge_storage_maps(storage_maps: Vec<StorageMap>) -> StorageMap {
    let mut merged_map = StorageMap::default();

    for map in storage_maps {
        for (addr, entry) in map.0 {
            let ent = merged_map.0.entry(addr);

            match ent {
                Entry::Vacant(_) => {
                    ent.or_insert(entry);
                }
                Entry::Occupied(mut val) => match entry {
                    StorageMapEntry::RootHash(hash) => {
                        val.insert(StorageMapEntry::RootHash(hash));
                    }
                    StorageMapEntry::Slots(slots) => match val.get_mut() {
                        StorageMapEntry::RootHash(_) => {}
                        StorageMapEntry::Slots(ref mut sl) => {
                            for (slot, value) in slots {
                                sl.insert(slot, value.clone());
                            }
                        }
                    },
                },
            }
        }
    }

    merged_map
}
