use ethers::types::{Address, U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame};
use silius_primitives::{
    constants::validation::entities::NUMBER_OF_LEVELS, reputation::StakeInfo,
    simulation::StorageMap, UserOperation,
};

#[derive(Debug)]
pub struct AccountValidationData {
    pub sig_authorizer: Address,
    valid_until: U256,
    valid_after: U256,
}

pub fn unpack_account_validation_data(data: U256) -> AccountValidationData {
    let mut b: [u8; 32] = [0; 32];
    data.to_big_endian(&mut b);
    let sig_authorizer = Address::from_slice(&b[..20]);
    let valid_until = U256::from_big_endian(&b[20..26]);
    let valid_after = U256::from_big_endian(&b[26..32]);
    AccountValidationData { sig_authorizer, valid_until, valid_after }
}

/// Helper function to extract the gas limit for verification from the simulation result
///
/// # Arguments
/// `sim_res` - The [simulation result](SimulateValidationResult) from the simulation
///
/// # Returns
/// The gas limit for verification
pub fn extract_verification_gas_limit(sim_res: &SimulateValidationResult) -> U256 {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => res.return_info.pre_op_gas,
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
        SimulateValidationResult::ValidationResult(res) => res.return_info.prefund,
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
            let validation_data =
                unpack_account_validation_data(res.return_info.account_validation_data);
            (validation_data.valid_until, validation_data.valid_after)
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
            (res.factory_info.clone(), res.sender_info.clone(), res.paymaster_info.clone())
        }
    };

    [
        // factory
        StakeInfo {
            address: uo.factory,
            stake: f_info.stake,
            unstake_delay: f_info.unstake_delay_sec,
        },
        // account
        StakeInfo {
            address: uo.sender,
            stake: s_info.stake,
            unstake_delay: s_info.unstake_delay_sec,
        },
        // paymaster
        StakeInfo {
            address: uo.paymaster,
            stake: p_info.stake,
            unstake_delay: p_info.unstake_delay_sec,
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
            storage_map.slots.insert(*addr, acc.reads.clone());
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
        for (addr, entry) in map.root_hashes {
            merged_map.root_hashes.insert(addr, entry);
            merged_map.slots.remove(&addr);
        }

        for (addr, entry) in map.slots {
            if !merged_map.root_hashes.contains_key(&addr) {
                match merged_map.slots.get_mut(&addr) {
                    Some(slots) => {
                        for (slot, value) in entry {
                            slots.insert(slot, value);
                        }
                    }
                    None => {
                        merged_map.slots.insert(addr, entry);
                    }
                }
            }
        }
    }

    merged_map
}
