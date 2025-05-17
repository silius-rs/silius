use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_sol_types::{SolValue, sol};

use crate::{network_spec::network_spec, utils::{pack_account_gas_limits, pack_address_and_data}};

sol! {
    struct PackedUserOperation {
        address sender;
        uint256 nonce;
        bytes init_code;
        bytes call_data;
        bytes32 accounts_gas_limit;
        uint256 pre_verification_gas;
        bytes32 gas_fees;
        bytes paymaster_and_data;
        bytes signature;
    }
}

pub struct UserOperationBase {
    pub sender: Address,
    pub nonce: U256,
    pub factory: Option<Address>,
    pub factory_data: Option<Bytes>,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster: Option<Address>,
    pub paymaster_verification_gas_limit: Option<U256>,
    pub paymaster_post_op_gas_limit: Option<U256>,
    pub paymaster_data: Option<Bytes>,
}

impl UserOperationBase {
    pub fn to_packed_user_operation(&self) -> PackedUserOperation {
        PackedUserOperation {
            sender: self.sender,
            nonce: self.nonce,
            init_code: pack_address_and_data(self.factory, self.factory_data.clone()),
            call_data: self.call_data.clone(),
            accounts_gas_limit: pack_account_gas_limits(self.verification_gas_limit, self.call_gas_limit),
        }
    }
}

pub struct UserOperation {
    pub inner: UserOperationBase,

    // additional data
    pub signature: Bytes,
    pub hash: B256,
}

impl UserOperation {
    pub fn builder() -> UserOperationBuilder {
        UserOperationBuilder::default()
    }
}

#[derive(Default)]
pub struct UserOperationBuilder {
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
}

impl UserOperationBuilder {
    pub fn with_sender(mut self, sender: Address) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_nonce(mut self, nonce: U256) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn with_factory(mut self, factory: Address) -> Self {
        self.factory = Some(factory);
        self
    }

    pub fn with_factory_data(mut self, factory_data: Bytes) -> Self {
        self.factory_data = Some(factory_data);
        self
    }

    pub fn with_call_data(mut self, call_data: Bytes) -> Self {
        self.call_data = Some(call_data);
        self
    }

    pub fn with_call_gas_limit(mut self, call_gas_limit: U256) -> Self {
        self.call_gas_limit = Some(call_gas_limit);
        self
    }

    pub fn with_verification_gas_limit(mut self, verification_gas_limit: U256) -> Self {
        self.verification_gas_limit = Some(verification_gas_limit);
        self
    }

    pub fn with_pre_verification_gas(mut self, pre_verification_gas: U256) -> Self {
        self.pre_verification_gas = Some(pre_verification_gas);
        self
    }

    pub fn with_max_fee_per_gas(mut self, max_fee_per_gas: U256) -> Self {
        self.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    pub fn with_max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: U256) -> Self {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    pub fn with_paymaster(mut self, paymaster: Address) -> Self {
        self.paymaster = Some(paymaster);
        self
    }

    pub fn with_paymaster_verification_gas_limit(
        mut self,
        paymaster_verification_gas_limit: U256,
    ) -> Self {
        self.paymaster_verification_gas_limit = Some(paymaster_verification_gas_limit);
        self
    }

    pub fn with_paymaster_post_op_gas_limit(mut self, paymaster_post_op_gas_limit: U256) -> Self {
        self.paymaster_post_op_gas_limit = Some(paymaster_post_op_gas_limit);
        self
    }

    pub fn with_paymaster_data(mut self, paymaster_data: Bytes) -> Self {
        self.paymaster_data = Some(paymaster_data);
        self
    }

    pub fn with_signature(mut self, signature: Bytes) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn build(self) -> UserOperation {
        let user_operation_base = UserOperationBase {
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
        };
        let hash = user_operation_base.hash();

        UserOperation {
            inner: user_operation_base,
            signature: self.signature.expect("signature is required"),
            hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::network_spec::{MAINNET, set_network_spec};

    use super::*;

    #[test]
    fn test_user_operation_hash() {
        set_network_spec(MAINNET.clone());
        let user_operation = UserOperationBuilder::default()
            .with_sender(
                "0x9c5754De1443984659E1b3a8d1931D83475ba29C"
                    .parse()
                    .unwrap(),
            )
            .with_nonce(U256::ZERO)
            .with_factory(
                "0x9406cc6185a346906296840746125a0e44976454"
                    .parse()
                    .unwrap(),
            )
            .with_factory_data("5fbfb9cf000000000000000000000000ce0fefa6f7979c4c9b5373e0f5105b7259092c6d0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap())
            .with_call_data("0xb61d27f60000000000000000000000009c5754de1443984659e1b3a8d1931d83475ba29c00000000000000000000000000000000000000000000000000005af3107a400000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000".parse().unwrap())
            .with_call_gas_limit(U256::from(33_100))
            .with_verification_gas_limit(U256::from(361_460))
            .with_pre_verification_gas(U256::from(44_980))
            .with_max_fee_per_gas(U256::from(1_695_000_030))
            .with_max_priority_fee_per_gas(U256::from(1_695_000_000))
            .with_signature("0xebfd4657afe1f1c05c1ec65f3f9cc992a3ac083c424454ba61eab93152195e1400d74df01fc9fa53caadcb83a891d478b713016bcc0c64307c1ad3d7ea2e2d921b".parse().unwrap())
            .build();
        assert_eq!(
            user_operation.hash,
            "0x95418c07086df02ff6bc9e8bdc150b380cb761beecc098630440bcec6e862702"
                .parse::<B256>()
                .unwrap()
        );
    }
}
