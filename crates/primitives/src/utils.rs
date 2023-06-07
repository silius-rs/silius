use ethers::{types::Address, utils::to_checksum};

/// Converts address to checksum address
pub fn as_checksum<S>(val: &Address, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&to_checksum(val, None))
}

/// If possible, parses address from the first 20 bytes
pub fn get_address(buf: &[u8]) -> Option<Address> {
    if buf.len() >= 20 {
        Some(Address::from_slice(&buf[0..20]))
    } else {
        None
    }
}
