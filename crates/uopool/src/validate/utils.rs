use ethers::types::{Address, U256};
use silius_contracts::{entry_point::SimulateValidationResult, tracer::JsTracerFrame};
use silius_primitives::{
    consts::entities::NUMBER_LEVELS, get_address, reputation::StakeInfo, simulation::StorageMap,
    UserOperation,
};

pub fn extract_verification_gas_limit(sim_res: &SimulateValidationResult) -> U256 {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => res.return_info.0,
        SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.0,
    }
}

pub fn extract_pre_fund(sim_res: &SimulateValidationResult) -> U256 {
    match sim_res {
        SimulateValidationResult::ValidationResult(res) => res.return_info.1,
        SimulateValidationResult::ValidationResultWithAggregation(res) => res.return_info.1,
    }
}

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

pub fn extract_stake_info(
    uo: &UserOperation,
    sim_res: &SimulateValidationResult,
) -> [StakeInfo; NUMBER_LEVELS] {
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
        StakeInfo {
            address: uo.sender,
            stake: s_info.0,
            unstake_delay: s_info.1,
        },
        // paymaster
        StakeInfo {
            address: get_address(&uo.paymaster_and_data).unwrap_or(Address::zero()),
            stake: p_info.0,
            unstake_delay: p_info.1,
        },
    ]
}

pub fn extract_storage_map(js_trace: &JsTracerFrame) -> StorageMap {
    let mut storage_map = StorageMap::default();

    for l in js_trace.number_levels.iter() {
        for (addr, acc) in l.access.iter() {
            storage_map.insert(*addr, acc.reads.clone());
        }
    }

    storage_map
}
