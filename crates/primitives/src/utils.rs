use std::{fs, sync::Arc};

use alloy_primitives::{Address, B256, Bytes, U256};

use crate::network_spec::{DEV, MAINNET, NetworkSpec};

pub fn pack_address_and_data(address: Address, data: Bytes) -> Bytes {
    let mut result = Vec::with_capacity(20 + data.len());
    result.extend_from_slice(address.as_slice());
    result.extend_from_slice(&data);
    Bytes::from(result)
}

pub fn pack_address_two_gas_and_data(
    address: Address,
    gas_1: U256,
    gas_2: U256,
    data: Bytes,
) -> Bytes {
    let mut result = Vec::with_capacity(20 + 32 + 32 + data.len());
    result.extend_from_slice(address.as_slice());
    result.extend_from_slice(pack_two_gas_values(gas_1, gas_2).as_slice());
    result.extend_from_slice(&data);
    Bytes::from(result)
}

pub fn pack_two_gas_values(gas_1: U256, gas_2: U256) -> B256 {
    let mut result = [0u8; 32];
    result[16..32].copy_from_slice(&gas_1.to_be_bytes_vec()[16..32]);
    result[0..16].copy_from_slice(&gas_2.to_be_bytes_vec()[16..32]);
    B256::from(result)
}

pub fn network_parser(network_string: &str) -> Result<Arc<NetworkSpec>, String> {
    match network_string {
        "mainnet" => Ok(MAINNET.clone()),
        "dev" => Ok(DEV.clone()),
        _ => {
            let contents = fs::read_to_string(network_string)
                .map_err(|err| format!("Failed to read file: {err}"))?;
            Ok(Arc::new(serde_yaml::from_str(&contents).map_err(
                |err| format!("Failed to parse YAML from: {err}"),
            )?))
        }
    }
}
