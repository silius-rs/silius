//! Misc utils

use ethers::{
    abi::AbiEncode,
    types::{Address, U256},
    utils::to_checksum,
};

/// Converts address to checksum address
pub fn as_checksum_addr<S>(val: &Address, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&to_checksum(val, None))
}

/// Converts bytes to checksum (first 20 bytes are address)
pub fn as_checksum_addr_opt<S>(val: &Option<Address>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if let Some(addr) = val {
        s.serialize_str(&to_checksum(addr, None))
    } else {
        s.serialize_none()
    }
}

/// Serializes U256 as u64
pub fn as_u64<S>(val: &U256, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&val.as_u64().to_string())
}

/// Serializes u64 as hex string
pub fn as_hex_string<S>(val: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde_hex::SerHex::<serde_hex::StrictPfx>::serialize(val, s)
}

/// Packs address and data
pub fn pack_address_and_data(addr: Address, data: &[u8]) -> &[u8] {
    &[addr.encode(), if data.is_empty() { vec![] } else { data.to_vec() }].concat()
}

/// Unpacks address and data from bytes
pub fn unpack_address_and_data(buf: &[u8]) -> (Option<Address>, Option<&[u8]>) {
    if buf.len() >= 20 {
        (Some(Address::from_slice(&buf[0..20])), Some(&buf[20..]))
    } else {
        (None, None)
    }
}

/// Packs two uint128
pub fn pack_uint128(a: U256, b: U256) -> [u8; 32] {
    let mut res = [0u8; 32];
    a.to_big_endian(&mut res[0..16]);
    b.to_big_endian(&mut res[16..32]);
    res
}

/// Unpacks two uint128 from bytes
pub fn unpack_uint128(buf: &[u8; 32]) -> (U256, U256) {
    let mut a = [0u8; 16];
    let mut b = [0u8; 16];
    a.copy_from_slice(&buf[0..16]);
    b.copy_from_slice(&buf[16..32]);
    (U256::from_big_endian(&a), U256::from_big_endian(&b))
}
