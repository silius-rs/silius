use alloy_primitives::{Address, B256, Bytes, U256};

pub fn pack_address_and_data(address: Option<Address>, data: Option<Bytes>) -> Bytes {
    if let (Some(address), Some(data)) = (address, data) {
        let mut result = Vec::with_capacity(20 + data.len());
        result.extend_from_slice(address.as_slice());
        result.extend_from_slice(&data);
        Bytes::from(result)
    } else {
        Bytes::new()
    }
}

pub fn pack_account_gas_limits(verification_gas_limit: U256, call_gas_limit: U256) -> B256 {
    let mut result = [0u8; 32];
    result[0..16].copy_from_slice(&verification_gas_limit.to_le_bytes());
    result[16..32].copy_from_slice(&call_gas_limit.to_le_bytes());
    B256::from(result)
}
