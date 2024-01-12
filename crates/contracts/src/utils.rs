use crate::gen::entry_point_api::{self, EntryPointAPICalls};
use ethers::{abi::AbiDecode, types::Bytes};
use silius_primitives::UserOperationSigned;

impl From<UserOperationSigned> for entry_point_api::UserOperation {
    fn from(uo: UserOperationSigned) -> Self {
        Self {
            sender: uo.sender,
            nonce: uo.nonce,
            init_code: uo.init_code,
            call_data: uo.call_data,
            call_gas_limit: uo.call_gas_limit,
            verification_gas_limit: uo.verification_gas_limit,
            pre_verification_gas: uo.pre_verification_gas,
            max_fee_per_gas: uo.max_fee_per_gas,
            max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
            paymaster_and_data: uo.paymaster_and_data,
            signature: uo.signature,
        }
    }
}

impl From<entry_point_api::UserOperation> for UserOperationSigned {
    fn from(uo: entry_point_api::UserOperation) -> Self {
        Self {
            sender: uo.sender,
            nonce: uo.nonce,
            init_code: uo.init_code,
            call_data: uo.call_data,
            call_gas_limit: uo.call_gas_limit,
            verification_gas_limit: uo.verification_gas_limit,
            pre_verification_gas: uo.pre_verification_gas,
            max_fee_per_gas: uo.max_fee_per_gas,
            max_priority_fee_per_gas: uo.max_priority_fee_per_gas,
            paymaster_and_data: uo.paymaster_and_data,
            signature: uo.signature,
        }
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
        let data = Bytes::from_str("0x1fad948c0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000690b9a9e9aa1c9db991c7721a92d351db4fac990000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000001ec271771e84999634e5e0330970feeb1c75f35200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000493e000000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000493e00000000000000000000000000000000000000000000000000000000077359400000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001e0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024a9e966b7000000000000000000000000000000000000000000000000000000000010f4470000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002face000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
        let res = parse_from_input_data(data);
        assert!(matches!(res, Some(..)), "No user operation found")
    }
}
