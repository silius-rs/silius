//! User operation request (optional fields)

use super::UserOperationSigned;
use crate::utils::{as_checksum_addr, as_checksum_bytes};
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// User operation with all fields being optional
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationRequest {
    #[serde(default = "Address::zero", serialize_with = "as_checksum_addr")]
    pub sender: Address,
    #[serde(default)]
    pub nonce: U256,
    #[serde(default, serialize_with = "as_checksum_bytes")]
    pub init_code: Bytes,
    #[serde(default)]
    pub call_data: Bytes,
    #[serde(default)]
    pub call_gas_limit: Option<U256>,
    #[serde(default)]
    pub verification_gas_limit: Option<U256>,
    #[serde(default)]
    pub pre_verification_gas: Option<U256>,
    #[serde(default)]
    pub max_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(default)]
    pub paymaster_and_data: Bytes,
    #[serde(default)]
    pub signature: Option<Bytes>,
}

impl From<UserOperationRequest> for UserOperationSigned {
    fn from(user_operation: UserOperationRequest) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            init_code: user_operation.init_code,
            call_data: user_operation.call_data,
            call_gas_limit: {
                if let Some(call_gas_limit) = user_operation.call_gas_limit {
                    call_gas_limit
                } else {
                    U256::zero()
                }
            },
            verification_gas_limit: {
                if let Some(verification_gas_limit) = user_operation.verification_gas_limit {
                    verification_gas_limit
                } else {
                    U256::zero()
                }
            },
            pre_verification_gas: {
                if let Some(pre_verification_gas) = user_operation.pre_verification_gas {
                    pre_verification_gas
                } else {
                    U256::zero()
                }
            },
            max_fee_per_gas: {
                if let Some(max_fee_per_gas) = user_operation.max_fee_per_gas {
                    max_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            max_priority_fee_per_gas: {
                if let Some(max_priority_fee_per_gas) = user_operation.max_priority_fee_per_gas {
                    max_priority_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            paymaster_and_data: user_operation.paymaster_and_data,
            signature: { user_operation.signature.unwrap_or_default() },
        }
    }
}

impl From<UserOperationSigned> for UserOperationRequest {
    fn from(user_operation: UserOperationSigned) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            init_code: user_operation.init_code,
            call_data: user_operation.call_data,
            call_gas_limit: Some(user_operation.call_gas_limit),
            verification_gas_limit: Some(user_operation.verification_gas_limit),
            pre_verification_gas: Some(user_operation.pre_verification_gas),
            max_fee_per_gas: Some(user_operation.max_fee_per_gas),
            max_priority_fee_per_gas: Some(user_operation.max_priority_fee_per_gas),
            paymaster_and_data: user_operation.paymaster_and_data,
            signature: Some(user_operation.signature),
        }
    }
}
