use ethers::{
    types::{Address, Bytes, U256},
    utils::{hex, to_checksum},
};

/// Converts address to checksum address
pub fn as_checksum_addr<S>(val: &Address, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&to_checksum(val, None))
}

/// Converts bytes to checksum (first 20 bytes are address)
pub fn as_checksum_bytes<S>(val: &Bytes, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut str = hex::encode_prefixed(val);
    s.serialize_str(if val.len() >= 20 {
        let addr = Address::from_slice(&val[0..20]);
        str.replace_range(0..42, &to_checksum(&addr, None));
        &str
    } else {
        &str
    })
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

/// If possible, parses address from the first 20 bytes
pub fn get_address(buf: &[u8]) -> Option<Address> {
    if buf.len() >= 20 {
        Some(Address::from_slice(&buf[0..20]))
    } else {
        None
    }
}
