use ethers::types::{Address, U256};
use std::str::FromStr;

pub fn parse_address(s: &str) -> Result<Address, String> {
    Address::from_str(s).map_err(|_| format!("Adress {} is not a valid address", s))
}

pub fn parse_u256(s: &str) -> Result<U256, String> {
    U256::from_str_radix(s, 10).map_err(|_| format!("{} is not a valid U256", s))
}
