use ethers::{
    types::{Address, U256},
    utils::to_checksum,
};
use std::str::FromStr;

pub fn as_checksum<S>(val: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&to_checksum(val, None))
}

// Try to get the address from first 20 bytes. Return None if length of bytes < 20.
pub fn get_addr(bytes: &[u8]) -> Option<Address> {
    if bytes.len() >= 20 {
        Some(Address::from_slice(&bytes[0..20]))
    } else {
        None
    }
}

pub fn parse_address(s: &str) -> Result<Address, String> {
    Address::from_str(s).map_err(|_| format!("Adress {s} is not a valid address"))
}
pub fn parse_u256(s: &str) -> Result<U256, String> {
    U256::from_str_radix(s, 10).map_err(|_| format!("{s} is not a valid U256"))
}
