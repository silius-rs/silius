use aa_bundler_primitives::UserOperation;
use ethers::{abi::AbiDecode, types::Bytes};

use crate::gen::entry_point_api::{self, EntryPointAPICalls};

impl From<UserOperation> for entry_point_api::UserOperation {
    fn from(user_operation: UserOperation) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            init_code: user_operation.init_code,
            call_data: user_operation.call_data,
            call_gas_limit: user_operation.call_gas_limit,
            verification_gas_limit: user_operation.verification_gas_limit,
            pre_verification_gas: user_operation.pre_verification_gas,
            max_fee_per_gas: user_operation.max_fee_per_gas,
            max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas,
            paymaster_and_data: user_operation.paymaster_and_data,
            signature: user_operation.signature,
        }
    }
}

impl From<entry_point_api::UserOperation> for UserOperation {
    fn from(value: entry_point_api::UserOperation) -> Self {
        Self {
            sender: value.sender,
            nonce: value.nonce,
            init_code: value.init_code,
            call_data: value.call_data,
            call_gas_limit: value.call_gas_limit,
            verification_gas_limit: value.verification_gas_limit,
            pre_verification_gas: value.pre_verification_gas,
            max_fee_per_gas: value.max_fee_per_gas,
            max_priority_fee_per_gas: value.max_priority_fee_per_gas,
            paymaster_and_data: value.paymaster_and_data,
            signature: value.signature,
        }
    }
}

pub fn parse_from_input_data(data: Bytes) -> Option<Vec<UserOperation>> {
    EntryPointAPICalls::decode(data)
        .ok()
        .and_then(|call| match call {
            EntryPointAPICalls::HandleOps(ops) => {
                Some(ops.ops.into_iter().map(|op| op.into()).collect())
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use ethers::types::Bytes;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parse_input_data() {
        let data = Bytes::from_str("0x1fad948c0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000690b9a9e9aa1c9db991c7721a92d351db4fac990000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000001ec271771e84999634e5e0330970feeb1c75f35200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000000000000000000000018000000000000000000000000000000000000000000000000000000000000493e000000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000493e00000000000000000000000000000000000000000000000000000000077359400000000000000000000000000000000000000000000000000000000003b9aca0000000000000000000000000000000000000000000000000000000000000001e0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024a9e966b7000000000000000000000000000000000000000000000000000000000010f4470000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002face000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
        let res = parse_from_input_data(data);
        assert!(matches!(res, Some(..)), "No user operation found")
    }
}
