//! User operation request (optional fields)

use super::UserOperationSigned;
use crate::utils::{as_checksum_addr, as_checksum_addr_opt};
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
    #[serde(serialize_with = "as_checksum_addr_opt")]
    pub factory: Option<Address>,
    #[serde(default)]
    pub factory_data: Option<Bytes>,
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
    #[serde(serialize_with = "as_checksum_addr_opt")]
    pub paymaster: Option<Address>,
    #[serde(default)]
    pub paymaster_verification_gas_limit: Option<U256>,
    #[serde(default)]
    pub paymaster_post_op_gas_limit: Option<U256>,
    #[serde(default)]
    pub paymaster_data: Option<Bytes>,
    #[serde(default)]
    pub signature: Option<Bytes>,
}

impl From<UserOperationRequest> for UserOperationSigned {
    fn from(user_operation: UserOperationRequest) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            factory: user_operation.factory,
            factory_data: user_operation.factory_data.unwrap_or_default(),
            call_data: user_operation.call_data,
            call_gas_limit: user_operation.call_gas_limit.unwrap_or_default(),
            verification_gas_limit: user_operation.verification_gas_limit.unwrap_or_default(),
            pre_verification_gas: user_operation.pre_verification_gas.unwrap_or_default(),
            max_fee_per_gas: user_operation.max_fee_per_gas.unwrap_or_default(),
            max_priority_fee_per_gas: user_operation.max_priority_fee_per_gas.unwrap_or_default(),
            paymaster: user_operation.paymaster,
            paymaster_verification_gas_limit: user_operation
                .paymaster_verification_gas_limit
                .unwrap_or_default(),
            paymaster_post_op_gas_limit: user_operation
                .paymaster_post_op_gas_limit
                .unwrap_or_default(),
            paymaster_data: user_operation.paymaster_data.unwrap_or_default(),
            signature: user_operation.signature.unwrap_or_default(),
        }
    }
}

impl From<UserOperationSigned> for UserOperationRequest {
    fn from(user_operation: UserOperationSigned) -> Self {
        Self {
            sender: user_operation.sender,
            nonce: user_operation.nonce,
            factory: user_operation.factory,
            factory_data: Some(user_operation.factory_data),
            call_data: user_operation.call_data,
            call_gas_limit: Some(user_operation.call_gas_limit),
            verification_gas_limit: Some(user_operation.verification_gas_limit),
            pre_verification_gas: Some(user_operation.pre_verification_gas),
            max_fee_per_gas: Some(user_operation.max_fee_per_gas),
            max_priority_fee_per_gas: Some(user_operation.max_priority_fee_per_gas),
            paymaster: user_operation.paymaster,
            paymaster_verification_gas_limit: Some(user_operation.paymaster_verification_gas_limit),
            paymaster_post_op_gas_limit: Some(user_operation.paymaster_post_op_gas_limit),
            paymaster_data: Some(user_operation.paymaster_data),
            signature: Some(user_operation.signature),
        }
    }
}
