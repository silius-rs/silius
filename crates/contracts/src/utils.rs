use crate::gen::entry_point_api::{self, EntryPointAPICalls};
use ethers::{
    abi::{AbiDecode, AbiEncode},
    types::{Address, Bytes},
};
use silius_primitives::{
    pack_init_code, pack_paymaster_fee_data, pack_uint128, unpack_init_code,
    unpack_paymaster_fee_data, unpack_uint128, UserOperationSigned,
};

impl From<UserOperationSigned> for entry_point_api::PackedUserOperation {
    fn from(uo: UserOperationSigned) -> Self {
        let paymaster_and_data = pack_paymaster_fee_data(
            uo.paymaster,
            uo.paymaster_verification_gas_limit,
            uo.paymaster_post_op_gas_limit,
            &uo.paymaster_data,
        )
        .into();
        let init_code = pack_init_code(uo.factory, uo.factory_data).into();
        Self {
            sender: uo.sender,
            nonce: uo.nonce,
            init_code,
            call_data: uo.call_data,
            account_gas_limits: pack_uint128(uo.verification_gas_limit, uo.call_gas_limit),
            pre_verification_gas: uo.pre_verification_gas,
            gas_fees: pack_uint128(uo.max_priority_fee_per_gas, uo.max_fee_per_gas),
            paymaster_and_data,
            signature: uo.signature,
        }
    }
}

impl From<entry_point_api::PackedUserOperation> for UserOperationSigned {
    fn from(uo: entry_point_api::PackedUserOperation) -> Self {
        let (verification_gas_limit, call_gas_limit) = unpack_uint128(&uo.account_gas_limits);
        let (max_priority_fee_per_gas, max_fee_per_gas) = unpack_uint128(&uo.gas_fees);
        let (
            paymaster,
            paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit,
            paymaster_data,
        ) = unpack_paymaster_fee_data(&uo.paymaster_and_data);
        let (factory, factory_data) = unpack_init_code(&uo.init_code);
        Self {
            sender: uo.sender,
            nonce: uo.nonce,
            factory,
            factory_data,
            call_data: uo.call_data,
            call_gas_limit,
            verification_gas_limit,
            pre_verification_gas: uo.pre_verification_gas,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            paymaster,
            paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit,
            paymaster_data,
            signature: uo.signature,
        }
    }
}

/// Packs address and data
pub fn pack_address_and_data(addr: Address, data: &[u8]) -> Vec<u8> {
    [addr.encode(), data.to_vec()].concat()
}

/// Unpacks address and data from bytes
pub fn unpack_address_and_data(buf: &[u8]) -> (Option<Address>, Bytes) {
    if buf.len() >= 20 {
        (Some(Address::from_slice(&buf[0..20])), Bytes::from(buf[20..].to_vec()))
    } else {
        (None, Bytes::default())
    }
}

pub fn parse_from_input_data(data: Bytes) -> Option<Vec<UserOperationSigned>> {
    EntryPointAPICalls::decode(data).ok().and_then(|call| match call {
        EntryPointAPICalls::HandleOps(ops) => {
            Some(ops.ops.into_iter().map(|op| op.into()).collect())
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::Bytes;
    use std::str::FromStr;

    #[test]
    fn parse_input_data() {
        let data = Bytes::from_str("0x765e827f00000000000000000000000000000000000000000000000000000000000000400000000000000000000000004337003fcd2f56de3977ccb806383e9161628d0e000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000001cb509e54b625e7279f982307d113c141cb6e28400008104e3ad430ea6d354d013a6789fdfc71e671c4300000000000000000008000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000001cdfd0000000000000000000000000001a09a000000000000000000000000000000000000000000000000000000000000e70400000000000000000000000000af4c85000000000000000000000004106e87e600000000000000000000000000000000000000000000000000000000000003200000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001a4e9ae5c5301000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000083f20f44975d03b1b09e64809b757c47f942beea000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb00000000000000000000000084c7c9bbc2de2485058f22d0f470461f677543de00000000000000000000000000000000000000000000000143154eb7298a5e38000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b59d0021a869f1ed3a661ffe8c9b41ec6244261d9800000000000000000000000000004e1f0000000000000000000000000000000100000000000000000000000000000000000000000000000000000000662033a30000000000000000000000000000000000000000000000000000000000000000e50a72c9531714f177b164d89d2f86d7b2d5e55c0a4c890581b81a6f4fa7f96b41228033a6f2b934a196f1791f84fa226996810922c4d83ee075b4ced01dcf3a1b00000000000000000000000000000000000000000000000000000000000000000000000000000000000041f5ecf66065c6907048f8c3fd8870908a484762e01472f80fb1247a29f8dc69b273e39affa41494cc6572b66691674e8c3ac4cf4cdc96000214923fe925c8df481c00000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
        let res = parse_from_input_data(data);
        assert!(matches!(res, Some(..)), "No user operation found")
    }
}
