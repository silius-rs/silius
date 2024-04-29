//! Misc utils

use ethers::{
    abi::AbiEncode,
    types::{Address, Bytes, U128, U256},
    utils::to_checksum,
};

/// Converts address to checksum address
pub fn as_checksum_addr<S>(val: &Address, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&to_checksum(val, None))
}

/// Converts Option address to checksum
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

/// If possible, parses address from the first 20 bytes
pub fn get_address(buf: &[u8]) -> Option<Address> {
    if buf.len() >= 20 {
        Some(Address::from_slice(&buf[0..20]))
    } else {
        None
    }
}

pub fn unpack_paymaster_data(buf: &[u8]) -> (Address, U256, U256, Bytes) {
    if buf.len() >= 52 {
        let (paymaster_verification_gas_limit, paymaster_post_op_gas_limit) =
            unpack_uint128(&buf[20..52]);
        (
            Address::from_slice(&buf[0..20]),
            paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit,
            Bytes::from(buf[52..].to_vec()),
        )
    } else {
        (Address::zero(), U256::zero(), U256::zero(), Bytes::default())
    }
}

pub fn pack_paymaster_data(
    addr: Address,
    paymaster_verification_gas_limit: U256,
    paymaster_post_op_gas_limit: U256,
    paymaster_data: &Bytes,
) -> Vec<u8> {
    if addr.is_zero() {
        vec![]
    } else {
        let gas_data = pack_uint128(paymaster_verification_gas_limit, paymaster_post_op_gas_limit);
        [addr.0.to_vec(), gas_data.encode(), paymaster_data.to_vec()].concat()
    }
}

pub fn pack_factory_data(factory: Address, factory_data: Bytes) -> Vec<u8> {
    if factory.is_zero() {
        vec![]
    } else {
        [factory.0.to_vec(), factory_data.to_vec()].concat()
    }
}

pub fn unpack_factory_data(init_code: &[u8]) -> (Address, Bytes) {
    if init_code.len() > 20 {
        (Address::from_slice(&init_code[0..20]), Bytes::from(init_code[20..].to_vec()))
    } else {
        (Address::default(), Bytes::default())
    }
}

/// Packs two uint128
pub fn pack_uint128(a: U256, b: U256) -> [u8; 32] {
    let mut res = [0u8; 32];
    let a: U128 = {
        let mut tem = [0; 32];
        a.to_big_endian(&mut tem);
        U128::from_big_endian(&tem[16..32])
    };
    let b: U128 = {
        let mut tem = [0; 32];
        b.to_big_endian(&mut tem);
        U128::from_big_endian(&tem[16..32])
    };
    a.to_big_endian(&mut res[0..16]);
    b.to_big_endian(&mut res[16..32]);
    res
}

/// Unpacks two uint128 from bytes
pub fn unpack_uint128(buf: &[u8]) -> (U256, U256) {
    let mut a = [0u8; 16];
    let mut b = [0u8; 16];
    a.copy_from_slice(&buf[0..16]);
    b.copy_from_slice(&buf[16..32]);
    (U256::from_big_endian(&a), U256::from_big_endian(&b))
}

#[cfg(test)]
mod tests {
    use crate::{
        pack_factory_data, unpack_factory_data,
        utils::{pack_uint128, unpack_uint128},
    };
    use ethers::types::{Address, Bytes, U256};

    #[test]
    fn pack_unpack_u128() {
        let a: U256 = 100.into();
        let b: U256 = 200.into();
        let packed = pack_uint128(a, b);
        let (new_a, new_b) = unpack_uint128(&packed);
        assert_eq!(a, new_a, "unpack a worked");
        assert_eq!(b, new_b, "unpack b worked");
    }

    #[test]
    fn pack_factory_data_unpack() {
        let addr: Address = "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5".parse().unwrap();
        let data: Bytes = "0x12345678".parse().unwrap();
        let packed: Bytes = pack_factory_data(addr, data.clone()).into();
        println!("packed {packed:?}");
        let (new_addr, new_data) = unpack_factory_data(&packed);
        assert_eq!(addr, new_addr, "addr work");
        assert_eq!(data, new_data, "data work");
    }
}
