use std::ops::{Deref, DerefMut};

use alloy_eips::eip7702::SignedAuthorization;
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

use crate::utils::{pack_address_and_data, pack_two_gas_values};

sol! {
    struct PackedUserOperation {
        address sender;
        uint256 nonce;
        bytes init_code;
        bytes call_data;
        bytes32 account_gas_limit;
        uint256 pre_verification_gas;
        bytes32 gas_fees;
        bytes paymaster_and_data;
        bytes signature;
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationBase {
    pub sender: Address,
    pub nonce: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factory: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factory_data: Option<Bytes>,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster_verification_gas_limit: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster_post_op_gas_limit: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster_data: Option<Bytes>,
    pub signature: Bytes,
    // EIP-7702 signed authorization tuple
    #[serde(rename = "eip7702Auth", skip_serializing_if = "Option::is_none")]
    pub signed_authorization: Option<SignedAuthorization>,
}

impl UserOperationBase {
    pub fn builder() -> UserOperationBaseBuilder {
        UserOperationBaseBuilder::default()
    }

    pub fn to_packed_user_operation(&self) -> PackedUserOperation {
        PackedUserOperation {
            sender: self.sender,
            nonce: self.nonce,
            init_code: pack_address_and_data(self.factory, self.factory_data.clone()),
            call_data: self.call_data.clone(),
            account_gas_limit: pack_two_gas_values(
                self.call_gas_limit,
                self.verification_gas_limit,
            ),
            pre_verification_gas: self.pre_verification_gas,
            gas_fees: pack_two_gas_values(self.max_fee_per_gas, self.max_priority_fee_per_gas),
            paymaster_and_data: pack_address_and_data(self.paymaster, self.paymaster_data.clone()),
            signature: self.signature.clone(),
        }
    }
}

#[derive(Default)]
pub struct UserOperationBaseBuilder {
    pub sender: Option<Address>,
    pub nonce: Option<U256>,
    pub factory: Option<Address>,
    pub factory_data: Option<Bytes>,
    pub call_data: Option<Bytes>,
    pub call_gas_limit: Option<U256>,
    pub verification_gas_limit: Option<U256>,
    pub pre_verification_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub paymaster: Option<Address>,
    pub paymaster_verification_gas_limit: Option<U256>,
    pub paymaster_post_op_gas_limit: Option<U256>,
    pub paymaster_data: Option<Bytes>,
    pub signature: Option<Bytes>,
    pub signed_authorization: Option<SignedAuthorization>,
}

impl UserOperationBaseBuilder {
    pub fn sender(mut self, sender: Address) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn nonce(mut self, nonce: U256) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn factory(mut self, factory: Address) -> Self {
        self.factory = Some(factory);
        self
    }

    pub fn factory_data(mut self, factory_data: Bytes) -> Self {
        self.factory_data = Some(factory_data);
        self
    }

    pub fn call_data(mut self, call_data: Bytes) -> Self {
        self.call_data = Some(call_data);
        self
    }

    pub fn call_gas_limit(mut self, call_gas_limit: U256) -> Self {
        self.call_gas_limit = Some(call_gas_limit);
        self
    }

    pub fn verification_gas_limit(mut self, verification_gas_limit: U256) -> Self {
        self.verification_gas_limit = Some(verification_gas_limit);
        self
    }

    pub fn pre_verification_gas(mut self, pre_verification_gas: U256) -> Self {
        self.pre_verification_gas = Some(pre_verification_gas);
        self
    }

    pub fn max_fee_per_gas(mut self, max_fee_per_gas: U256) -> Self {
        self.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: U256) -> Self {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    pub fn paymaster(mut self, paymaster: Address) -> Self {
        self.paymaster = Some(paymaster);
        self
    }

    pub fn paymaster_verification_gas_limit(
        mut self,
        paymaster_verification_gas_limit: U256,
    ) -> Self {
        self.paymaster_verification_gas_limit = Some(paymaster_verification_gas_limit);
        self
    }

    pub fn paymaster_post_op_gas_limit(mut self, paymaster_post_op_gas_limit: U256) -> Self {
        self.paymaster_post_op_gas_limit = Some(paymaster_post_op_gas_limit);
        self
    }

    pub fn paymaster_data(mut self, paymaster_data: Bytes) -> Self {
        self.paymaster_data = Some(paymaster_data);
        self
    }

    pub fn signature(mut self, signature: Bytes) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn signed_authorization(mut self, signed_authorization: SignedAuthorization) -> Self {
        self.signed_authorization = Some(signed_authorization);
        self
    }

    pub fn build(self) -> UserOperationBase {
        UserOperationBase {
            sender: self.sender.expect("sender is required"),
            nonce: self.nonce.expect("nonce is required"),
            factory: self.factory,
            factory_data: self.factory_data,
            call_data: self.call_data.expect("call_data is required"),
            call_gas_limit: self.call_gas_limit.expect("call_gas_limit is required"),
            verification_gas_limit: self
                .verification_gas_limit
                .expect("verification_gas_limit is required"),
            pre_verification_gas: self
                .pre_verification_gas
                .expect("pre_verification_gas is required"),
            max_fee_per_gas: self.max_fee_per_gas.expect("max_fee_per_gas is required"),
            max_priority_fee_per_gas: self
                .max_priority_fee_per_gas
                .expect("max_priority_fee_per_gas is required"),
            paymaster: self.paymaster,
            paymaster_verification_gas_limit: self.paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit: self.paymaster_post_op_gas_limit,
            paymaster_data: self.paymaster_data,
            signature: self.signature.expect("signature is required"),
            signed_authorization: self.signed_authorization,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserOperation {
    pub inner: UserOperationBase,

    // additional data
    pub hash: B256,
}

impl UserOperation {
    pub fn new(user_operation_base: UserOperationBase, hash: B256) -> Self {
        Self {
            inner: user_operation_base,
            hash,
        }
    }
}

impl Deref for UserOperation {
    type Target = UserOperationBase;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for UserOperation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, B256, Bytes, U256, hex::FromHex};

    use crate::user_operation::UserOperationBase;

    #[test]
    fn test_user_operation_base_deserialize() {
        let user_operation = r#"
        {
            "sender": "0x452D8Fa4640d78100215c35c85A20fAE171B6B01",
            "nonce": "0x0",
            "callData": "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447",
            "callGasLimit": "0x493e0",
            "verificationGasLimit": "0xf4240",
            "preVerificationGas": "0x61a80",
            "maxFeePerGas": "0xee6b2800",
            "maxPriorityFeePerGas": "0xb2d05e00",
            "signature": "0xface"
        }"#;
        let user_operation: UserOperationBase = serde_json::from_str(user_operation).unwrap();
        assert_eq!(
            user_operation.sender,
            "0x452D8Fa4640d78100215c35c85A20fAE171B6B01"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(user_operation.nonce, U256::ZERO);
        assert_eq!(
            user_operation.call_data,
            "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447"
                .parse::<Bytes>()
                .unwrap()
        );
        assert_eq!(user_operation.call_gas_limit, U256::from(300_000));
    }

    #[test]
    fn test_user_operation_base_packed() {
        let user_operation = r#"
        {
            "sender": "0x3C9f140494B8aa32f53279326AbD5B132b63a32b",
            "nonce": "0x0",
            "callData": "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447",
            "callGasLimit": "0x493e0",
            "verificationGasLimit": "0xf4240",
            "preVerificationGas": "0x61a80",
            "maxFeePerGas": "0xee6b2800",
            "maxPriorityFeePerGas": "0xb2d05e00",
            "signature": "0xface"
        }"#;
        let user_operation: UserOperationBase = serde_json::from_str(user_operation).unwrap();
        let user_operation_packed = user_operation.to_packed_user_operation();
        assert_eq!(
            user_operation_packed.sender,
            "0x3C9f140494B8aa32f53279326AbD5B132b63a32b"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(user_operation_packed.nonce, U256::ZERO);
        assert_eq!(
            user_operation_packed.init_code,
            Bytes::from_hex("0x").unwrap()
        );
        assert_eq!(
            user_operation_packed.call_data,
            "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447"
                .parse::<Bytes>()
                .unwrap()
        );
        assert_eq!(
            user_operation_packed.account_gas_limit,
            "0x000000000000000000000000000f4240000000000000000000000000000493e0"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            user_operation_packed.pre_verification_gas,
            U256::from(400_000)
        );
        assert_eq!(
            user_operation_packed.gas_fees,
            "0x000000000000000000000000b2d05e00000000000000000000000000ee6b2800"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            user_operation_packed.paymaster_and_data,
            Bytes::from_hex("0x").unwrap()
        );
        assert_eq!(
            user_operation_packed.signature,
            "0xface".parse::<Bytes>().unwrap()
        );
    }
}
